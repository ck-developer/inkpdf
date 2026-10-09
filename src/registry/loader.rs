//! Chargement d'un dossier de template en une entrée du registre.
//!
//! C'est le seul endroit où les fichiers d'un template sont lus : le rendu ne sert ensuite que
//! l'instantané en mémoire.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use time::OffsetDateTime;
use typst::foundations::Bytes;

use super::fingerprint::{self, Fingerprint};
use crate::render::fonts::{self, FontSet};
use crate::template::{Manifest, TemplateId, TemplateSchema};

/// Statut d'un template après chargement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateStatus {
    Valid,
    Invalid { reason: String },
}

/// Entrée du registre : un template chargé, valide ou non.
pub struct TemplateEntry {
    pub id: TemplateId,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub status: TemplateStatus,
    /// Présent si le template est valide.
    pub schema: Option<TemplateSchema>,
    /// Instantané de tous les fichiers ; clé = chemin relatif séparé par `/`.
    pub files: HashMap<String, Bytes>,
    /// Polices embarquées + polices du template (présent si valide).
    pub fonts: Option<Arc<FontSet>>,
    pub fingerprint: Fingerprint,
    pub loaded_at: OffsetDateTime,
}

impl std::fmt::Debug for TemplateEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateEntry")
            .field("id", &self.id)
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

/// Résultat d'un chargement.
#[derive(Debug)]
pub enum LoadOutcome {
    Loaded(Box<TemplateEntry>),
    /// Le dossier a changé pendant la lecture : nouvel essai plus tard.
    Unstable,
}

/// Charge le dossier `dir` du template `id`.
pub fn load(dir: &Path, id: TemplateId, max_template_bytes: u64) -> LoadOutcome {
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

    LoadOutcome::Loaded(Box::new(validate(id, files, before.fingerprint)))
}

/// Applique les règles de validité du data-model à un instantané.
fn validate(
    id: TemplateId,
    files: HashMap<String, Bytes>,
    fingerprint: Fingerprint,
) -> TemplateEntry {
    let manifest = match files.get("template.json") {
        None => Manifest::default(),
        Some(bytes) => match Manifest::parse(bytes) {
            Ok(manifest) => manifest,
            Err(e) => return invalid(id, None, fingerprint, format!("template.json: {e}")),
        },
    };

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
            name: manifest.name.unwrap_or_else(|| id.to_string()),
            description: manifest.description,
            version: manifest.version,
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
        id,
        status: TemplateStatus::Invalid { reason },
        schema: None,
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
            r#"{"type":"object","properties":{"design":{}}}"#,
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
        // Un fichier qui change entre les deux parcours rend le chargement instable : on simule
        // le changement en modifiant le dossier depuis un second thread pendant la lecture d'un
        // gros fichier.
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
}
