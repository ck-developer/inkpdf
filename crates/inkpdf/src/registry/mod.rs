//! In-memory registry of the volume's templates.
//!
//! Readers get a consistent snapshot (`ArcSwap`); updates replace the whole map at once.

pub mod fingerprint;
pub mod loader;
pub mod version;
pub mod watcher;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;

pub use fingerprint::Fingerprint;
pub use loader::{LoadOutcome, TemplateEntry, TemplateStatus};
pub use version::{Version, VersionSelector};

use crate::template::TemplateId;

/// Number of attempts for an unstable load during a full scan.
const SCAN_ATTEMPTS: usize = 3;

pub type TemplateMap = BTreeMap<TemplateId, Arc<TemplateVersions>>;

/// Every loaded version of one template folder (research R9 of feature 005).
#[derive(Debug, Clone)]
pub struct TemplateVersions {
    /// Of the whole template folder: tells a rescan whether anything changed.
    pub(crate) fingerprint: Fingerprint,
    pub(crate) layout: Layout,
}

#[derive(Debug, Clone)]
pub(crate) enum Layout {
    /// The flat layout: one entry, with the manifest's version or none.
    Single(Arc<TemplateEntry>),
    /// The versioned layout: one entry per version folder, at least one.
    Versioned(BTreeMap<Version, Arc<TemplateEntry>>),
    /// A folder that cannot be laid out (mixed layout, unreadable folder): one invalid entry,
    /// without a version, used whatever version is asked for.
    Broken(Arc<TemplateEntry>),
}

/// No version of a template matches the selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionNotFound {
    /// The template's versions, newest first, as [`TemplateVersions::versions`] gives them.
    pub available: Vec<String>,
}

impl TemplateVersions {
    /// Fingerprint of the whole template folder.
    pub fn fingerprint(&self) -> Fingerprint {
        self.fingerprint
    }

    /// The versions that have a version, newest first: one per version folder, or the flat
    /// template's single entry when its manifest gives a version.
    pub fn versions(&self) -> Vec<&Arc<TemplateEntry>> {
        self.entries()
            .into_iter()
            .filter(|entry| entry.version.is_some())
            .collect()
    }

    /// Every entry, newest first.
    pub fn entries(&self) -> Vec<&Arc<TemplateEntry>> {
        match &self.layout {
            Layout::Single(entry) | Layout::Broken(entry) => vec![entry],
            Layout::Versioned(versions) => versions.values().rev().collect(),
        }
    }

    /// The version used when none is asked for: the highest valid one, or the highest one when
    /// none is valid.
    pub fn default_entry(&self) -> &Arc<TemplateEntry> {
        match &self.layout {
            Layout::Single(entry) | Layout::Broken(entry) => entry,
            Layout::Versioned(versions) => {
                highest(versions.values()).expect("at least one version")
            }
        }
    }

    /// The version folder `version`, in the versioned layout.
    pub(crate) fn get(&self, version: Version) -> Option<&Arc<TemplateEntry>> {
        match &self.layout {
            Layout::Versioned(versions) => versions.get(&version),
            _ => None,
        }
    }

    /// The version a caller asks for:
    /// - none: the [default](Self::default_entry);
    /// - a major (`1`): the highest valid `1.x.y`, or the highest `1.x.y` when none is valid;
    /// - an exact version (`1.0.0`): that version, valid or not.
    ///
    /// A flat template whose version does not parse never matches a selector; a template that
    /// cannot be laid out gives its invalid entry whatever is asked for.
    pub fn resolve(
        &self,
        selector: Option<VersionSelector>,
    ) -> Result<&Arc<TemplateEntry>, VersionNotFound> {
        let Some(selector) = selector else {
            return Ok(self.default_entry());
        };
        let matching = match &self.layout {
            Layout::Broken(entry) => return Ok(entry),
            Layout::Single(entry) => {
                let matches = entry.semver.is_some_and(|v| selector.matches(v));
                matches.then_some(entry)
            }
            Layout::Versioned(versions) => highest(
                versions
                    .iter()
                    .filter(|(version, _)| selector.matches(**version))
                    .map(|(_, entry)| entry),
            ),
        };
        matching.ok_or_else(|| VersionNotFound {
            available: self
                .versions()
                .iter()
                .filter_map(|entry| entry.version.clone())
                .collect(),
        })
    }
}

