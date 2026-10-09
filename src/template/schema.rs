//! Schéma `schema.json` d'un template : chargement, défauts de `design`, validation.

use jsonschema::{Draft, Validator};
use serde_json::{Map, Value};
use typst::foundations::Bytes;

use crate::error::Violation;

/// Schéma compilé d'un template.
pub struct TemplateSchema {
    /// Octets d'origine du fichier, exposés tels quels (constitution VI).
    raw: Bytes,
    /// Schéma tel qu'écrit par l'auteur (sans l'`additionalProperties` ajouté).
    value: Value,
    validator: Validator,
}

impl std::fmt::Debug for TemplateSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TemplateSchema")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

impl TemplateSchema {
    /// Charge et compile un schéma ; l'erreur est destinée à l'auteur du template.
    pub fn load(raw: Bytes) -> Result<Self, String> {
        let value: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        let root = value
            .as_object()
            .ok_or("the root of the schema must be an object")?;

        if root.get("type").and_then(Value::as_str) != Some("object") {
            return Err("the root of the schema must declare `\"type\": \"object\"`".into());
        }
        let properties = root
            .get("properties")
            .and_then(Value::as_object)
            .ok_or("the root of the schema must declare `properties.data`")?;
        if !properties.contains_key("data") {
            return Err("the root of the schema must declare `properties.data`".into());
        }
        if let Some(other) = properties.keys().find(|k| *k != "data" && *k != "design") {
            return Err(format!(
                "only `data` and `design` may be declared at the root of the schema, found `{other}`"
            ));
        }
        if let Some(reference) = find_external_ref(&value) {
            return Err(format!(
                "only internal `$ref` (starting with `#`) are supported, found `{reference}`"
            ));
        }

        let mut compiled = value.clone();
        if let Some(root) = compiled.as_object_mut() {
            root.entry("additionalProperties")
                .or_insert(Value::Bool(false));
        }
        let validator = jsonschema::options()
            .with_draft(Draft::Draft202012)
            .build(&compiled)
            .map_err(|e| format!("invalid JSON Schema: {e}"))?;

        Ok(Self {
            raw,
            value,
            validator,
        })
    }

    /// Octets d'origine de `schema.json`.
    pub fn raw(&self) -> &Bytes {
        &self.raw
    }

    /// Schéma tel qu'écrit par l'auteur.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Applique les défauts de `design` puis valide le corps.
    ///
    /// Le corps doit être un objet JSON (vérifié par l'appelant). Toutes les violations sont
    /// renvoyées.
    pub fn prepare(&self, mut body: Value) -> Result<Value, Vec<Violation>> {
        // `design` n'est initialisé que si le schéma le déclare : sinon la clé ajoutée serait
        // refusée par `additionalProperties: false`.
        if let (Some(root), Some(design_schema)) = (
            body.as_object_mut(),
            self.value.pointer("/properties/design"),
        ) {
            let design = root
                .entry("design")
                .or_insert_with(|| Value::Object(Map::new()));
            if let Some(design) = design.as_object_mut() {
                apply_defaults(design_schema, design);
            }
        }

        let violations: Vec<Violation> = self
            .validator
            .iter_errors(&body)
            .map(|error| Violation {
                path: error.instance_path().to_string(),
                schema_path: error.schema_path().to_string(),
                message: error.to_string(),
            })
            .collect();

        if violations.is_empty() {
            Ok(body)
        } else {
            Err(violations)
        }
    }
}

/// Insère les `default` des propriétés absentes, récursivement sur les sous-objets.
fn apply_defaults(schema: &Value, target: &mut Map<String, Value>) {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return;
    };
    for (key, property) in properties {
        match target.get_mut(key) {
            Some(Value::Object(child)) => apply_defaults(property, child),
            Some(_) => {}
            None => {
                if let Some(default) = property.get("default") {
                    target.insert(key.clone(), default.clone());
                } else if has_defaults(property) {
                    let mut child = Map::new();
                    apply_defaults(property, &mut child);
                    target.insert(key.clone(), Value::Object(child));
                }
            }
        }
    }
}

/// Vrai si une propriété de ce (sous-)schéma, à n'importe quelle profondeur, a un `default`.
fn has_defaults(schema: &Value) -> bool {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| {
            properties
                .values()
                .any(|p| p.get("default").is_some() || has_defaults(p))
        })
}

