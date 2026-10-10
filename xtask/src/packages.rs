//! `cargo xtask packages`: the bundled Typst packages (specs/003-open-source-project,
//! contracts/xtask-cli.md).
//!
//! Every change is computed and verified in memory first (lock entries + archives); files are
//! written only when the new state is consistent, so a failure leaves the repository untouched.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use sha2::{Digest, Sha256};
use typst_syntax::{SyntaxNode, ast};

use crate::lock::{self, Entry, Lock, Role, Version};
use crate::universe::{self, Source};

/// Package sub-folders that are not reachable from its entrypoint.
const SKIPPED_DIRS: [&str; 5] = ["tests/", "docs/", "examples/", "gallery/", "template/"];

/// The bundled set: lock entries and their archives (by archive file name).
#[derive(Debug, Clone, Default)]
pub struct State {
    pub entries: Vec<Entry>,
    pub archives: BTreeMap<String, Vec<u8>>,
}

impl State {
    pub fn load(root: &Path) -> Result<Self> {
        let lock = Lock::read(root)?;
        let mut archives = BTreeMap::new();
        for item in fs::read_dir(Lock::vendor(root))? {
            let path = item?.path();
            if path.extension().is_some_and(|e| e == "gz") {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                archives.insert(name, fs::read(&path)?);
            }
        }
        Ok(Self {
            entries: lock.package,
            archives,
        })
    }

    fn find(&self, name: &str, version: &str) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.name == name && e.version == version)
    }

    fn selected(&self, name: &str) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.name == name && e.role == Role::Selected)
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Literal `@preview/<name>:<version>` imports of an archive's reachable `.typ` files.
pub fn imports(archive: &[u8]) -> Result<BTreeSet<(String, String)>> {
    fn walk(node: &SyntaxNode, found: &mut BTreeSet<(String, String)>) {
        let source = node
            .cast::<ast::ModuleImport>()
            .map(|i| i.source())
            .or_else(|| node.cast::<ast::ModuleInclude>().map(|i| i.source()));
        if let Some(ast::Expr::Str(string)) = source
            && let Some(spec) = string.get().strip_prefix("@preview/")
            && let Some((name, version)) = spec.split_once(':')
            && Version::parse(version).is_some()
        {
            found.insert((name.to_owned(), version.to_owned()));
        }
        for child in node.children() {
            walk(child, found);
        }
    }
    let mut found = BTreeSet::new();
    for (path, bytes) in lock::archive_files(archive)? {
        if !path.ends_with(".typ") || SKIPPED_DIRS.iter().any(|dir| path.starts_with(dir)) {
            continue;
        }
        if let Ok(text) = std::str::from_utf8(&bytes) {
            walk(&typst_syntax::parse(text), &mut found);
        }
    }
    Ok(found)
}

/// Every inconsistency of `state` (empty when valid).
pub fn problems(state: &State) -> Vec<String> {
    let mut problems = Vec::new();
    let listed: BTreeSet<String> = state.entries.iter().map(Entry::archive_name).collect();
    let mut selected = BTreeMap::<&str, Vec<&str>>::new();
    let mut available = BTreeSet::new();

    for entry in &state.entries {
        let label = format!("{} {}", entry.name, entry.version);
        match state.archives.get(&entry.archive_name()) {
            None => problems.push(format!(
                "{label}: archive packages/vendor/{} missing",
                entry.archive_name()
            )),
            Some(bytes) if sha256(bytes) != entry.sha256 => problems.push(format!(
                "{label}: sha256 mismatch (expected {}, got {})",
                entry.sha256,
                sha256(bytes)
            )),
            Some(_) => {}
        }
        if entry.role == Role::Selected {
            selected
                .entry(&entry.name)
                .or_default()
                .push(&entry.version);
        }
        available.insert((entry.name.clone(), entry.version.clone()));
    }
    for archive in state.archives.keys() {
        if !listed.contains(archive) {
            problems.push(format!(
                "packages/vendor/{archive} is not listed in lock.toml"
            ));
        }
    }
    for (name, versions) in selected {
        if versions.len() > 1 {
            problems.push(format!(
                "{name}: several selected versions ({})",
                versions.join(", ")
            ));
        }
    }
    for entry in &state.entries {
        let Some(bytes) = state.archives.get(&entry.archive_name()) else {
            continue;
        };
        match imports(bytes) {
            Ok(deps) => {
                for (name, version) in deps {
                    if !available.contains(&(name.clone(), version.clone())) {
                        problems.push(format!(
                            "{} {}: imports @preview/{name}:{version}, which is not bundled",
                            entry.name, entry.version
                        ));
                    }
                }
            }
            Err(e) => problems.push(format!("{} {}: {e:#}", entry.name, entry.version)),
        }
    }
    problems
}