/// Of entries in ascending version order: the last valid one, or the last one.
fn highest<'a>(
    entries: impl DoubleEndedIterator<Item = &'a Arc<TemplateEntry>> + Clone,
) -> Option<&'a Arc<TemplateEntry>> {
    entries
        .clone()
        .rev()
        .find(|entry| entry.is_valid())
        .or_else(|| entries.last())
}

pub struct Registry {
    templates_dir: PathBuf,
    max_template_bytes: u64,
    map: ArcSwap<TemplateMap>,
    ready: AtomicBool,
    /// Serialized refreshes; candidate fingerprints seen once, awaiting confirmation that they
    /// are stable.
    candidates: Mutex<HashMap<TemplateId, Fingerprint>>,
}

/// Outcome of a refresh.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RefreshOutcome {
    /// Some changes are waiting for a confirming observation.
    pub needs_confirmation: bool,
}

impl Registry {
    pub fn new(templates_dir: impl Into<PathBuf>, max_template_bytes: u64) -> Self {
        Self {
            templates_dir: templates_dir.into(),
            max_template_bytes,
            map: ArcSwap::from_pointee(TemplateMap::new()),
            ready: AtomicBool::new(false),
            candidates: Mutex::new(HashMap::new()),
        }
    }

    pub fn templates_dir(&self) -> &Path {
        &self.templates_dir
    }

    pub fn max_template_bytes(&self) -> u64 {
        self.max_template_bytes
    }

    /// True once the initial scan has finished.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    pub fn set_ready(&self) {
        self.ready.store(true, Ordering::Release);
    }

    /// Every version of the template `id`.
    pub fn get(&self, id: &TemplateId) -> Option<Arc<TemplateVersions>> {
        self.map.load().get(id).cloned()
    }

    /// Snapshot of the map, sorted by id.
    pub fn snapshot(&self) -> Arc<TemplateMap> {
        self.map.load_full()
    }

    /// Templates sorted by id.
    pub fn list(&self) -> Vec<Arc<TemplateVersions>> {
        self.map.load().values().cloned().collect()
    }

    /// Number of templates whose default version is valid.
    pub fn valid_count(&self) -> usize {
        self.map
            .load()
            .values()
            .filter(|t| t.default_entry().is_valid())
            .count()
    }

    /// Atomically publishes a new map.
    pub fn replace(&self, map: TemplateMap) {
        self.map.store(Arc::new(map));
    }

    /// Full scan of the volume (blocking), then publication.
    pub fn scan_all(&self) {
        let mut map = TemplateMap::new();
        for (id, dir) in self.template_dirs(true) {
            if let Some(template) = self.load_with_retries(&dir, id.clone()) {
                log_loaded(&template, None);
                map.insert(id, Arc::new(template));
            }
        }
        self.replace(map);
    }

    /// Incremental refresh (blocking): reloads a template only if its fingerprint has changed
    /// **and** stayed the same across two observations; meanwhile, the previous version is still
    /// served. Directories that disappeared are removed. Atomic publication.
    pub fn refresh(&self) -> RefreshOutcome {
        let mut candidates = self.candidates.lock().expect("registry lock poisoned");
        let current = self.snapshot();
        let mut next = (*current).clone();
        let mut changed = false;
        let mut outcome = RefreshOutcome::default();

        let dirs = self.template_dirs(false);
        let present: HashSet<TemplateId> = dirs.iter().map(|(id, _)| id.clone()).collect();

        for (id, dir) in dirs {
            let Ok(fingerprint) = Fingerprint::of(&dir) else {
                // Directory being deleted or unreadable: try again on the next round.
                outcome.needs_confirmation = true;
                continue;
            };
            let previous = current.get(&id);
            if previous.is_some_and(|t| t.fingerprint == fingerprint) {
                candidates.remove(&id);
                continue;
            }
            if candidates.get(&id) != Some(&fingerprint) {
                // First observation of this version: wait for confirmation.
                candidates.insert(id, fingerprint);
                outcome.needs_confirmation = true;
                continue;
            }
            // Unchanged version folders keep their entry.
            let loaded = loader::load_template(
                &dir,
                id.clone(),
                self.max_template_bytes,
                previous.map(Arc::as_ref),
            );
            match loaded {
                Some(template) => {
                    candidates.remove(&id);
                    log_loaded(&template, previous.map(Arc::as_ref));
                    next.insert(id, Arc::new(template));
                    changed = true;
                }
                None => {
                    candidates.remove(&id);
                    outcome.needs_confirmation = true;
                }
            }
        }

        for id in current.keys().filter(|id| !present.contains(*id)) {
            next.remove(id);
            changed = true;
            tracing::info!(event = "template.removed", templateId = %id);
        }
        candidates.retain(|id, _| present.contains(id));

        if changed {
            self.replace(next);
        }
        outcome
    }

    fn load_with_retries(&self, dir: &Path, id: TemplateId) -> Option<TemplateVersions> {
        for _ in 0..SCAN_ATTEMPTS {
            if let Some(template) =
                loader::load_template(dir, id.clone(), self.max_template_bytes, None)
            {
                return Some(template);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        tracing::warn!(event = "template.unstable", templateId = %id, "template is still changing, will retry");
        None
    }

    /// Template directories of the volume, valid ids only. `verbose` enables warnings (initial
    /// scan only, so they are not repeated on every rescan).
    fn template_dirs(&self, verbose: bool) -> Vec<(TemplateId, PathBuf)> {
        let entries = match fs::read_dir(&self.templates_dir) {
            Ok(entries) => entries,
            Err(e) => {
                if !verbose {
                    return Vec::new();
                }
                tracing::warn!(
                    event = "registry.volume_unreadable",
                    dir = %self.templates_dir.display(),
                    error = %e,
                    "templates volume is missing or unreadable"
                );
                return Vec::new();
            }
        };
        let mut dirs = Vec::new();
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if name.starts_with('.') {
                continue;
            }
            let path = entry.path();
            if !fs::metadata(&path).is_ok_and(|m| m.is_dir()) {
                continue;
            }
            match name.parse::<TemplateId>() {
                Ok(id) => dirs.push((id, path)),
                Err(_) if verbose => tracing::warn!(
                    event = "template.ignored",
                    dir = %name,
                    "directory name is not a valid template id"
                ),
                Err(_) => {}
            }
        }
        if verbose && dirs.is_empty() {
            tracing::warn!(
                event = "registry.empty",
                dir = %self.templates_dir.display(),
                "no template found in the volume"
            );
        }
        dirs.sort_by(|a, b| a.0.cmp(&b.0));
        dirs
    }
}

