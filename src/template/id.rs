//! Identifiant de template = nom de son dossier dans le volume.

use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

/// Format d'un identifiant de template (exposé tel quel dans l'OpenAPI).
pub const TEMPLATE_ID_PATTERN: &str = "^[a-z0-9][a-z0-9_-]{0,63}$";

static TEMPLATE_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(TEMPLATE_ID_PATTERN).expect("valid pattern"));

/// Identifiant validé : ne peut contenir ni `/`, ni `.`, ni majuscule, d'où l'absence de
/// traversée de chemin possible.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct TemplateId(String);

#[derive(Debug, thiserror::Error)]
#[error("invalid template id")]
pub struct InvalidTemplateId;

impl TemplateId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for TemplateId {
    type Err = InvalidTemplateId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if TEMPLATE_ID_RE.is_match(s) {
            Ok(Self(s.to_owned()))
        } else {
            Err(InvalidTemplateId)
        }
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for TemplateId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_ids() {
        for id in ["sample", "a", "a-b_c9", &"a".repeat(64)] {
            assert!(id.parse::<TemplateId>().is_ok(), "{id}");
        }
    }

    #[test]
    fn rejects_invalid_ids() {
        for id in ["", "-a", "A", "../x", "a/b", "a.b", "%2e", &"a".repeat(65)] {
            assert!(id.parse::<TemplateId>().is_err(), "{id}");
        }
    }

    #[test]
    fn rejects_trailing_newline() {
        assert!("sample\n".parse::<TemplateId>().is_err());
    }
}
