//! Métadonnées du PDF (specs/002-typst-packages, R17).
//!
//! La section facultative `metadata` du corps est validée par un schéma fixe, puis appliquée au
//! document compilé (`PagedDocument::info_mut`) : jamais injectée comme code Typst.
//! Priorité : requête, puis `set document(...)` du template, puis valeurs par défaut.

use std::sync::LazyLock;

use jsonschema::{Draft, Validator};
use serde_json::{Map, Value, json};
use typst::foundations::{Datetime, Smart};
use typst::model::DocumentInfo;

use crate::error::Violation;

/// Clé de la section dans le corps de génération.
pub const KEY: &str = "metadata";

/// Métadonnées demandées par l'appelant ; tout est facultatif.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub subject: Option<String>,
    pub keywords: Vec<String>,
    /// Date du document (année, mois, jour).
    pub date: Option<(i32, u8, u8)>,
}

/// Valeurs du service, utilisées quand ni la requête ni le template ne renseignent un champ.
pub struct Defaults<'a> {
    /// Nom du template (`template.json`).
    pub title: &'a str,
    pub author: &'a str,
}

/// Schéma de la section `metadata`, exposé tel quel dans l'OpenAPI.
pub fn schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "title": { "type": "string", "minLength": 1, "maxLength": 500 },
            "author": {
                "oneOf": [
                    { "type": "string", "minLength": 1, "maxLength": 200 },
                    {
                        "type": "array", "minItems": 1, "maxItems": 20,
                        "items": { "type": "string", "minLength": 1, "maxLength": 200 }
                    }
                ]
            },
            "subject": { "type": "string", "maxLength": 2000 },
            "keywords": {
                "type": "array", "maxItems": 50,
                "items": { "type": "string", "minLength": 1, "maxLength": 100 }
            },
            "date": { "type": "string", "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}$" }
        }
    })
}

static VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    jsonschema::options()
        .with_draft(Draft::Draft202012)
        .build(&schema())
        .expect("valid metadata schema")
});

/// Retire `metadata` du corps et la valide. Les chemins des violations commencent par
/// `/metadata`.
pub fn extract(body: &mut Map<String, Value>) -> Result<DocumentMetadata, Vec<Violation>> {
    let Some(value) = body.remove(KEY) else {
        return Ok(DocumentMetadata::default());
    };
    let violation = |path: String, schema_path: String, message: String| Violation {
        path: format!("/{KEY}{path}"),
        schema_path,
        message,
    };
    let mut violations: Vec<Violation> = VALIDATOR
        .iter_errors(&value)
        .map(|e| {
            violation(
                e.instance_path().to_string(),
                e.schema_path().to_string(),
                e.to_string(),
            )
        })
        .collect();

    let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
    let list = |key: &str| match value.get(key) {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    };

    let date = text("date").and_then(|raw| {
        let parts: Vec<&str> = raw.split('-').collect();
        let parsed = match parts[..] {
            [y, m, d] => match (y.parse(), m.parse(), d.parse()) {
                (Ok(y), Ok(m), Ok(d)) => Datetime::from_ymd(y, m, d).map(|_| (y, m, d)),
                _ => None,
            },
            _ => None,
        };
        if parsed.is_none() && violations.iter().all(|v| v.path != "/metadata/date") {
            violations.push(violation(
                "/date".into(),
                "/properties/date".into(),
                format!("\"{raw}\" is not a valid calendar date"),
            ));
        }
        parsed
    });

    if !violations.is_empty() {
        return Err(violations);
    }
    Ok(DocumentMetadata {
        title: text("title"),
        authors: list("author"),
        subject: text("subject"),
        keywords: list("keywords"),
        date,
    })
}

/// Applique les métadonnées au document compilé.
pub fn apply(info: &mut DocumentInfo, metadata: &DocumentMetadata, defaults: &Defaults) {
    if let Some(title) = &metadata.title {
        info.title = Some(title.as_str().into());
    } else if info.title.is_none() {
        info.title = Some(defaults.title.into());
    }

    if !metadata.authors.is_empty() {
        info.author = metadata.authors.iter().map(|a| a.as_str().into()).collect();
    } else if info.author.is_empty() && !defaults.author.is_empty() {
        info.author = vec![defaults.author.into()];
    }

    if let Some(subject) = &metadata.subject {
        info.description = Some(subject.as_str().into());
    }
    if !metadata.keywords.is_empty() {
        info.keywords = metadata.keywords.iter().map(|k| k.as_str().into()).collect();
    }
    if let Some((y, m, d)) = metadata.date {
        info.date = Smart::Custom(Datetime::from_ymd(y, m, d));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(metadata: Value) -> Map<String, Value> {
        let mut body = Map::new();
        body.insert("data".into(), json!({}));
        body.insert(KEY.into(), metadata);
        body
    }

    #[test]
    fn absent_metadata_is_empty() {
        let mut body = Map::new();
        assert_eq!(extract(&mut body).unwrap(), DocumentMetadata::default());
    }

    #[test]
    fn metadata_is_removed_from_the_body_and_parsed() {
        let mut body = body(json!({
            "title": "T", "author": "A", "subject": "S", "keywords": ["k"], "date": "2026-02-28"
        }));
        let metadata = extract(&mut body).unwrap();
        assert!(!body.contains_key(KEY));
        assert_eq!(metadata.title.as_deref(), Some("T"));
        assert_eq!(metadata.authors, ["A"]);
        assert_eq!(metadata.keywords, ["k"]);
        assert_eq!(metadata.date, Some((2026, 2, 28)));
    }

    #[test]
    fn invalid_metadata_reports_prefixed_paths() {
        let mut body = body(json!({ "title": "", "author": [], "date": "2026-02-30", "x": 1 }));
        let paths: Vec<String> = extract(&mut body)
            .unwrap_err()
            .into_iter()
            .map(|v| v.path)
            .collect();
        assert!(paths.contains(&"/metadata/title".into()), "{paths:?}");
        assert!(paths.contains(&"/metadata/author".into()), "{paths:?}");
        assert!(paths.contains(&"/metadata".into()), "{paths:?}");
        assert!(paths.contains(&"/metadata/date".into()), "{paths:?}");
    }

    #[test]
    fn precedence_is_request_then_template_then_defaults() {
        let defaults = Defaults { title: "Nom", author: "inkpdf" };

        let mut info = DocumentInfo::default();
        apply(&mut info, &DocumentMetadata::default(), &defaults);
        assert_eq!(info.title.as_deref(), Some("Nom"));
        assert_eq!(info.author, ["inkpdf"]);

        let mut info = DocumentInfo {
            title: Some("Du template".into()),
            author: vec!["Auteur du template".into()],
            ..DocumentInfo::default()
        };
        apply(&mut info, &DocumentMetadata::default(), &defaults);
        assert_eq!(info.title.as_deref(), Some("Du template"));
        assert_eq!(info.author, ["Auteur du template"]);

        let request = DocumentMetadata {
            title: Some("Requête".into()),
            authors: vec!["ACME".into()],
            ..DocumentMetadata::default()
        };
        apply(&mut info, &request, &defaults);
        assert_eq!(info.title.as_deref(), Some("Requête"));
        assert_eq!(info.author, ["ACME"]);
    }
}