/// Adds `name@version` (and, recursively, its missing dependencies) to `state`.
fn add_with_dependencies(
    state: &mut State,
    source: &dyn Source,
    index: &[universe::IndexEntry],
    engine: Version,
    name: &str,
    version: Version,
    role: Role,
) -> Result<Vec<String>> {
    let mut added = Vec::new();
    let mut queue = VecDeque::from([(name.to_owned(), version.to_string(), role)]);
    while let Some((name, version, role)) = queue.pop_front() {
        if let Some(i) = state.find(&name, &version) {
            if role == Role::Selected {
                state.entries[i].role = Role::Selected;
            }
            continue;
        }
        universe::check_available(index, &name, Version::parse(&version).unwrap(), engine)?;
        let archive = source.archive(&name, &version)?;
        let files = lock::archive_files(&archive)?;
        let manifest = lock::manifest(&files)?;
        let field = |key: &str| {
            manifest
                .get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned()
        };
        if field("name") != name || field("version") != version {
            bail!(
                "{name} {version}: the archive's typst.toml declares {} {}",
                field("name"),
                field("version")
            );
        }
        for (dep_name, dep_version) in imports(&archive)? {
            queue.push_back((dep_name, dep_version, Role::Dependency));
        }
        let license = match field("license") {
            l if l.is_empty() => "UNKNOWN".to_owned(),
            l => l,
        };
        let entry = Entry {
            namespace: "preview".into(),
            name: name.clone(),
            version: version.clone(),
            sha256: sha256(&archive),
            license,
            role,
        };
        state.archives.insert(entry.archive_name(), archive);
        added.push(format!(
            "{name} {version} ({})",
            if role == Role::Selected {
                "selected"
            } else {
                "dependency"
            }
        ));
        state.entries.push(entry);
    }
    Ok(added)
}

/// Removes dependency entries that no selected package needs anymore (directly or not).
fn prune(state: &mut State) -> Result<Vec<String>> {
    let mut needed = BTreeSet::new();
    let mut queue: VecDeque<(String, String)> = state
        .entries
        .iter()
        .filter(|e| e.role == Role::Selected)
        .map(|e| (e.name.clone(), e.version.clone()))
        .collect();
    while let Some(key) = queue.pop_front() {
        if !needed.insert(key.clone()) {
            continue;
        }
        if let Some(bytes) = state.archives.get(&lock::archive_name(&key.0, &key.1)) {
            queue.extend(imports(bytes)?);
        }
    }
    let mut removed = Vec::new();
    state.entries.retain(|e| {
        let keep = needed.contains(&(e.name.clone(), e.version.clone()));
        if !keep {
            removed.push(format!("{} {}", e.name, e.version));
        }
        keep
    });
    let listed: BTreeSet<String> = state.entries.iter().map(Entry::archive_name).collect();
    state.archives.retain(|archive, _| listed.contains(archive));
    Ok(removed)
}

