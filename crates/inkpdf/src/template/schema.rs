//! A template's `schema.json`: loading, `layout` defaults, validation.

use jsonschema::error::ValidationErrorKind;
use jsonschema::{Draft, ValidationError, Validator};
use serde_json::{Map, Value};
use typst::foundations::Bytes;

use crate::error::Violation;

/// Compiled schema of a template.
pub struct TemplateSchema {
    /// Original bytes of the file, exposed as is (constitution VI).
    raw: Bytes,
    /// Schema as written by the author (without the added `additionalProperties`).
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
    /// Loads and compiles a schema; the error is meant for the template author.
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
        if let Some(other) = properties.keys().find(|k| *k != "data" && *k != "layout") {
            return Err(format!(
                "only `data` and `layout` may be declared at the root of the schema, found `{other}`"
            ));
        }
        if let Some(reference) = find_external_ref(&value) {
            return Err(format!(
                "only internal `$ref` (starting with `#`) are supported, found `{reference}`"
            ));
        }
        check_roles(&value, &mut String::new())?;

        let mut compiled = value.clone();
        if let Some(root) = compiled.as_object_mut() {
            root.entry("additionalProperties")
                .or_insert(Value::Bool(false));
        }
        // Standard formats are asserted (`date`, `date-time`, `email`, …); unknown format names
        // stay ignored (research R2 of feature 005).
        let validator = jsonschema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .build(&compiled)
            .map_err(|e| format!("invalid JSON Schema: {e}"))?;

        Ok(Self {
            raw,
            value,
            validator,
        })
    }

    /// Original bytes of `schema.json`.
    pub fn raw(&self) -> &Bytes {
        &self.raw
    }

    /// Schema as written by the author.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Applies the `layout` defaults, then validates the body.
    ///
    /// The body must be a JSON object (checked by the caller). All violations are returned.
    pub fn prepare(&self, mut body: Value) -> Result<Value, Vec<Violation>> {
        // `layout` is initialized only if the schema declares it: otherwise the added key would
        // be rejected by `additionalProperties: false`.
        if let (Some(root), Some(layout_schema)) = (
            body.as_object_mut(),
            self.value.pointer("/properties/layout"),
        ) {
            let layout = root
                .entry("layout")
                .or_insert_with(|| Value::Object(Map::new()));
            if let Some(layout) = layout.as_object_mut() {
                apply_defaults(layout_schema, layout);
            }
        }

        let violations: Vec<Violation> = self
            .validator
            .iter_errors(&body)
            .map(|error| Violation {
                path: error.instance_path().to_string(),
                schema_path: error.schema_path().to_string(),
                message: message(&error),
            })
            .collect();

        if violations.is_empty() {
            Ok(body)
        } else {
            Err(violations)
        }
    }
}

/// The validator's message, except for `format: date`, whose message says what is expected
/// (`"2026-02-30" is not a valid calendar date (YYYY-MM-DD)`).
fn message(error: &ValidationError<'_>) -> String {
    match error.kind() {
        ValidationErrorKind::Format { format } if format == "date" => format!(
            "{} is not a valid calendar date (YYYY-MM-DD)",
            error.instance()
        ),
        _ => error.to_string(),
    }
}

/// Inserts the `default` of missing properties, recursively into sub-objects.
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

/// True if a property of this (sub-)schema, at any depth, has a `default`.
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

/// Values of the `x-role` annotation (research R1).
const ROLES: [&str; 3] = ["ask", "fill", "default"];

/// Keywords whose value is a map of sub-schemas.
const SCHEMA_MAPS: [&str; 5] = [
    "properties",
    "patternProperties",
    "$defs",
    "definitions",
    "dependentSchemas",
];

/// Keywords whose value is one sub-schema (`items` may also be an array, before 2020-12).
const SCHEMA_VALUES: [&str; 11] = [
    "items",
    "additionalProperties",
    "unevaluatedProperties",
    "unevaluatedItems",
    "contains",
    "propertyNames",
    "if",
    "then",
    "else",
    "not",
    "additionalItems",
];

/// Keywords whose value is an array of sub-schemas.
const SCHEMA_ARRAYS: [&str; 4] = ["prefixItems", "allOf", "anyOf", "oneOf"];

