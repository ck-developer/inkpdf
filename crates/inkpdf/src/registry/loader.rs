//! Loading a template directory into a registry entry.
//!
//! This is the only place where a template's files are read: rendering then only uses the
//! in-memory snapshot.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use time::OffsetDateTime;
use typst::foundations::Bytes;

use super::fingerprint::{self, Fingerprint};
use super::version::Version;
use super::{Layout, TemplateVersions};
use crate::render::fonts::{self, FontSet};
use crate::render::metadata;
use crate::template::{Manifest, TemplateId, TemplateSchema, imports};

/// Name of the optional example request body of a template (research R3 of feature 005).
const EXAMPLE_FILE: &str = "example.json";

/// Reason given to a template with files both at its root and in version folders (research R9
/// of feature 005).
pub const MIXED_LAYOUT: &str =
    "put the template files either at the root or in version folders, not both";

/// Status of a template after loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateStatus {
    Valid,
    Invalid { reason: String },
}

/// Registry entry: a loaded template, valid or not.
pub struct TemplateEntry {
    pub id: TemplateId,
    pub name: String,
    pub description: Option<String>,
    /// The version shown to callers: the version folder's name, or in the flat layout the
    /// `version` of `template.json`, as written.
    pub version: Option<String>,
    /// `version` parsed as `MAJOR.MINOR.PATCH`, used for ordering and selection; `None` when
    /// there is no version, or when a flat template's manifest version does not parse.
    pub semver: Option<Version>,
    pub status: TemplateStatus,
    /// Present if the template is valid.
    pub schema: Option<TemplateSchema>,
    /// The template's `example.json` as written, present only if the template is valid and
    /// the example passed the load-time check.
    pub example: Option<serde_json::Value>,
    /// Snapshot of all files; key = `/`-separated relative path.
    pub files: HashMap<String, Bytes>,
    /// Embedded fonts + template fonts (present if valid).
    pub fonts: Option<Arc<FontSet>>,
    pub fingerprint: Fingerprint,
    pub loaded_at: OffsetDateTime,
}

impl std::fmt::Debug for TemplateEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateEntry")
            .field("id", &self.id)
            .field("version", &self.version)
            .field("status", &self.status)
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}

impl TemplateEntry {
    pub fn is_valid(&self) -> bool {
        self.status == TemplateStatus::Valid
    }

    pub fn invalid_reason(&self) -> Option<&str> {
        match &self.status {
            TemplateStatus::Valid => None,
            TemplateStatus::Invalid { reason } => Some(reason),
        }
    }
}

/// Result of a load.
#[derive(Debug)]
pub enum LoadOutcome {
    Loaded(Box<TemplateEntry>),
    /// The directory changed during the read: try again later.
    Unstable,
}

/// Loads every version of the template folder `dir` (research R9 of feature 005): the flat
/// layout gives one entry, the versioned layout one per `MAJOR.MINOR.PATCH` sub-folder, each
/// loaded, fingerprinted and checked on its own, with its own size limit.
///
/// The versions of `previous` whose folder is unchanged are kept as they are. `None` when the
/// folder changed during the load: try again later.
pub fn load_template(
    dir: &Path,
    id: TemplateId,
    max_template_bytes: u64,
    previous: Option<&TemplateVersions>,
) -> Option<TemplateVersions> {
    let broken = |fingerprint, reason| {
        Some(TemplateVersions {
            fingerprint,
            layout: Layout::Broken(Arc::new(invalid(id.clone(), None, fingerprint, reason))),
        })
    };
    let fingerprint = match Fingerprint::of(dir) {
        Ok(fingerprint) => fingerprint,
        Err(e) => {
            return broken(
                Fingerprint::default(),
                format!("template directory is unreadable: {e}"),
            );
        }
    };
    let folders = match version_folders(dir) {
        Ok(folders) => folders,
        Err(e) => {
            return broken(
                fingerprint,
                format!("template directory is unreadable: {e}"),
            );
        }
    };

    let layout = if folders.versions.is_empty() {
        match load(dir, id.clone(), max_template_bytes) {
            LoadOutcome::Loaded(entry) => Layout::Single(Arc::from(entry)),
            LoadOutcome::Unstable => return None,
        }
    } else if folders.root_files {
        return broken(fingerprint, MIXED_LAYOUT.to_owned());
    } else {
        for folder in &folders.ignored {
            tracing::warn!(
                event = "template.version_ignored",
                templateId = %id,
                folder = %folder,
                "sub-folder of a versioned template is not a MAJOR.MINOR.PATCH version"
            );
        }
        let mut versions = BTreeMap::new();
        for (version, path) in folders.versions {
            let kept = previous.and_then(|p| p.get(version)).filter(|entry| {
                Fingerprint::of(&path).is_ok_and(|current| current == entry.fingerprint)
            });
            let entry = match kept {
                Some(entry) => entry.clone(),
                None => match load_dir(&path, id.clone(), max_template_bytes, Some(version)) {
                    LoadOutcome::Loaded(entry) => Arc::from(entry),
                    LoadOutcome::Unstable => return None,
                },
            };
            versions.insert(version, entry);
        }
        Layout::Versioned(versions)
    };

    // A version folder added or removed during the load.
    match Fingerprint::of(dir) {
        Ok(after) if after == fingerprint => Some(TemplateVersions {
            fingerprint,
            layout,
        }),
        _ => None,
    }
}