/// Resolves the version to install: the given one, or the latest compatible one.
fn resolve(
    index: &[universe::IndexEntry],
    name: &str,
    version: Option<&str>,
    engine: Version,
) -> Result<Version> {
    match version {
        Some(v) => {
            Version::parse(v).ok_or_else(|| anyhow!("invalid version `{v}` (expected X.Y.Z)"))
        }
        None => universe::latest_compatible(index, name, engine).ok_or_else(|| {
            anyhow!("no version of @preview/{name} is compatible with Typst {engine}")
        }),
    }
}

pub enum Change<'a> {
    Add {
        name: &'a str,
        version: Option<&'a str>,
    },
    Update {
        name: &'a str,
        version: Option<&'a str>,
    },
    Remove {
        name: &'a str,
    },
}

/// Computes the new state for `change`, verified, plus a human-readable summary.
pub fn plan(
    state: &State,
    change: Change,
    source: &dyn Source,
    engine: Version,
) -> Result<(State, Vec<String>)> {
    let mut next = state.clone();
    let mut summary = Vec::new();
    match change {
        Change::Add { name, version } => {
            if next.selected(name).is_some() {
                bail!("{name} is already bundled; use `cargo xtask packages update {name}`");
            }
            let index = source.index()?;
            let version = resolve(&index, name, version, engine)?;
            summary.extend(
                add_with_dependencies(
                    &mut next,
                    source,
                    &index,
                    engine,
                    name,
                    version,
                    Role::Selected,
                )?
                .into_iter()
                .map(|a| format!("add {a}")),
            );
        }
        Change::Update { name, version } => {
            let current = next
                .selected(name)
                .ok_or_else(|| {
                    anyhow!("{name} is not bundled; use `cargo xtask packages add {name}`")
                })?
                .clone();
            let index = source.index()?;
            let version = resolve(&index, name, version, engine)?;
            if current.version == version.to_string() {
                summary.push(format!("{name} is already at {version}"));
                return Ok((next, summary));
            }
            // The previous version stays only if another package imports it.
            if let Some(i) = next.find(name, &current.version) {
                next.entries[i].role = Role::Dependency;
            }
            summary.extend(
                add_with_dependencies(
                    &mut next,
                    source,
                    &index,
                    engine,
                    name,
                    version,
                    Role::Selected,
                )?
                .into_iter()
                .map(|a| format!("add {a}")),
            );
        }
        Change::Remove { name } => {
            let current = next
                .selected(name)
                .ok_or_else(|| anyhow!("{name} is not bundled"))?
                .clone();
            if let Some(i) = next.find(name, &current.version) {
                next.entries[i].role = Role::Dependency;
            }
            summary.push(format!(
                "warning: templates importing @preview/{name} will become invalid"
            ));
        }
    }
    summary.extend(prune(&mut next)?.into_iter().map(|r| format!("remove {r}")));
    let problems = problems(&next);
    if !problems.is_empty() {
        bail!(
            "the resulting package set is inconsistent:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok((next, summary))
}

/// Writes `next` to the repository: new archives, lock file, removed archives, generated docs.
pub fn write(root: &Path, current: &State, next: &State) -> Result<()> {
    let vendor = Lock::vendor(root);
    for (name, bytes) in &next.archives {
        if !current.archives.contains_key(name) {
            fs::write(vendor.join(name), bytes).with_context(|| format!("writing {name}"))?;
        }
    }
    let lock = Lock {
        package: next.entries.clone(),
    };
    fs::write(Lock::path(root), lock.to_toml())?;
    for name in current.archives.keys() {
        if !next.archives.contains_key(name) {
            fs::remove_file(vendor.join(name))?;
        }
    }
    crate::docs::generate(root)
}

/// `cargo xtask packages list`.
pub fn list(state: &State) -> Result<String> {
    let mut entries = state.entries.clone();
    entries.sort_by_key(Entry::key);
    let mut out = format!(
        "{:<20} {:<9} {:<11} {:<20} {}\n",
        "NAME", "VERSION", "ROLE", "LICENSE", "WASM"
    );
    for e in entries {
        let files = lock::archive_files(
            state
                .archives
                .get(&e.archive_name())
                .map(Vec::as_slice)
                .unwrap_or_default(),
        )?;
        let wasm = files.keys().any(|p| p.ends_with(".wasm"));
        out.push_str(&format!(
            "{:<20} {:<9} {:<11} {:<20} {}\n",
            e.name,
            e.version,
            if e.role == Role::Selected {
                "selected"
            } else {
                "dependency"
            },
            e.license,
            if wasm { "yes" } else { "" }
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// In-memory `.tar.gz` with the given files.
    fn tarball(files: &[(&str, &str)]) -> Vec<u8> {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        for (path, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, path, content.as_bytes())
                .unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn package(name: &str, version: &str, lib: &str) -> Vec<u8> {
        let manifest = format!(
            "[package]\nname = \"{name}\"\nversion = \"{version}\"\nentrypoint = \"lib.typ\"\nlicense = \"MIT\"\n"
        );
        tarball(&[
            ("typst.toml", &manifest),
            ("lib.typ", lib),
            ("tests/t.typ", "#import \"@preview/ghost:9.9.9\""),
        ])
    }

    struct Fake {
        index: Vec<universe::IndexEntry>,
        archives: BTreeMap<String, Vec<u8>>,
        fail_on: RefCell<Option<String>>,
    }

    impl Source for Fake {
        fn index(&self) -> Result<Vec<universe::IndexEntry>> {
            Ok(self.index.clone())
        }
        fn archive(&self, name: &str, version: &str) -> Result<Vec<u8>> {
            if self.fail_on.borrow().as_deref() == Some(name) {
                bail!("network error for {name}");
            }
            self.archives
                .get(&lock::archive_name(name, version))
                .cloned()
                .ok_or_else(|| anyhow!("404 {name} {version}"))
        }
    }

    fn fake() -> Fake {
        let mut archives = BTreeMap::new();
        let mut index = Vec::new();
        let mut publish = |name: &str, version: &str, compiler: Option<&str>, lib: &str| {
            archives.insert(
                lock::archive_name(name, version),
                package(name, version, lib),
            );
            index.push(universe::IndexEntry {
                name: name.into(),
                version: version.into(),
                compiler: compiler.map(Into::into),
            });
        };
        publish(
            "chart",
            "1.0.0",
            Some("0.14"),
            "#import \"@preview/draw:0.5.0\": canvas\n",
        );
        publish("chart", "2.0.0", Some("0.16.0"), "");
        publish("chart", "1.1.0", None, "#import \"@preview/draw:0.6.0\"\n");
        publish("draw", "0.5.0", None, "#import \"@preview/fmt:1.0.0\"\n");
        publish("draw", "0.6.0", None, "");
        publish("fmt", "1.0.0", None, "");
        Fake {
            index,
            archives,
            fail_on: RefCell::new(None),
        }
    }

    const ENGINE: Version = Version(0, 15, 1);

    fn names(state: &State) -> Vec<String> {
        let mut out: Vec<String> = state
            .entries
            .iter()
            .map(|e| {
                format!(
                    "{}@{}:{}",
                    e.name,
                    e.version,
                    if e.role == Role::Selected { "s" } else { "d" }
                )
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn imports_ignore_tests_and_docs_folders() {
        let found = imports(&package(
            "x",
            "1.0.0",
            "#import \"@preview/a:1.2.3\": f\n#include \"@preview/b:0.1.0\"",
        ))
        .unwrap();
        assert_eq!(
            found.into_iter().collect::<Vec<_>>(),
            [("a".into(), "1.2.3".into()), ("b".into(), "0.1.0".into())]
        );
    }

    #[test]
    fn add_picks_the_latest_compatible_version_and_its_dependencies() {
        let source = fake();
        let (state, summary) = plan(
            &State::default(),
            Change::Add {
                name: "chart",
                version: Some("1.0.0"),
            },
            &source,
            ENGINE,
        )
        .unwrap();
        assert_eq!(
            names(&state),
            ["chart@1.0.0:s", "draw@0.5.0:d", "fmt@1.0.0:d"]
        );
        assert!(summary.iter().any(|s| s.contains("fmt 1.0.0 (dependency)")));

        // Without a version: 2.0.0 needs Typst 0.16, so 1.1.0 is picked.
        let (state, _) = plan(
            &State::default(),
            Change::Add {
                name: "chart",
                version: None,
            },
            &source,
            ENGINE,
        )
        .unwrap();
        assert_eq!(names(&state), ["chart@1.1.0:s", "draw@0.6.0:d"]);
    }

    #[test]
    fn update_replaces_the_version_and_prunes_orphans() {
        let source = fake();
        let (state, _) = plan(
            &State::default(),
            Change::Add {
                name: "chart",
                version: Some("1.0.0"),
            },
            &source,
            ENGINE,
        )
        .unwrap();
        let (state, summary) = plan(
            &state,
            Change::Update {
                name: "chart",
                version: None,
            },
            &source,
            ENGINE,
        )
        .unwrap();
        assert_eq!(names(&state), ["chart@1.1.0:s", "draw@0.6.0:d"]);
        assert!(summary.iter().any(|s| s == "remove fmt 1.0.0"));
        assert_eq!(state.archives.len(), 2);
    }

    #[test]
    fn remove_drops_the_package_and_its_orphans() {
        let source = fake();
        let (state, _) = plan(
            &State::default(),
            Change::Add {
                name: "chart",
                version: Some("1.0.0"),
            },
            &source,
            ENGINE,
        )
        .unwrap();
        let (state, _) = plan(&state, Change::Remove { name: "chart" }, &source, ENGINE).unwrap();
        assert!(state.entries.is_empty() && state.archives.is_empty());
    }

    #[test]
    fn failures_leave_the_state_unchanged() {
        let source = fake();
        let (state, _) = plan(
            &State::default(),
            Change::Add {
                name: "draw",
                version: Some("0.6.0"),
            },
            &source,
            ENGINE,
        )
        .unwrap();
        *source.fail_on.borrow_mut() = Some("fmt".into());
        let before = names(&state);
        assert!(
            plan(
                &state,
                Change::Add {
                    name: "chart",
                    version: Some("1.0.0")
                },
                &source,
                ENGINE
            )
            .is_err()
        );
        assert_eq!(names(&state), before);
        assert!(
            plan(
                &state,
                Change::Add {
                    name: "chart",
                    version: Some("2.0.0")
                },
                &source,
                ENGINE
            )
            .is_err(),
            "incompatible"
        );
        assert!(
            plan(
                &state,
                Change::Add {
                    name: "draw",
                    version: None
                },
                &source,
                ENGINE
            )
            .is_err(),
            "already bundled"
        );
    }

    #[test]
    fn problems_detect_inconsistencies() {
        let source = fake();
        let (mut state, _) = plan(
            &State::default(),
            Change::Add {
                name: "chart",
                version: Some("1.0.0"),
            },
            &source,
            ENGINE,
        )
        .unwrap();
        assert!(problems(&state).is_empty());

        state
            .archives
            .insert("chart-1.0.0.tar.gz".into(), b"tampered".to_vec());
        state
            .archives
            .insert("extra-1.0.0.tar.gz".into(), Vec::new());
        state.archives.remove("fmt-1.0.0.tar.gz");
        let mut duplicate = state.entries[0].clone();
        duplicate.version = "9.9.9".into();
        state.entries.push(duplicate);
        let found = problems(&state).join("\n");
        assert!(found.contains("chart 1.0.0: sha256 mismatch"), "{found}");
        assert!(
            found.contains("extra-1.0.0.tar.gz is not listed"),
            "{found}"
        );
        assert!(found.contains("fmt 1.0.0: archive"), "{found}");
        assert!(found.contains("several selected versions"), "{found}");
    }

    #[test]
    fn the_repository_set_is_consistent() {
        let state = State::load(&crate::repo_root()).unwrap();
        assert_eq!(problems(&state), Vec::<String>::new());
    }
}