/// Logs each version of `template` that was (re)loaded, not those kept from `previous`.
fn log_loaded(template: &TemplateVersions, previous: Option<&TemplateVersions>) {
    let kept = |entry: &Arc<TemplateEntry>| {
        previous.is_some_and(|p| p.entries().iter().any(|old| Arc::ptr_eq(old, entry)))
    };
    for entry in template.entries().into_iter().filter(|e| !kept(e)) {
        let version = entry.version.as_deref();
        match &entry.status {
            TemplateStatus::Valid => tracing::info!(
                event = "template.loaded",
                templateId = %entry.id,
                templateVersion = version,
                fingerprint = %entry.fingerprint,
            ),
            TemplateStatus::Invalid { reason } => tracing::warn!(
                event = "template.invalid",
                templateId = %entry.id,
                templateVersion = version,
                reason = %reason,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_template(volume: &Path, id: &str) {
        let dir = volume.join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("main.typ"), "= x").unwrap();
        fs::write(
            dir.join("schema.json"),
            r#"{"type":"object","properties":{"data":{}}}"#,
        )
        .unwrap();
    }

    #[test]
    fn scan_all_lists_templates_sorted_and_ignores_bad_names() {
        let volume = tempfile::tempdir().unwrap();
        write_template(volume.path(), "zeta");
        write_template(volume.path(), "alpha");
        write_template(volume.path(), "Bad.Name");
        write_template(volume.path(), ".hidden");
        fs::create_dir(volume.path().join("broken")).unwrap();

        let registry = Registry::new(volume.path(), u64::MAX);
        registry.scan_all();
        let ids: Vec<String> = registry
            .list()
            .iter()
            .map(|t| t.default_entry().id.to_string())
            .collect();
        assert_eq!(ids, ["alpha", "broken", "zeta"]);
        assert_eq!(registry.valid_count(), 2);
        assert!(registry.get(&"alpha".parse().unwrap()).is_some());
    }

    #[test]
    fn refresh_loads_a_change_only_once_it_is_stable() {
        let volume = tempfile::tempdir().unwrap();
        write_template(volume.path(), "alpha");
        let registry = Registry::new(volume.path(), u64::MAX);
        registry.scan_all();
        let alpha: TemplateId = "alpha".parse().unwrap();
        let before = registry.get(&alpha).unwrap().fingerprint();

        fs::write(volume.path().join("alpha/main.typ"), "= changed").unwrap();
        assert!(registry.refresh().needs_confirmation);
        assert_eq!(registry.get(&alpha).unwrap().fingerprint(), before);

        assert!(!registry.refresh().needs_confirmation);
        assert_ne!(registry.get(&alpha).unwrap().fingerprint(), before);
    }

    #[test]
    fn refresh_adds_and_removes_templates() {
        let volume = tempfile::tempdir().unwrap();
        let registry = Registry::new(volume.path(), u64::MAX);
        registry.scan_all();
        write_template(volume.path(), "beta");
        registry.refresh();
        registry.refresh();
        assert!(registry.get(&"beta".parse().unwrap()).is_some());

        fs::remove_dir_all(volume.path().join("beta")).unwrap();
        registry.refresh();
        assert!(registry.list().is_empty());
    }

    /// A template `id` laid out with one valid folder per version; `invalid` ones lack
    /// `main.typ`.
    fn write_versions(volume: &Path, id: &str, valid: &[&str], invalid: &[&str]) {
        for version in valid {
            write_template(volume, &format!("{id}/{version}"));
        }
        for version in invalid {
            write_template(volume, &format!("{id}/{version}"));
            fs::remove_file(volume.join(format!("{id}/{version}/main.typ"))).unwrap();
        }
    }

    fn scanned(volume: &Path) -> Registry {
        let registry = Registry::new(volume, u64::MAX);
        registry.scan_all();
        registry
    }

    fn select(
        registry: &Registry,
        id: &str,
        selector: Option<&str>,
    ) -> Result<String, Vec<String>> {
        let template = registry.get(&id.parse().unwrap()).unwrap();
        let selector = selector.map(|s| s.parse().unwrap());
        template
            .resolve(selector)
            .map(|entry| entry.version.clone().unwrap_or_default())
            .map_err(|not_found| not_found.available)
    }

    #[test]
    fn selection_follows_the_versions() {
        let volume = tempfile::tempdir().unwrap();
        write_versions(
            volume.path(),
            "facture",
            &["1.0.0", "1.1.0", "1.9.0", "1.10.0", "3.0.0"],
            &["1.11.0", "2.0.0", "4.0.0"],
        );
        let registry = scanned(volume.path());
        let all = [
            "4.0.0", "3.0.0", "2.0.0", "1.11.0", "1.10.0", "1.9.0", "1.1.0", "1.0.0",
        ];
        // Newest first, numerically.
        let template = registry.get(&"facture".parse().unwrap()).unwrap();
        let listed: Vec<_> = template
            .versions()
            .iter()
            .map(|e| e.version.clone().unwrap())
            .collect();
        assert_eq!(listed, all);

        // None: the highest valid version, not the highest one.
        assert_eq!(select(&registry, "facture", None), Ok("3.0.0".into()));
        assert_eq!(registry.valid_count(), 1);
        // A major: its highest valid version (1.10.0 > 1.9.0; 1.11.0 is invalid).
        assert_eq!(select(&registry, "facture", Some("1")), Ok("1.10.0".into()));
        assert_eq!(select(&registry, "facture", Some("3")), Ok("3.0.0".into()));
        // A major whose versions are all invalid: its highest one, to report why.
        assert_eq!(select(&registry, "facture", Some("2")), Ok("2.0.0".into()));
        // An exact version: that version, even invalid.
        assert_eq!(
            select(&registry, "facture", Some("1.0.0")),
            Ok("1.0.0".into())
        );
        assert_eq!(
            select(&registry, "facture", Some("1.11.0")),
            Ok("1.11.0".into())
        );
        // No match: the versions available.
        let available = all.map(String::from).to_vec();
        assert_eq!(
            select(&registry, "facture", Some("5")),
            Err(available.clone())
        );
        assert_eq!(select(&registry, "facture", Some("1.2.0")), Err(available));
    }

    #[test]
    fn all_versions_invalid_gives_the_highest_by_default() {
        let volume = tempfile::tempdir().unwrap();
        write_versions(volume.path(), "facture", &[], &["1.0.0", "2.0.0"]);
        let registry = scanned(volume.path());
        let template = registry.get(&"facture".parse().unwrap()).unwrap();
        assert!(!template.default_entry().is_valid());
        assert_eq!(template.default_entry().version.as_deref(), Some("2.0.0"));
        assert_eq!(registry.valid_count(), 0);
    }

    #[test]
    fn flat_manifest_versions_are_selectable_when_they_parse() {
        let volume = tempfile::tempdir().unwrap();
        for (id, version) in [("semver", "1.2.0"), ("short", "1.0"), ("prefixed", "v1")] {
            write_template(volume.path(), id);
            fs::write(
                volume.path().join(id).join("template.json"),
                format!(r#"{{"version":"{version}"}}"#),
            )
            .unwrap();
        }
        write_template(volume.path(), "none");
        let registry = scanned(volume.path());

        assert_eq!(select(&registry, "semver", None), Ok("1.2.0".into()));
        assert_eq!(select(&registry, "semver", Some("1")), Ok("1.2.0".into()));
        assert_eq!(
            select(&registry, "semver", Some("1.2.0")),
            Ok("1.2.0".into())
        );
        assert_eq!(
            select(&registry, "semver", Some("2")),
            Err(vec!["1.2.0".into()])
        );

        // Shown as is, used without a selector, never matched by one.
        for version in ["1.0", "v1"] {
            let id = if version == "1.0" {
                "short"
            } else {
                "prefixed"
            };
            let template = registry.get(&id.parse().unwrap()).unwrap();
            assert_eq!(template.default_entry().semver, None);
            assert_eq!(select(&registry, id, None), Ok(version.into()));
            for selector in ["1", "1.0.0"] {
                assert_eq!(
                    select(&registry, id, Some(selector)),
                    Err(vec![version.into()])
                );
            }
        }

        // Without a version: no version to select.
        assert_eq!(select(&registry, "none", None), Ok(String::new()));
        assert_eq!(select(&registry, "none", Some("1")), Err(vec![]));
    }

    #[test]
    fn mixed_layout_is_invalid_whatever_version_is_asked_for() {
        let volume = tempfile::tempdir().unwrap();
        write_template(volume.path(), "facture");
        write_template(volume.path(), "facture/1.0.0");
        let registry = scanned(volume.path());
        let template = registry.get(&"facture".parse().unwrap()).unwrap();
        assert!(template.versions().is_empty());
        for selector in [None, Some("1".parse().unwrap())] {
            let entry = template.resolve(selector).unwrap();
            assert_eq!(entry.invalid_reason(), Some(loader::MIXED_LAYOUT));
        }
    }

    #[test]
    fn refresh_reloads_only_the_changed_version() {
        let volume = tempfile::tempdir().unwrap();
        write_versions(volume.path(), "facture", &["1.0.0", "2.0.0"], &[]);
        let registry = scanned(volume.path());
        let id: TemplateId = "facture".parse().unwrap();
        let before = registry.get(&id).unwrap();

        fs::write(volume.path().join("facture/2.0.0/main.typ"), "= changed").unwrap();
        write_template(volume.path(), "facture/3.0.0");
        registry.refresh();
        registry.refresh();
        let after = registry.get(&id).unwrap();
        let versions = |t: &TemplateVersions| -> Vec<String> {
            t.versions()
                .iter()
                .map(|e| e.version.clone().unwrap())
                .collect()
        };
        assert_eq!(versions(&after), ["3.0.0", "2.0.0", "1.0.0"]);
        assert!(Arc::ptr_eq(before.versions()[1], after.versions()[2]));
        assert!(!Arc::ptr_eq(before.versions()[0], after.versions()[1]));

        fs::remove_dir_all(volume.path().join("facture/3.0.0")).unwrap();
        registry.refresh();
        registry.refresh();
        assert_eq!(versions(&registry.get(&id).unwrap()), ["2.0.0", "1.0.0"]);
    }

    #[test]
    fn missing_volume_gives_empty_registry() {
        let registry = Registry::new("/nonexistent/inkpdf-volume", u64::MAX);
        registry.scan_all();
        assert!(registry.list().is_empty());
    }
}
