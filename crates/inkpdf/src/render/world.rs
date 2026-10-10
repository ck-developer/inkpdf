//! Sandboxed Typst `World`: serves only the template's in-memory snapshot.
//!
//! No disk, network or environment variable access during rendering. Packages come only from
//! the set bundled into the binary (`crate::packages`), each confined to its own root.

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

/// Entry point of a template.
pub const MAIN_FILE: &str = "main.typ";

/// Package file sources, shared between renders: their `FileId`s are stable, and reusing the
/// same `Source` lets comemo reuse the packages' evaluation (R6).
static PACKAGE_SOURCES: LazyLock<Mutex<HashMap<FileId, Source>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct SandboxWorld {
    entry: Arc<TemplateEntry>,
    fonts: Arc<FontSet>,
    library: LazyHash<Library>,
    main: FileId,
    cancel: Arc<AtomicBool>,
    /// Sources parsed during this compilation.
    sources: Mutex<HashMap<FileId, Source>>,
}

impl SandboxWorld {
    /// `body` is the validated `{ data, layout }` body; `cancel` interrupts the compilation on
    /// the next world access.
    pub fn new(
        entry: Arc<TemplateEntry>,
        body: &serde_json::Value,
        cancel: Arc<AtomicBool>,
    ) -> Self {
        let mut inputs = Dict::new();
        for key in ["data", "layout"] {
            // A missing `layout` (schema without `layout`) is seen as an empty dictionary.
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

    /// Path shown in diagnostics: relative to the template folder, or prefixed with the
    /// package (`@preview/zero:0.7.1/src/num.typ`).
    pub fn relative_path(id: FileId) -> Option<String> {
        let path = id.vpath().get_without_slash();
        match id.root() {
            VirtualRoot::Project => Some(path.to_owned()),
            VirtualRoot::Package(spec) => Some(format!("{spec}/{path}")),
        }
    }

    fn lookup(&self, id: FileId) -> FileResult<&Bytes> {
        self.check_cancelled()?;
        // The virtual path is already normalized by Typst (no `..` escapes the root).
        let vpath = id.vpath();
        let not_found = || FileError::NotFound(PathBuf::from(vpath.get_with_slash()));
        match id.root() {
            // Only the template snapshot is consulted.
            VirtualRoot::Project => self
                .entry
                .files
                .get(vpath.get_without_slash())
                .ok_or_else(not_found),
            // Exact match among the bundled packages; a package only sees its own
            // files.
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