fn find_external_ref(value: &Value) -> Option<&str> {
    match value {
        Value::Object(map) => map.iter().find_map(|(key, v)| match (key.as_str(), v) {
            ("$ref" | "$dynamicRef", Value::String(r)) if !r.starts_with('#') => Some(r.as_str()),
            _ => find_external_ref(v),
        }),
        Value::Array(items) => items.iter().find_map(find_external_ref),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema(value: Value) -> TemplateSchema {
        TemplateSchema::load(Bytes::new(serde_json::to_vec(&value).unwrap())).unwrap()
    }

    fn sample() -> TemplateSchema {
        schema(json!({
            "type": "object",
            "required": ["data"],
            "properties": {
                "data": {
                    "type": "object",
                    "required": ["title"],
                    "properties": {
                        "title": { "type": "string", "minLength": 1 },
                        "count": { "type": "integer", "default": 3 }
                    }
                },
                "design": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "color": { "type": "string", "default": "#000000" },
                        "align": { "enum": ["left", "right"], "default": "left" },
                        "header": {
                            "type": "object",
                            "properties": {
                                "visible": { "type": "boolean", "default": true },
                                "size": { "type": "integer", "default": 12 }
                            }
                        }
                    }
                }
            }
        }))
    }

    #[test]
    fn load_rejects_bad_roots() {
        let load = |v: Value| TemplateSchema::load(Bytes::new(serde_json::to_vec(&v).unwrap()));
        assert!(TemplateSchema::load(Bytes::new(b"{".to_vec())).is_err());
        assert!(load(json!({"type": "object", "properties": {}})).is_err());
        assert!(
            load(json!({"type": "object", "properties": {"data": {}, "extra": {}}}))
                .unwrap_err()
                .contains("extra")
        );
        let external = load(json!({
            "type": "object",
            "properties": {"data": {"$ref": "https://example.com/s.json"}}
        }));
        assert!(external.unwrap_err().contains("https://example.com"));
        assert!(load(json!({"type": "object", "properties": {"data": {"type": 12}}})).is_err());
    }

    #[test]
    fn load_accepts_internal_refs_and_keeps_raw_bytes() {
        let raw = br##"{"type":"object","properties":{"data":{"$ref":"#/$defs/d"}},"$defs":{"d":{"type":"object"}}}"##;
        let schema = TemplateSchema::load(Bytes::new(raw.to_vec())).unwrap();
        assert_eq!(schema.raw().as_slice(), raw);
        assert!(schema.value().get("additionalProperties").is_none());
    }

    #[test]
    fn missing_design_gets_defaults() {
        let body = sample().prepare(json!({"data": {"title": "T"}})).unwrap();
        assert_eq!(body["design"]["color"], "#000000");
        assert_eq!(body["design"]["align"], "left");
    }

    #[test]
    fn provided_design_values_are_kept() {
        let body = sample()
            .prepare(json!({"data": {"title": "T"}, "design": {"align": "right"}}))
            .unwrap();
        assert_eq!(body["design"]["align"], "right");
        assert_eq!(body["design"]["color"], "#000000");
    }

    #[test]
    fn defaults_are_not_applied_to_data() {
        let body = sample().prepare(json!({"data": {"title": "T"}})).unwrap();
        assert!(body["data"].get("count").is_none());
    }

    #[test]
    fn invalid_body_lists_every_violation() {
        let violations = sample()
            .prepare(json!({"data": {"title": ""}, "design": {"align": "top"}}))
            .unwrap_err();
        let paths: Vec<&str> = violations.iter().map(|v| v.path.as_str()).collect();
        assert!(paths.contains(&"/data/title"), "{paths:?}");
        assert!(paths.contains(&"/design/align"), "{paths:?}");
        let align = violations
            .iter()
            .find(|v| v.path == "/design/align")
            .unwrap();
        assert_eq!(
            align.schema_path,
            "/properties/design/properties/align/enum"
        );
        assert!(!align.message.is_empty());
    }

    #[test]
    fn design_is_not_added_when_the_schema_does_not_declare_it() {
        let schema = schema(json!({"type": "object", "properties": {"data": {}}}));
        let body = schema.prepare(json!({"data": {}})).unwrap();
        assert!(body.get("design").is_none());
    }

    #[test]
    fn unknown_root_key_is_a_violation() {
        let violations = sample()
            .prepare(json!({"data": {"title": "T"}, "extra": 1}))
            .unwrap_err();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].path, "");
    }

    #[test]
    fn defaults_are_applied_recursively_to_design_sub_objects() {
        let body = sample().prepare(json!({"data": {"title": "T"}})).unwrap();
        assert_eq!(body["design"]["header"]["visible"], true);
        assert_eq!(body["design"]["header"]["size"], 12);
    }

    #[test]
    fn partial_sub_object_is_completed_without_overwriting() {
        let body = sample()
            .prepare(json!({"data": {"title": "T"}, "design": {"header": {"visible": false}}}))
            .unwrap();
        assert_eq!(body["design"]["header"]["visible"], false);
        assert_eq!(body["design"]["header"]["size"], 12);
    }
}
