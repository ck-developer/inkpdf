//! Intègre au binaire les paquets Typst listés dans `packages/lock.toml`.
//!
//! Pour chaque entrée : vérifie l'empreinte de l'archive de `packages/vendor/`, la décompresse
//! dans `OUT_DIR`, contrôle son manifeste, puis génère `bundled_packages.rs` (une table
//! statique dont chaque fichier est inclus par `include_bytes!`). Toute incohérence fait
//! échouer la compilation. Contrat : specs/002-typst-packages/contracts/lock-file.md.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct Lock {
    #[serde(default)]
    package: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    namespace: String,
    name: String,
    version: String,
    sha256: String,
    license: String,
    role: Role,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Role {
    Selected,
    Dependency,
}

struct Package {
    entry: Entry,
    description: String,
    entrypoint: String,
    /// Chemin relatif dans le paquet → chemin absolu du fichier extrait.
    files: BTreeMap<String, PathBuf>,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=packages");

    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let vendor = root.join("packages/vendor");

    let lock_text = fs::read_to_string(root.join("packages/lock.toml"))
        .unwrap_or_else(|e| fail(format!("packages/lock.toml: {e}")));
    let lock: Lock =
        toml::from_str(&lock_text).unwrap_or_else(|e| fail(format!("packages/lock.toml: {e}")));

    check_lock(&lock, &vendor);

    let mut packages: Vec<Package> = lock
        .package
        .into_iter()
        .map(|entry| extract(entry, &vendor, &out.join("packages")))
        .collect();
    packages.sort_by(|a, b| {
        (a.entry.name.as_str(), parse_version(&a.entry.version))
            .cmp(&(b.entry.name.as_str(), parse_version(&b.entry.version)))
    });

    fs::write(out.join("bundled_packages.rs"), generate(&packages))
        .unwrap_or_else(|e| fail(format!("bundled_packages.rs: {e}")));
}

fn fail(message: String) -> ! {
    panic!("bundled Typst packages: {message}");
}

fn parse_version(version: &str) -> (u32, u32, u32) {
    let parts: Vec<u32> = version.split('.').filter_map(|p| p.parse().ok()).collect();
    match parts[..] {
        [a, b, c] if version.split('.').count() == 3 => (a, b, c),
        _ => fail(format!("invalid version `{version}`")),
    }
}

/// Cohérence entre le lock et le dossier `vendor/`, avant toute extraction.
fn check_lock(lock: &Lock, vendor: &Path) {
    let mut seen = BTreeSet::new();
    let mut selected: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for entry in &lock.package {
        if entry.namespace != "preview" {
            fail(format!(
                "{} {}: only the `preview` namespace is supported",
                entry.name, entry.version
            ));
        }
        parse_version(&entry.version);
        if !seen.insert((entry.name.as_str(), entry.version.as_str())) {
            fail(format!("{} {}: duplicate entry", entry.name, entry.version));
        }
        if entry.role == Role::Selected {
            selected
                .entry(&entry.name)
                .or_default()
                .push(&entry.version);
        }
    }
    for (name, versions) in &selected {
        if versions.len() > 1 {
            fail(format!(
                "{name}: several selected versions ({})",
                versions.join(", ")
            ));
        }
    }
    let listed: BTreeSet<String> = lock
        .package
        .iter()
        .map(|e| format!("{}-{}.tar.gz", e.name, e.version))
        .collect();
    for item in fs::read_dir(vendor).unwrap_or_else(|e| fail(format!("packages/vendor: {e}"))) {
        let file = item.unwrap().file_name().to_string_lossy().into_owned();
        if file.ends_with(".tar.gz") && !listed.contains(&file) {
            fail(format!("packages/vendor/{file} not listed in lock.toml"));
        }
    }
}