/// The direct children of a template folder, hidden ones ignored.
#[derive(Default)]
struct VersionFolders {
    /// Sub-folders named `MAJOR.MINOR.PATCH`.
    versions: Vec<(Version, PathBuf)>,
    /// Other sub-folders, by name.
    ignored: Vec<String>,
    /// Whether there is any file at the root.
    root_files: bool,
}

fn version_folders(dir: &Path) -> std::io::Result<VersionFolders> {
    let root = fs::canonicalize(dir)?;
    let mut folders = VersionFolders::default();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        // Links are followed as in the fingerprint's walk: a target outside the template
        // folder is not part of it.
        let Ok(target) = fs::canonicalize(&path) else {
            continue;
        };
        if !target.starts_with(&root) {
            continue;
        }
        if !target.is_dir() {
            folders.root_files = true;
        } else if let Ok(version) = name.parse::<Version>() {
            folders.versions.push((version, path));
        } else {
            folders.ignored.push(name);
        }
    }
    folders.ignored.sort();
    Ok(folders)
}

/// Loads the directory `dir` of template `id` as a single version (the flat layout).
pub fn load(dir: &Path, id: TemplateId, max_template_bytes: u64) -> LoadOutcome {
    load_dir(dir, id, max_template_bytes, None)
}

/// Loads one template root: the template folder itself in the flat layout, or the version
/// folder of `folder`.
fn load_dir(
    dir: &Path,
    id: TemplateId,
    max_template_bytes: u64,
    folder: Option<Version>,
) -> LoadOutcome {
    match load_root(dir, id, max_template_bytes, folder) {
        LoadOutcome::Loaded(mut entry) => {
            match folder {
                Some(version) => {
                    entry.version = Some(version.to_string());
                    entry.semver = Some(version);
                }
                None => entry.semver = entry.version.as_deref().and_then(|v| v.parse().ok()),
            }
            LoadOutcome::Loaded(entry)
        }
        LoadOutcome::Unstable => LoadOutcome::Unstable,
    }
}

