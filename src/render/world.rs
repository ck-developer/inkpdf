//! `World` Typst en bac à sable : ne sert que l'instantané en mémoire du template.
//!
//! Aucun accès disque, réseau, ni variable d'environnement pendant le rendu ; les imports de
//! paquets sont refusés.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use typst::diag::{FileError, FileResult};
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

    /// Chemin relatif au dossier du template, pour les diagnostics.
    pub fn relative_path(id: FileId) -> Option<String> {
        match id.root() {
            VirtualRoot::Project => Some(id.vpath().get_without_slash().to_owned()),
            VirtualRoot::Package(_) => None,
        }
    }

    fn lookup(&self, id: FileId) -> FileResult<&Bytes> {
        self.check_cancelled()?;
        if let VirtualRoot::Package(_) = id.root() {
            return Err(FileError::Other(Some(
                "package imports are not supported".into(),
            )));
        }
        // Le chemin virtuel est déjà normalisé par Typst (aucun `..` ne sort de la racine) ;
        // seul l'instantané est consulté.
        let vpath = id.vpath();
        self.entry
            .files
            .get(vpath.get_without_slash())
            .ok_or_else(|| FileError::NotFound(PathBuf::from(vpath.get_with_slash())))
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
        let mut sources = self.sources.lock().expect("source cache poisoned");
        if let Some(source) = sources.get(&id) {
            return Ok(source.clone());
        }
        let bytes = self.lookup(id)?;
        let text = std::str::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;
        let source = Source::new(id, text.to_owned());
        sources.insert(id, source.clone());
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