fn extract(entry: Entry, vendor: &Path, out: &Path) -> Package {
    let label = format!("{} {}", entry.name, entry.version);
    let archive_path = vendor.join(format!("{}-{}.tar.gz", entry.name, entry.version));
    let archive = fs::read(&archive_path).unwrap_or_else(|_| {
        fail(format!(
            "packages/vendor/{}-{}.tar.gz missing",
            entry.name, entry.version
        ))
    });

    let digest: String = Sha256::digest(&archive)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if digest != entry.sha256 {
        fail(format!(
            "{label}: sha256 mismatch (expected {}, got {digest})",
            entry.sha256
        ));
    }

    let dir = out.join(format!("{}-{}", entry.name, entry.version));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();

    let mut files = BTreeMap::new();
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(&archive[..]));
    let entries = tar
        .entries()
        .unwrap_or_else(|_| fail(format!("{label}: unreadable archive")));
    for item in entries {
        let mut item = item.unwrap_or_else(|_| fail(format!("{label}: unreadable archive")));
        if !item.header().entry_type().is_file() {
            continue;
        }
        let raw = item.path().unwrap().into_owned();
        let relative = normalize(&raw)
            .unwrap_or_else(|| fail(format!("{label}: unsafe path {}", raw.display())));
        let mut data = Vec::new();
        item.read_to_end(&mut data)
            .unwrap_or_else(|_| fail(format!("{label}: unreadable archive")));
        let target = dir.join(&relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, data).unwrap();
        files.insert(relative, target);
    }

    let manifest_path = files
        .get("typst.toml")
        .unwrap_or_else(|| fail(format!("{label}: manifest mismatch (typst.toml missing)")));
    let manifest: toml::Table = fs::read_to_string(manifest_path)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or_else(|| {
            fail(format!(
                "{label}: manifest mismatch (unreadable typst.toml)"
            ))
        });
    let package = manifest
        .get("package")
        .and_then(|p| p.as_table())
        .unwrap_or_else(|| fail(format!("{label}: manifest mismatch ([package] missing)")));
    let field = |key: &str| package.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    if field("name").as_deref() != Some(&entry.name)
        || field("version").as_deref() != Some(&entry.version)
    {
        fail(format!("{label}: manifest mismatch"));
    }
    let entrypoint = field("entrypoint")
        .and_then(|e| normalize(Path::new(&e)))
        .unwrap_or_else(|| fail(format!("{label}: manifest mismatch (entrypoint)")));
    if !files.contains_key(&entrypoint) {
        fail(format!("{label}: entrypoint {entrypoint} not found"));
    }

    Package {
        description: field("description").unwrap_or_default(),
        entrypoint,
        files,
        entry,
    }
}

/// Chemin relatif sûr, séparé par `/` ; `None` s'il est absolu ou remonte avec `..`.
fn normalize(path: &Path) -> Option<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_owned()),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn generate(packages: &[Package]) -> String {
    let mut code = String::from(
        "// Généré par build.rs à partir de packages/lock.toml. Ne pas modifier.\n\
         pub(crate) static BUNDLED: &[BundledPackageDef] = &[\n",
    );
    for package in packages {
        let entry = &package.entry;
        writeln!(code, "    BundledPackageDef {{").unwrap();
        writeln!(code, "        name: {:?},", entry.name).unwrap();
        writeln!(code, "        version: {:?},", entry.version).unwrap();
        writeln!(code, "        description: {:?},", package.description).unwrap();
        writeln!(code, "        license: {:?},", entry.license).unwrap();
        writeln!(code, "        selected: {},", entry.role == Role::Selected).unwrap();
        writeln!(code, "        entrypoint: {:?},", package.entrypoint).unwrap();
        writeln!(code, "        files: &[").unwrap();
        for (relative, absolute) in &package.files {
            let absolute = absolute.to_str().expect("UTF-8 OUT_DIR");
            writeln!(
                code,
                "            ({relative:?}, include_bytes!({absolute:?})),"
            )
            .unwrap();
        }
        writeln!(code, "        ],\n    }},").unwrap();
    }
    code.push_str("];\n");
    code
}