fn load_root(
    dir: &Path,
    id: TemplateId,
    max_template_bytes: u64,
    folder: Option<Version>,
) -> LoadOutcome {
    let before = match fingerprint::scan(dir) {
        Ok(scan) => scan,
        Err(e) => {
            return LoadOutcome::Loaded(Box::new(invalid(
                id,
                None,
                Fingerprint::default(),
                format!("template directory is unreadable: {e}"),
            )));
        }
    };
    let total = before.fingerprint.total_bytes;
    if total > max_template_bytes {
        return LoadOutcome::Loaded(Box::new(invalid(
            id,
            None,
            before.fingerprint,
            format!("template size {total} bytes exceeds the limit of {max_template_bytes} bytes"),
        )));
    }
    for link in &before.escaping_links {
        tracing::warn!(
            event = "template.link_excluded",
            templateId = %id,
            file = %link,
            "symbolic link pointing outside the template directory is excluded"
        );
    }

    let mut files = HashMap::with_capacity(before.files.len());
    let mut read_error = None;
    for file in &before.files {
        match fs::read(&file.path) {
            Ok(data) => {
                files.insert(file.relative.clone(), Bytes::new(data));
            }
            Err(e) => {
                read_error = Some(format!("{}: {e}", file.relative));
                break;
            }
        }
    }

    match fingerprint::Fingerprint::of(dir) {
        Ok(after) if after == before.fingerprint => {}
        _ => return LoadOutcome::Unstable,
    }
    if let Some(reason) = read_error {
        return LoadOutcome::Loaded(Box::new(invalid(id, None, before.fingerprint, reason)));
    }

    LoadOutcome::Loaded(Box::new(validate(id, files, before.fingerprint, folder)))
}

/// Applies the data-model validity rules to a snapshot; `folder` is the version of a version
/// folder, which the manifest's `version`, when given, must equal.
fn validate(
    id: TemplateId,
    mut files: HashMap<String, Bytes>,
    fingerprint: Fingerprint,
    folder: Option<Version>,
) -> TemplateEntry {
    let manifest = match files.get("template.json") {
        None => Manifest::default(),
        Some(bytes) => match Manifest::parse(bytes) {
            Ok(manifest) => manifest,
            Err(e) => return invalid(id, None, fingerprint, format!("template.json: {e}")),
        },
    };
    if let (Some(folder), Some(version)) = (folder, manifest.version.as_deref())
        && version != folder.to_string()
    {
        let reason =
            format!("template.json: version \"{version}\" does not match its folder \"{folder}\"");
        return invalid(id, Some(manifest), fingerprint, reason);
    }

    if let Err(reason) = resolve_package_imports(&mut files) {
        return invalid(id, Some(manifest), fingerprint, reason);
    }

    let checked = (|| {
        let main = files.get("main.typ").ok_or("main.typ is missing")?;
        std::str::from_utf8(main).map_err(|_| "main.typ is not valid UTF-8")?;

        let raw = files.get("schema.json").ok_or("schema.json is missing")?;
        let schema = TemplateSchema::load(raw.clone()).map_err(|e| format!("schema.json: {e}"))?;

        let mut extra = Vec::new();
        let mut font_files: Vec<_> = files
            .iter()
            .filter(|(path, _)| fonts::is_font_file(path))
            .collect();
        font_files.sort_by(|a, b| a.0.cmp(b.0));
        for (path, data) in font_files {
            extra.extend(fonts::parse_font_file(path, data.clone())?);
        }
        Ok::<_, String>((schema, FontSet::with_template_fonts(extra)))
    })();

    match checked {
        Ok((schema, fonts)) => TemplateEntry {
            example: example(&id, &schema, &files),
            name: manifest.name.unwrap_or_else(|| id.to_string()),
            description: manifest.description,
            version: manifest.version,
            semver: None,
            id,
            status: TemplateStatus::Valid,
            schema: Some(schema),
            files,
            fonts: Some(fonts),
            fingerprint,
            loaded_at: OffsetDateTime::now_utc(),
        },
        Err(reason) => invalid(id, Some(manifest), fingerprint, reason),
    }
}

/// The template's `example.json`, if it passes [`check_example`]; otherwise it is dropped and
/// logged, and the template stays valid (FR-006 of feature 005).
fn example(
    id: &TemplateId,
    schema: &TemplateSchema,
    files: &HashMap<String, Bytes>,
) -> Option<serde_json::Value> {
    let raw = files.get(EXAMPLE_FILE)?;
    match check_example(schema, raw) {
        Ok(example) => Some(example),
        Err(reason) => {
            // The reason names the faulty paths; the example itself is not logged.
            tracing::warn!(
                event = "template.example_invalid",
                templateId = %id,
                reason = %reason,
            );
            None
        }
    }
}

