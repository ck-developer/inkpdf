//! `World` Typst en bac à sable : ne sert que l'instantané en mémoire du template.
//!
//! Aucun accès disque, réseau, ni variable d'environnement pendant le rendu. Les paquets ne
//! viennent que de l'ensemble intégré au binaire (`crate::packages`), chacun confiné à sa
//! propre racine.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use typst::diag::{FileError, FileResult, PackageError};
use typst::foundations::{Bytes, Datetime, Dict, Duration, Value};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use super::fonts::FontSet;
use super::value::json_to_value;
use crate::registry::TemplateEntry;

/// Point d'entrée d'un template.
pub const MAIN_FILE: &str = "main.typ";

/// Sources des fichiers de paquets, partagées entre rendus : leurs `FileId` sont stables, et
/// réutiliser la même `Source` permet à comemo de réutiliser l'évaluation des paquets (R6).
static PACKAGE_SOURCES: LazyLock<Mutex<HashMap<FileId, Source>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct SandboxWorld {
    entry: Arc<TemplateEntry>,
    fonts: Arc<FontSet>,
    library: LazyHash<Library>,
    main: FileId,
    cancel: Arc<AtomicBool>,
    /// Sources parsées pendant cette compilation.
    sources: Mutex<HashMap<FileId, Source>>,
}

impl SandboxWorld {
    /// `body` est le corps validé `{ data, design }` ; `cancel` interrompt la compilation au
    /// prochain accès au monde.
    pub fn new(
        entry: Arc<TemplateEntry>,
        body: &serde_json::Value,
        cancel: Arc<AtomicBool>,
    ) -> Self {
        let mut inputs = Dict::new();
        for key in ["data", "design"] {
            // Un `design` absent (schéma sans `design`) est vu comme un dictionnaire vide.
            let value = body
                .get(key)
                .map_or_else(|| Value::Dict(Dict::new()), json_to_value);
            inputs.insert(key.into(), value);
        }
        let library = Library::builder().with_inputs(inputs).build();
        let fonts = entry.fonts.clone().unwrap_or_else(FontSet::embedded);
        let main = RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new(MAIN_FILE).expect("valid path"),
        )
        .intern();
        Self {
            entry,
            fonts,
            library: LazyHash::new(library),
            main,
            cancel,
            sources: Mutex::new(HashMap::new()),
        }
    }

    fn check_cancelled(&self) -> FileResult<()> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(FileError::Other(Some("render cancelled".into())));
        }
        Ok(())
    }

    /// Chemin affiché dans les diagnostics : relatif au dossier du template, ou préfixé par
    /// le paquet (`@preview/zero:0.7.1/src/num.typ`).
    pub fn relative_path(id: FileId) -> Option<String> {
        let path = id.vpath().get_without_slash();
        match id.root() {
            VirtualRoot::Project => Some(path.to_owned()),
            VirtualRoot::Package(spec) => Some(format!("{spec}/{path}")),
        }
    }

    fn lookup(&self, id: FileId) -> FileResult<&Bytes> {
        self.check_cancelled()?;
        // Le chemin virtuel est déjà normalisé par Typst (aucun `..` ne sort de la racine).
        let vpath = id.vpath();
        let not_found = || FileError::NotFound(PathBuf::from(vpath.get_with_slash()));
        match id.root() {
            // Seul l'instantané du template est consulté.
            VirtualRoot::Project => self
                .entry
                .files
                .get(vpath.get_without_slash())
                .ok_or_else(not_found),
            // Correspondance exacte dans les paquets intégrés ; un paquet ne voit que ses
            // propres fichiers.
            VirtualRoot::Package(spec) => crate::packages::get(spec)
                .ok_or_else(|| FileError::Package(PackageError::NotFound(spec.clone())))?
                .file(vpath.get_without_slash())
                .ok_or_else(not_found),
        }
    }

    fn parse(&self, id: FileId) -> FileResult<Source> {
        let bytes = self.lookup(id)?;
        let text = std::str::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.to_owned()))
    }
}

impl World for SandboxWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.check_cancelled()?;
        let cache = match id.root() {
            VirtualRoot::Project => &self.sources,
            VirtualRoot::Package(_) => &*PACKAGE_SOURCES,
        };
        if let Some(source) = cache.lock().expect("source cache poisoned").get(&id) {
            return Ok(source.clone());
        }
        let source = self.parse(id)?;
        cache
            .lock()
            .expect("source cache poisoned")
            .insert(id, source.clone());
        Ok(source)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.lookup(id).cloned()
    }

    fn font(&self, index: usize) -> Option<Font> {
        if self.cancel.load(Ordering::Relaxed) {
            return None;
        }
        self.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let offset_secs = offset.map_or(0.0, |d| d.seconds());
        let now = time::OffsetDateTime::now_utc() + time::Duration::seconds_f64(offset_secs);
        Datetime::from_ymd(now.year(), now.month().into(), now.day())
    }
}
