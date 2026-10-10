//! Fonts: those embedded in the binary plus those in a template's `fonts/` folder.
//! System fonts are never loaded.

use std::sync::{Arc, OnceLock};

use typst::foundations::Bytes;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

/// Set of fonts visible to a compilation.
pub struct FontSet {
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
}

impl FontSet {
    fn new(fonts: Vec<Font>) -> Self {
        let book = FontBook::from_infos(fonts.iter().map(|f| f.info().clone()));
        Self {
            book: LazyHash::new(book),
            fonts,
        }
    }

    /// Embedded fonts, loaded once per process.
    pub fn embedded() -> Arc<FontSet> {
        static EMBEDDED: OnceLock<Arc<FontSet>> = OnceLock::new();
        EMBEDDED
            .get_or_init(|| {
                let fonts = typst_kit::fonts::embedded().map(|(font, _)| font).collect();
                Arc::new(FontSet::new(fonts))
            })
            .clone()
    }

    /// Embedded fonts plus those of a template.
    pub fn with_template_fonts(extra: Vec<Font>) -> Arc<FontSet> {
        let embedded = Self::embedded();
        if extra.is_empty() {
            return embedded;
        }
        let mut fonts = embedded.fonts.clone();
        fonts.extend(extra);
        Arc::new(FontSet::new(fonts))
    }

    pub fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    pub fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }
}

/// True for a file under `fonts/` that should be loaded as a font.
pub fn is_font_file(relative_path: &str) -> bool {
    relative_path.starts_with("fonts/") && {
        let lower = relative_path.to_ascii_lowercase();
        lower.ends_with(".ttf") || lower.ends_with(".otf") || lower.ends_with(".ttc")
    }
}

/// Reads every font in a file; errors if it contains no readable font.
pub fn parse_font_file(relative_path: &str, data: Bytes) -> Result<Vec<Font>, String> {
    let fonts: Vec<Font> = Font::iter(data).collect();
    if fonts.is_empty() {
        return Err(format!("{relative_path}: no readable font"));
    }
    Ok(fonts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_fonts_are_available() {
        let set = FontSet::embedded();
        assert!(!set.is_empty());
        assert!(set.book().contains_family("libertinus serif"));
    }

    #[test]
    fn unreadable_font_is_an_error() {
        let err = parse_font_file("fonts/bad.ttf", Bytes::new(b"not a font".to_vec())).unwrap_err();
        assert!(err.contains("fonts/bad.ttf"));
    }

    #[test]
    fn font_files_are_recognised_by_extension_under_fonts() {
        assert!(is_font_file("fonts/A.TTF"));
        assert!(is_font_file("fonts/sub/b.otf"));
        assert!(!is_font_file("assets/a.ttf"));
        assert!(!is_font_file("fonts/readme.txt"));
    }
}