/// Checks every `x-role` of a (sub-)schema, `pointer` being its JSON pointer in the schema.
///
/// Only the keywords that hold sub-schemas are followed, so that an `x-role` key inside a
/// `default`, `const`, `enum` or `examples` value, or a property named `x-role`, is not
/// mistaken for a role. Reports the first invalid role.
fn check_roles(schema: &Value, pointer: &mut String) -> Result<(), String> {
    let Some(map) = schema.as_object() else {
        // `true`, `false`: no annotation.
        return Ok(());
    };
    if let Some(role) = map.get("x-role")
        && !role.as_str().is_some_and(|role| ROLES.contains(&role))
    {
        return Err(format!(
            "invalid role {role} at {pointer}/x-role; expected \"ask\", \"fill\" or \"default\""
        ));
    }

    let visit = |pointer: &mut String, tokens: &[&str], schema: &Value| {
        let length = pointer.len();
        for token in tokens {
            pointer.push('/');
            pointer.push_str(&token.replace('~', "~0").replace('/', "~1"));
        }
        let outcome = check_roles(schema, pointer);
        pointer.truncate(length);
        outcome
    };
    for (keyword, value) in map {
        let keyword = keyword.as_str();
        match value {
            Value::Object(schemas) if SCHEMA_MAPS.contains(&keyword) => {
                for (name, schema) in schemas {
                    visit(pointer, &[keyword, name], schema)?;
                }
            }
            Value::Array(schemas) if SCHEMA_ARRAYS.contains(&keyword) || keyword == "items" => {
                for (index, schema) in schemas.iter().enumerate() {
                    visit(pointer, &[keyword, &index.to_string()], schema)?;
                }
            }
            _ if SCHEMA_VALUES.contains(&keyword) => visit(pointer, &[keyword], value)?,
            _ => {}
        }
    }
    Ok(())
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
                "layout": {
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
    fn missing_layout_gets_defaults() {
        let body = sample().prepare(json!({"data": {"title": "T"}})).unwrap();
        assert_eq!(body["layout"]["color"], "#000000");
        assert_eq!(body["layout"]["align"], "left");
    }

    #[test]
    fn provided_layout_values_are_kept() {
        let body = sample()
            .prepare(json!({"data": {"title": "T"}, "layout": {"align": "right"}}))
            .unwrap();
        assert_eq!(body["layout"]["align"], "right");
        assert_eq!(body["layout"]["color"], "#000000");
    }

    #[test]
    fn defaults_are_not_applied_to_data() {
        let body = sample().prepare(json!({"data": {"title": "T"}})).unwrap();
        assert!(body["data"].get("count").is_none());
    }

    #[test]
    fn invalid_body_lists_every_violation() {
        let violations = sample()
            .prepare(json!({"data": {"title": ""}, "layout": {"align": "top"}}))
            .unwrap_err();
        let paths: Vec<&str> = violations.iter().map(|v| v.path.as_str()).collect();
        assert!(paths.contains(&"/data/title"), "{paths:?}");
        assert!(paths.contains(&"/layout/align"), "{paths:?}");
        let align = violations
            .iter()
            .find(|v| v.path == "/layout/align")
            .unwrap();
        assert_eq!(
            align.schema_path,
            "/properties/layout/properties/align/enum"
        );
        assert!(!align.message.is_empty());
    }

    #[test]
    fn layout_is_not_added_when_the_schema_does_not_declare_it() {
        let schema = schema(json!({"type": "object", "properties": {"data": {}}}));
        let body = schema.prepare(json!({"data": {}})).unwrap();
        assert!(body.get("layout").is_none());
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
    fn defaults_are_applied_recursively_to_layout_sub_objects() {
        let body = sample().prepare(json!({"data": {"title": "T"}})).unwrap();
        assert_eq!(body["layout"]["header"]["visible"], true);
        assert_eq!(body["layout"]["header"]["size"], 12);
    }

    #[test]
    fn partial_sub_object_is_completed_without_overwriting() {
        let body = sample()
            .prepare(json!({"data": {"title": "T"}, "layout": {"header": {"visible": false}}}))
            .unwrap();
        assert_eq!(body["layout"]["header"]["visible"], false);
        assert_eq!(body["layout"]["header"]["size"], 12);
    }

    /// A schema with a role at every place one is allowed: `data`, `layout`, nested
    /// `properties`, array `items`, `$defs` and `oneOf`.
    fn with_roles() -> Value {
        json!({
            "type": "object",
            "required": ["data"],
            "$defs": {
                "amount": { "type": "string", "pattern": "^[0-9]+$", "x-role": "ask" }
            },
            "properties": {
                "data": {
                    "type": "object",
                    "required": ["title", "lines"],
                    "properties": {
                        "title": { "type": "string", "minLength": 1, "x-role": "ask" },
                        "date": { "type": "string", "x-role": "fill" },
                        "recipient": {
                            "type": "object",
                            "x-role": "ask",
                            "properties": {
                                "name": { "type": "string", "x-role": "ask" }
                            }
                        },
                        "lines": {
                            "type": "array",
                            "x-role": "ask",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "position": { "type": "integer", "x-role": "fill" },
                                    "amount": { "$ref": "#/$defs/amount" }
                                }
                            }
                        },
                        "status": {
                            "x-role": "default",
                            "oneOf": [
                                { "const": "draft", "x-role": "default" },
                                { "const": "final", "x-role": "default" }
                            ]
                        }
                    }
                },
                "layout": {
                    "type": "object",
                    "properties": {
                        "color": { "type": "string", "default": "#000000", "x-role": "default" }
                    }
                }
            }
        })
    }

    fn load(value: &Value) -> Result<TemplateSchema, String> {
        TemplateSchema::load(Bytes::new(serde_json::to_vec(value).unwrap()))
    }

    #[test]
    fn valid_roles_are_accepted_everywhere() {
        let schema = load(&with_roles()).unwrap();
        assert_eq!(schema.value(), &with_roles());
    }

    #[test]
    fn invalid_roles_are_refused_with_their_pointer() {
        let cases = [
            (
                "/properties/data/properties/title",
                json!("maybe"),
                "\"maybe\"",
            ),
            (
                "/properties/data/properties/recipient/properties/name",
                json!(""),
                "\"\"",
            ),
            (
                "/properties/data/properties/lines/items/properties/position",
                json!(1),
                "1",
            ),
            ("/$defs/amount", json!("maybe"), "\"maybe\""),
            (
                "/properties/data/properties/status/oneOf/1",
                json!("Ask"),
                "\"Ask\"",
            ),
            ("/properties/layout/properties/color", json!(null), "null"),
        ];
        for (at, role, shown) in cases {
            let mut value = with_roles();
            value.pointer_mut(at).unwrap()["x-role"] = role;
            let error = load(&value).unwrap_err();
            assert_eq!(
                error,
                format!(
                    "invalid role {shown} at {at}/x-role; expected \"ask\", \"fill\" or \"default\""
                )
            );
        }
    }

    #[test]
    fn role_pointer_escapes_property_names() {
        let value = json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "object",
                    "properties": { "a/b~c": { "type": "string", "x-role": "maybe" } }
                }
            }
        });
        assert_eq!(
            load(&value).unwrap_err(),
            "invalid role \"maybe\" at /properties/data/properties/a~1b~0c/x-role; \
             expected \"ask\", \"fill\" or \"default\""
        );
    }

    /// `x-role` is only a keyword where a schema is expected: inside a `default`, a `const` or
    /// an `enum`, and as a property name, it is data.
    #[test]
    fn x_role_outside_a_schema_is_not_a_role() {
        let value = json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "object",
                    "properties": {
                        "x-role": { "type": "string", "x-role": "ask" },
                        "options": {
                            "type": "object",
                            "default": { "x-role": "maybe" },
                            "examples": [{ "x-role": "maybe" }]
                        },
                        "fixed": { "const": { "x-role": 1 } },
                        "choice": { "enum": [{ "x-role": "" }] }
                    }
                }
            }
        });
        assert!(load(&value).is_ok());
    }

    #[test]
    fn roles_do_not_change_what_input_is_accepted() {
        fn strip_roles(value: &mut Value) {
            match value {
                Value::Object(map) => {
                    map.remove("x-role");
                    map.values_mut().for_each(strip_roles);
                }
                Value::Array(items) => items.iter_mut().for_each(strip_roles),
                _ => {}
            }
        }
        let mut without = with_roles();
        strip_roles(&mut without);
        let (with, without) = (load(&with_roles()).unwrap(), load(&without).unwrap());

        let line = json!({ "position": 1, "amount": "12" });
        let inputs = [
            json!({ "data": { "title": "T", "lines": [line] } }),
            json!({ "data": { "title": "T", "lines": [line], "status": "final" } }),
            json!({ "data": { "title": "", "lines": [{ "position": "1", "amount": "x" }] } }),
            json!({ "data": { "title": "T", "lines": [line], "status": "other" } }),
            json!({ "data": { "lines": [] }, "layout": { "color": 1 } }),
        ];
        let outcome = |schema: &TemplateSchema, input: &Value| {
            schema.prepare(input.clone()).map_err(|violations| {
                violations
                    .into_iter()
                    .map(|v| (v.path, v.schema_path, v.message))
                    .collect::<Vec<_>>()
            })
        };
        for input in inputs {
            assert_eq!(outcome(&with, &input), outcome(&without, &input), "{input}");
        }
    }

    /// A `format: date` property at the root of `data`, in a nested object, in array `items`
    /// and through `$defs`, next to other formats.
    fn dated() -> TemplateSchema {
        schema(json!({
            "type": "object",
            "$defs": {
                "day": { "type": "string", "format": "date" }
            },
            "properties": {
                "data": {
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "format": "date" },
                        "period": {
                            "type": "object",
                            "properties": {
                                "start": { "type": "string", "format": "date" }
                            }
                        },
                        "lines": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "on": { "type": "string", "format": "date" }
                                }
                            }
                        },
                        "due": { "$ref": "#/$defs/day" },
                        "sentAt": { "type": "string", "format": "date-time" },
                        "email": { "type": "string", "format": "email" },
                        "code": { "type": "string", "format": "made-up" },
                        "count": { "type": "integer" }
                    }
                }
            }
        }))
    }

    /// Violations of `data` as `(path, message)`.
    fn data_violations(data: Value) -> Vec<(String, String)> {
        match dated().prepare(json!({ "data": data })) {
            Ok(_) => Vec::new(),
            Err(violations) => violations
                .into_iter()
                .map(|v| (v.path, v.message))
                .collect(),
        }
    }

    #[test]
    fn impossible_dates_are_refused_at_every_depth() {
        let impossible = [
            "2026-02-30",
            "2026-13-45",
            "2025-02-29",
            "2026-04-31",
            "0000-00-00",
        ];
        for date in impossible {
            let message = format!("\"{date}\" is not a valid calendar date (YYYY-MM-DD)");
            let cases = [
                ("/data/date", json!({ "date": date })),
                ("/data/period/start", json!({ "period": { "start": date } })),
                ("/data/lines/1/on", json!({ "lines": [{}, { "on": date }] })),
                ("/data/due", json!({ "due": date })),
            ];
            for (path, data) in cases {
                assert_eq!(
                    data_violations(data),
                    [(path.to_owned(), message.clone())],
                    "{date} at {path}"
                );
            }
        }
    }

    #[test]
    fn real_dates_are_accepted() {
        let data = json!({
            "date": "2024-02-29",
            "period": { "start": "2026-12-31" },
            "lines": [{ "on": "2026-01-01" }],
            "due": "2000-02-29"
        });
        assert!(data_violations(data).is_empty());
    }

    #[test]
    fn other_standard_formats_are_asserted() {
        assert!(data_violations(json!({ "sentAt": "2024-02-29T10:00:00Z" })).is_empty());
        let violations = data_violations(json!({ "sentAt": "2026-02-30T10:00:00Z" }));
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert_eq!(violations[0].0, "/data/sentAt");
        // Other formats keep the validator's message.
        assert!(violations[0].1.contains("date-time"), "{}", violations[0].1);

        assert!(data_violations(json!({ "email": "someone@example.com" })).is_empty());
        let violations = data_violations(json!({ "email": "not-an-email" }));
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert_eq!(violations[0].0, "/data/email");
        assert!(violations[0].1.contains("email"), "{}", violations[0].1);
    }

    #[test]
    fn unknown_formats_are_ignored() {
        assert!(data_violations(json!({ "code": "anything at all" })).is_empty());
    }

    #[test]
    fn date_faults_come_back_with_every_other_fault() {
        let violations = data_violations(json!({
            "date": "2026-02-30",
            "lines": [{ "on": "2026-13-45" }],
            "count": "three"
        }));
        let paths: Vec<&str> = violations.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(violations.len(), 3, "{violations:?}");
        for path in ["/data/date", "/data/lines/0/on", "/data/count"] {
            assert!(paths.contains(&path), "{path}: {violations:?}");
        }
    }
}