/// Checks an example like a render body (research R3 of feature 005): `metadata` against the
/// service's schema, then `data` and `layout` against the template's, `layout` defaults
/// applied. Returns the example as written, or the reason it fails: one `path: message` per
/// violation, `; `-separated, or the JSON error.
fn check_example(schema: &TemplateSchema, raw: &[u8]) -> Result<serde_json::Value, String> {
    let example: serde_json::Value =
        serde_json::from_slice(raw).map_err(|e| format!("invalid JSON: {e}"))?;
    // Checked on a copy: `metadata` extraction and `layout` defaults change it.
    let mut body = example
        .as_object()
        .cloned()
        .ok_or("the example must be a JSON object (a render body)")?;
    let mut violations = metadata::extract(&mut body).err().unwrap_or_default();
    violations.extend(
        schema
            .prepare(serde_json::Value::Object(body))
            .err()
            .unwrap_or_default(),
    );
    if violations.is_empty() {
        return Ok(example);
    }
    Err(violations
        .iter()
        .map(|v| format!("{}: {}", v.path, v.message))
        .collect::<Vec<_>>()
        .join("; "))
}

/// Rewrites the `@preview/<name>` imports of each `.typ` file with the installed version;
/// on incorrect imports, returns one `file:line: message` line per error.
fn resolve_package_imports(files: &mut HashMap<String, Bytes>) -> Result<(), String> {
    let mut paths: Vec<String> = files
        .keys()
        .filter(|path| path.ends_with(".typ"))
        .cloned()
        .collect();
    paths.sort();
    let mut errors = Vec::new();
    for path in paths {
        // A non-UTF-8 `.typ` will be reported by Typst if it is used.
        let Ok(text) = std::str::from_utf8(&files[&path]) else {
            continue;
        };
        match imports::resolve_imports(&path, text) {
            Ok(Some(rewritten)) => {
                files.insert(path, Bytes::new(rewritten.into_bytes()));
            }
            Ok(None) => {}
            Err(found) => errors.extend(found),
        }
    }
    if errors.is_empty() {
        return Ok(());
    }
    Err(errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn invalid(
    id: TemplateId,
    manifest: Option<Manifest>,
    fingerprint: Fingerprint,
    reason: String,
) -> TemplateEntry {
    let manifest = manifest.unwrap_or_default();
    TemplateEntry {
        name: manifest.name.unwrap_or_else(|| id.to_string()),
        description: manifest.description,
        version: manifest.version,
        semver: None,
        id,
        status: TemplateStatus::Invalid { reason },
        schema: None,
        example: None,
        files: HashMap::new(),
        fonts: None,
        fingerprint,
        loaded_at: OffsetDateTime::now_utc(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const SCHEMA: &str = r#"{"type":"object","properties":{"data":{"type":"object"}}}"#;

    struct Fixture {
        _tmp: tempfile::TempDir,
        dir: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let dir = tmp.path().join("tpl");
            fs::create_dir(&dir).unwrap();
            let fixture = Self { _tmp: tmp, dir };
            fixture.write("main.typ", "= Hello");
            fixture.write("schema.json", SCHEMA);
            fixture
        }

        fn write(&self, name: &str, content: impl AsRef<[u8]>) {
            let path = self.dir.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }

        fn load_with(&self, max: u64) -> TemplateEntry {
            match load(&self.dir, "tpl".parse().unwrap(), max) {
                LoadOutcome::Loaded(entry) => *entry,
                LoadOutcome::Unstable => panic!("unexpected unstable load"),
            }
        }

        fn load(&self) -> TemplateEntry {
            self.load_with(u64::MAX)
        }
    }

    fn reason(entry: &TemplateEntry) -> &str {
        entry.invalid_reason().expect("template should be invalid")
    }

    #[test]
    fn package_import_is_rewritten_with_installed_version() {
        let f = Fixture::new();
        f.write("main.typ", "#import \"@preview/zero\": num\n#num(1)\n");
        let entry = f.load();
        assert!(entry.is_valid(), "{:?}", entry.status);
        let main = std::str::from_utf8(&entry.files["main.typ"]).unwrap();
        let version = crate::packages::selected("zero").unwrap().version();
        assert!(
            main.contains(&format!("\"@preview/zero:{version}\"")),
            "{main}"
        );
    }

    #[test]
    fn incorrect_package_imports_make_the_template_invalid() {
        let f = Fixture::new();
        f.write("main.typ", "#import \"parts/a.typ\"\n");
        f.write(
            "parts/a.typ",
            "\n#import \"@preview/zero:0.7.1\"\n#import \"@preview/nope\"\n",
        );
        let entry = f.load();
        assert_eq!(
            reason(&entry),
            "parts/a.typ:2: remove the version: write @preview/zero (inkpdf uses its installed version)\n\
             parts/a.typ:3: package @preview/nope is not available in inkpdf (see GET /packages)"
        );
    }

    #[test]
    fn valid_template_with_manifest() {
        let f = Fixture::new();
        f.write(
            "template.json",
            r#"{"name":"N","description":"D","version":"1.0.0"}"#,
        );
        f.write("parts/footer.typ", "footer");
        let entry = f.load();
        assert!(entry.is_valid(), "{:?}", entry.status);
        assert_eq!(entry.name, "N");
        assert_eq!(entry.description.as_deref(), Some("D"));
        assert_eq!(entry.version.as_deref(), Some("1.0.0"));
        assert!(entry.files.contains_key("parts/footer.typ"));
        assert!(entry.schema.is_some());
    }

    #[test]
    fn name_defaults_to_id_without_manifest() {
        let entry = Fixture::new().load();
        assert!(entry.is_valid());
        assert_eq!(entry.name, "tpl");
    }

    #[test]
    fn missing_main_typ_is_invalid() {
        let f = Fixture::new();
        fs::remove_file(f.dir.join("main.typ")).unwrap();
        assert!(reason(&f.load()).contains("main.typ"));
    }

    #[test]
    fn unreadable_schema_is_invalid() {
        let f = Fixture::new();
        f.write("schema.json", "{");
        assert!(reason(&f.load()).contains("schema.json"));
    }

    #[test]
    fn schema_without_data_is_invalid() {
        let f = Fixture::new();
        f.write(
            "schema.json",
            r#"{"type":"object","properties":{"layout":{}}}"#,
        );
        assert!(reason(&f.load()).contains("properties.data"));
    }

    #[test]
    fn schema_with_other_root_property_is_invalid() {
        let f = Fixture::new();
        f.write(
            "schema.json",
            r#"{"type":"object","properties":{"data":{},"other":{}}}"#,
        );
        assert!(reason(&f.load()).contains("other"));
    }

    #[test]
    fn external_ref_is_invalid() {
        let f = Fixture::new();
        f.write(
            "schema.json",
            r#"{"type":"object","properties":{"data":{"$ref":"https://example.com/x.json"}}}"#,
        );
        assert!(reason(&f.load()).contains("$ref"));
    }

    #[test]
    fn manifest_with_unknown_key_is_invalid() {
        let f = Fixture::new();
        f.write("template.json", r#"{"title":"typo"}"#);
        assert!(reason(&f.load()).contains("template.json"));
    }

    #[test]
    fn unreadable_font_is_invalid() {
        let f = Fixture::new();
        f.write("fonts/broken.ttf", "not a font");
        assert!(reason(&f.load()).contains("fonts/broken.ttf"));
    }

    #[test]
    fn oversized_template_is_invalid_without_reading_files() {
        let f = Fixture::new();
        f.write("assets/big.bin", vec![0u8; 4096]);
        let entry = f.load_with(1024);
        let reason = reason(&entry);
        assert!(reason.contains("1024"), "{reason}");
        assert!(reason.contains(&entry.fingerprint.total_bytes.to_string()));
        assert!(entry.files.is_empty());
    }

    /// A `data.title`, a `data.date` with `format: date` and a `layout` with a default.
    const EXAMPLE_SCHEMA: &str = r##"{
        "type": "object",
        "properties": {
            "data": {
                "type": "object",
                "required": ["title"],
                "additionalProperties": false,
                "properties": {
                    "title": { "type": "string", "minLength": 1 },
                    "date": { "type": "string", "format": "date" }
                }
            },
            "layout": {
                "type": "object",
                "properties": { "color": { "type": "string", "default": "#000000" } }
            }
        }
    }"##;

    fn with_example(example: &str) -> TemplateEntry {
        let f = Fixture::new();
        f.write("schema.json", EXAMPLE_SCHEMA);
        f.write("example.json", example);
        f.load()
    }

    #[test]
    fn valid_example_is_kept_as_written() {
        let example = r#"{
            "data": { "title": "Report", "date": "2024-02-29" },
            "metadata": { "title": "Quarterly report", "author": ["A", "B"] }
        }"#;
        let entry = with_example(example);
        assert!(entry.is_valid(), "{:?}", entry.status);
        // As written: `metadata` kept, no `layout` defaults added.
        assert_eq!(
            entry.example,
            Some(serde_json::from_str::<serde_json::Value>(example).unwrap())
        );
    }

    #[test]
    fn failing_example_is_dropped_and_the_template_stays_valid() {
        for example in [
            // Breaks the template's schema.
            r#"{ "data": { "title": "" , "colour": "red" } }"#,
            r#"{ "data": { "title": "T", "date": "2026-02-30" } }"#,
            r#"{ "data": { "title": "T" }, "layout": { "color": 1 } }"#,
            // Breaks the metadata schema.
            r#"{ "data": { "title": "T" }, "metadata": { "date": "01/10/2026" } }"#,
            r#"{ "data": { "title": "T" }, "metadata": { "colour": "red" } }"#,
            // Not a render body.
            r#"[{ "data": { "title": "T" } }]"#,
            // Malformed JSON.
            r#"{ "data": "#,
        ] {
            let entry = with_example(example);
            assert!(entry.is_valid(), "{example}: {:?}", entry.status);
            assert_eq!(entry.example, None, "{example}");
            assert!(entry.files.contains_key("example.json"), "{example}");
        }
    }

    #[test]
    fn template_without_example_has_none() {
        let f = Fixture::new();
        f.write("schema.json", EXAMPLE_SCHEMA);
        let entry = f.load();
        assert!(entry.is_valid());
        assert_eq!(entry.example, None);
    }

    #[test]
    fn example_of_an_invalid_template_is_not_checked() {
        let f = Fixture::new();
        f.write("example.json", r#"{ "data": {} }"#);
        fs::remove_file(f.dir.join("main.typ")).unwrap();
        let entry = f.load();
        assert!(!entry.is_valid());
        assert_eq!(entry.example, None);
    }

    #[test]
    fn example_check_gives_the_violations_or_the_json_error() {
        let schema = TemplateSchema::load(Bytes::new(EXAMPLE_SCHEMA.as_bytes().to_vec())).unwrap();
        let check = |example: &str| check_example(&schema, example.as_bytes());

        assert_eq!(
            check(r#"{ "data": { "title": "T", "date": "2026-02-30" } }"#),
            Err("/data/date: \"2026-02-30\" is not a valid calendar date (YYYY-MM-DD)".into())
        );
        // Every violation, `metadata` first, as in a render's problem.
        assert_eq!(
            check(r#"{ "data": {}, "metadata": { "date": "x" } }"#),
            Err(
                "/metadata/date: \"x\" does not match \"^[0-9]{4}-[0-9]{2}-[0-9]{2}$\"; \
                 /data: \"title\" is a required property"
                    .into()
            )
        );
        assert_eq!(
            check(r#"{ "data": "#),
            Err("invalid JSON: EOF while parsing a value at line 1 column 10".into())
        );
        assert_eq!(
            check("[]"),
            Err("the example must be a JSON object (a render body)".into())
        );
        let valid = r#"{ "data": { "title": "T" } }"#;
        assert_eq!(check(valid), Ok(serde_json::from_str(valid).unwrap()));
    }

    #[cfg(unix)]
    #[test]
    fn escaping_symlink_is_excluded_from_snapshot() {
        let f = Fixture::new();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), "secret").unwrap();
        fs::create_dir_all(f.dir.join("assets")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("secret"), f.dir.join("assets/x")).unwrap();
        let entry = f.load();
        assert!(entry.is_valid());
        assert!(!entry.files.contains_key("assets/x"));
    }

    #[test]
    fn modification_during_read_is_unstable() {
        // A file that changes between the two walks makes the load unstable: the change is
        // simulated by modifying the directory from a second thread while a large file is being
        // read.
        let f = Fixture::new();
        f.write("assets/big.bin", vec![0u8; 64 * 1024 * 1024]);
        let dir = f.dir.clone();
        let writer = std::thread::spawn(move || {
            for i in 0..200 {
                fs::write(dir.join("main.typ"), format!("= v{i}")).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });
        let mut saw_unstable = false;
        for _ in 0..20 {
            if matches!(
                load(&f.dir, "tpl".parse().unwrap(), u64::MAX),
                LoadOutcome::Unstable
            ) {
                saw_unstable = true;
                break;
            }
        }
        writer.join().unwrap();
        assert!(saw_unstable);
    }

    // --- Versions (research R9 of feature 005) -------------------------------------------

    /// An empty template folder `tpl`, to lay out flat or with version folders.
    fn template_folder() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("tpl");
        fs::create_dir(&dir).unwrap();
        Fixture { _tmp: tmp, dir }
    }

    impl Fixture {
        /// A valid template in `folder` (relative to the template folder; `""` for the root),
        /// with `template.json` holding `version` when given.
        fn template(&self, folder: &str, version: Option<&str>) -> &Self {
            let prefix = if folder.is_empty() {
                String::new()
            } else {
                format!("{folder}/")
            };
            self.write(&format!("{prefix}main.typ"), "= Hello");
            self.write(&format!("{prefix}schema.json"), SCHEMA);
            if let Some(version) = version {
                self.write(
                    &format!("{prefix}template.json"),
                    format!(r#"{{"version":"{version}"}}"#),
                );
            }
            self
        }

        fn load_template(&self) -> TemplateVersions {
            load_template(&self.dir, "tpl".parse().unwrap(), u64::MAX, None)
                .expect("unexpected unstable load")
        }
    }

    /// The versions of a template, newest first, as `(version, valid)`.
    fn versions(template: &TemplateVersions) -> Vec<(String, bool)> {
        template
            .versions()
            .iter()
            .map(|e| (e.version.clone().unwrap(), e.is_valid()))
            .collect()
    }

    fn version(text: &str) -> Version {
        text.parse().unwrap()
    }

    #[test]
    fn flat_layout_is_one_version_from_the_manifest() {
        let f = template_folder();
        f.template("", Some("1.2.0"));
        let template = f.load_template();
        assert_eq!(versions(&template), [("1.2.0".to_owned(), true)]);
        let entry = template.default_entry();
        assert_eq!(entry.version.as_deref(), Some("1.2.0"));
        assert_eq!(entry.semver, Some(version("1.2.0")));

        // Without a manifest version: no version at all.
        let f = template_folder();
        f.template("", None);
        let template = f.load_template();
        assert!(template.versions().is_empty());
        assert!(template.default_entry().is_valid());
        assert_eq!(template.default_entry().version, None);
        assert_eq!(template.default_entry().semver, None);
    }

    #[test]
    fn versioned_layout_loads_each_version_folder() {
        let f = template_folder();
        f.template("1.0.0", None)
            .template("1.1.0", Some("1.1.0"))
            .template("2.0.0", None);
        f.write("2.0.0/parts/footer.typ", "footer");
        let template = f.load_template();
        assert_eq!(
            versions(&template),
            [
                ("2.0.0".to_owned(), true),
                ("1.1.0".to_owned(), true),
                ("1.0.0".to_owned(), true),
            ]
        );
        for entry in template.versions() {
            assert_eq!(entry.id.as_str(), "tpl");
            assert_eq!(
                entry.semver.map(|v| v.to_string()).as_deref(),
                entry.version.as_deref()
            );
        }
        // Each version folder is its own root, with its own fingerprint.
        let newest = template.default_entry();
        assert_eq!(newest.version.as_deref(), Some("2.0.0"));
        assert!(newest.files.contains_key("parts/footer.typ"));
        assert!(newest.files.contains_key("main.typ"));
        assert_ne!(
            newest.fingerprint,
            template.versions()[1].fingerprint,
            "versions share a fingerprint"
        );
    }

    #[test]
    fn mixed_layout_makes_the_template_invalid() {
        let f = template_folder();
        f.template("", Some("1.0.0")).template("2.0.0", None);
        let template = f.load_template();
        assert_eq!(
            template.default_entry().invalid_reason(),
            Some("put the template files either at the root or in version folders, not both")
        );
        assert!(template.versions().is_empty());
        // Any file at the root counts, not only `main.typ`.
        let f = template_folder();
        f.template("1.0.0", None);
        f.write("notes.txt", "x");
        assert_eq!(
            f.load_template().default_entry().invalid_reason(),
            Some(MIXED_LAYOUT)
        );
    }

    #[test]
    fn non_version_folders_are_skipped() {
        let f = template_folder();
        f.template("1.0.0", None)
            .template("draft", None)
            .template("1.2", None)
            .template("v2.0.0", None)
            .template("01.0.0", None);
        f.write(".hidden/main.typ", "x");
        let template = f.load_template();
        assert_eq!(versions(&template), [("1.0.0".to_owned(), true)]);
    }

    #[test]
    fn manifest_version_must_match_its_folder() {
        let f = template_folder();
        f.template("1.0.0", Some("1.0.0"))
            .template("1.3.0", Some("1.2.0"));
        let template = f.load_template();
        assert_eq!(
            versions(&template),
            [("1.3.0".to_owned(), false), ("1.0.0".to_owned(), true)]
        );
        assert_eq!(
            template.versions()[0].invalid_reason(),
            Some(r#"template.json: version "1.2.0" does not match its folder "1.3.0""#)
        );
        // The invalid version does not hide the valid one.
        assert_eq!(template.default_entry().version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn invalid_version_keeps_its_folder_version() {
        let f = template_folder();
        f.template("1.0.0", None).template("2.0.0", None);
        f.write("2.0.0/schema.json", "{");
        let template = f.load_template();
        let invalid = template.versions()[0];
        assert_eq!(invalid.version.as_deref(), Some("2.0.0"));
        assert_eq!(invalid.semver, Some(version("2.0.0")));
        assert!(invalid.invalid_reason().unwrap().contains("schema.json"));
    }

    #[test]
    fn size_limit_applies_to_each_version_folder() {
        let f = template_folder();
        f.template("1.0.0", None).template("2.0.0", None);
        f.write("1.0.0/assets/a.bin", vec![0u8; 600]);
        f.write("2.0.0/assets/b.bin", vec![0u8; 600]);
        let template = load_template(&f.dir, "tpl".parse().unwrap(), 1024, None).unwrap();
        // 1200 bytes in all, but each version is under the limit.
        assert_eq!(
            versions(&template),
            [("2.0.0".to_owned(), true), ("1.0.0".to_owned(), true)]
        );
    }

    #[test]
    fn flat_manifest_version_that_does_not_parse_is_shown_as_is() {
        for written in ["1.0", "v1"] {
            let f = template_folder();
            f.template("", Some(written));
            let template = f.load_template();
            assert_eq!(versions(&template), [(written.to_owned(), true)]);
            assert_eq!(template.default_entry().semver, None);
        }
    }

    #[test]
    fn versions_are_ordered_numerically() {
        let f = template_folder();
        f.template("1.9.0", None)
            .template("1.10.0", None)
            .template("1.2.0", None);
        let listed: Vec<_> = versions(&f.load_template())
            .into_iter()
            .map(|(v, _)| v)
            .collect();
        assert_eq!(listed, ["1.10.0", "1.9.0", "1.2.0"]);
    }

    #[test]
    fn unchanged_versions_are_kept_on_reload() {
        let f = template_folder();
        f.template("1.0.0", None).template("2.0.0", None);
        let first = f.load_template();
        f.write("2.0.0/main.typ", "= Changed");
        let id: TemplateId = "tpl".parse().unwrap();
        let second = load_template(&f.dir, id, u64::MAX, Some(&first)).unwrap();
        assert!(Arc::ptr_eq(first.versions()[1], second.versions()[1]));
        assert!(!Arc::ptr_eq(first.versions()[0], second.versions()[0]));
        assert_ne!(first.fingerprint(), second.fingerprint());
    }
}
