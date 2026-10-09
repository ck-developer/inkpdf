//! Registre en mémoire des templates du volume.
//!
//! Les lecteurs obtiennent un instantané cohérent (`ArcSwap`) ; les mises à jour remplacent la
//! carte entière d'un seul coup.

pub mod fingerprint;
pub mod loader;
pub mod watcher;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;

pub use fingerprint::Fingerprint;
pub use loader::{LoadOutcome, TemplateEntry, TemplateStatus};

use crate::template::TemplateId;

/// Nombre d'essais d'un chargement instable lors d'un scan complet.
const SCAN_ATTEMPTS: usize = 3;

pub type TemplateMap = BTreeMap<TemplateId, Arc<TemplateEntry>>;

pub struct Registry {
    templates_dir: PathBuf,
    max_template_bytes: u64,
    map: ArcSwap<TemplateMap>,
    ready: AtomicBool,
    /// Rafraîchissements sérialisés ; empreintes candidates observées une fois, en attente de
    /// confirmation de stabilité.
    candidates: Mutex<HashMap<TemplateId, Fingerprint>>,
}

/// Issue d'un rafraîchissement.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RefreshOutcome {
    /// Des changements attendent une observation de confirmation.
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

    /// Vrai une fois le scan initial terminé.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    pub fn set_ready(&self) {
        self.ready.store(true, Ordering::Release);
    }

    pub fn get(&self, id: &TemplateId) -> Option<Arc<TemplateEntry>> {
        self.map.load().get(id).cloned()
    }

    /// Instantané de la carte, trié par identifiant.
    pub fn snapshot(&self) -> Arc<TemplateMap> {
        self.map.load_full()
    }

    /// Entrées triées par identifiant.
    pub fn list(&self) -> Vec<Arc<TemplateEntry>> {
        self.map.load().values().cloned().collect()
    }

    /// Nombre de templates valides.
    pub fn valid_count(&self) -> usize {
        self.map.load().values().filter(|e| e.is_valid()).count()
    }

    /// Publie atomiquement une nouvelle carte.
    pub fn replace(&self, map: TemplateMap) {
        self.map.store(Arc::new(map));
    }

    /// Scan complet du volume (bloquant) puis publication.
    pub fn scan_all(&self) {
        let mut map = TemplateMap::new();
        for (id, dir) in self.template_dirs(true) {
            if let Some(entry) = self.load_with_retries(&dir, id) {
                log_loaded(&entry);
                map.insert(entry.id.clone(), Arc::new(entry));
            }
        }
        self.replace(map);
    }

    /// Rafraîchissement incrémental (bloquant) : ne recharge un template que si son empreinte a
    /// changé **et** est restée identique entre deux observations ; pendant ce temps, la version
    /// précédente reste servie. Les dossiers disparus sont retirés. Publication atomique.
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
                // Dossier en cours de suppression ou illisible : nouvel essai au tour suivant.
                outcome.needs_confirmation = true;
                continue;
            };
            if current
                .get(&id)
                .is_some_and(|e| e.fingerprint == fingerprint)
            {
                candidates.remove(&id);
                continue;
            }
            if candidates.get(&id) != Some(&fingerprint) {
                // Première observation de cette version : attendre la confirmation.
                candidates.insert(id, fingerprint);
                outcome.needs_confirmation = true;
                continue;
            }
            match loader::load(&dir, id.clone(), self.max_template_bytes) {
                LoadOutcome::Loaded(entry) => {
                    candidates.remove(&id);
                    log_loaded(&entry);
                    next.insert(id, Arc::from(entry));
                    changed = true;
                }
                LoadOutcome::Unstable => {
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

    fn load_with_retries(&self, dir: &Path, id: TemplateId) -> Option<TemplateEntry> {
        for _ in 0..SCAN_ATTEMPTS {
            if let LoadOutcome::Loaded(entry) =
                loader::load(dir, id.clone(), self.max_template_bytes)
            {
                return Some(*entry);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        tracing::warn!(event = "template.unstable", templateId = %id, "template is still changing, will retry");
        None
    }

    /// Dossiers de templates du volume, identifiants valides seulement. `verbose` active les
    /// avertissements (scan initial seulement, pour ne pas les répéter à chaque rescan).
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

fn log_loaded(entry: &TemplateEntry) {
    match &entry.status {
        TemplateStatus::Valid => tracing::info!(
            event = "template.loaded",
            templateId = %entry.id,
            fingerprint = %entry.fingerprint,
        ),
        TemplateStatus::Invalid { reason } => tracing::warn!(
            event = "template.invalid",
            templateId = %entry.id,
            reason = %reason,
        ),
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
        let ids: Vec<String> = registry.list().iter().map(|e| e.id.to_string()).collect();
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
        let before = registry.get(&alpha).unwrap().fingerprint;

        fs::write(volume.path().join("alpha/main.typ"), "= changed").unwrap();
        assert!(registry.refresh().needs_confirmation);
        assert_eq!(registry.get(&alpha).unwrap().fingerprint, before);

        assert!(!registry.refresh().needs_confirmation);
        assert_ne!(registry.get(&alpha).unwrap().fingerprint, before);
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

    #[test]
    fn missing_volume_gives_empty_registry() {
        let registry = Registry::new("/nonexistent/inkpdf-volume", u64::MAX);
        registry.scan_all();
        assert!(registry.list().is_empty());
    }
}
