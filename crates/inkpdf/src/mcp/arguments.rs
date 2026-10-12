//! Faults in a tool call's arguments themselves (unknown key, missing or wrongly typed
//! argument), reported as a tool error the assistant can see and fix (FR-014 of feature 005).
//!
//! rmcp refuses such a call before the tool runs, with serde's message, which does not always
//! name the argument (`invalid type: sequence, expected a map`). The faults are worked out
//! instead from the tool's input schema and an outline of the arguments.

use rmcp::model::{CallToolResult, ContentBlock, JsonObject};
use serde_json::{Map, Value};

/// Prefix of rmcp's message when it refuses a call's arguments.
const RMCP_PREFIX: &str = "failed to deserialize parameters: ";

/// Closing hint of the tool error.
const HINT: &str = "Fix the arguments and call the tool again.";

/// The arguments of a call with each value replaced by an empty value of the same JSON type:
/// enough to explain a refusal, without keeping a copy of the input (rmcp consumes the
/// arguments).
pub(super) fn outline(arguments: Option<&JsonObject>) -> JsonObject {
    let empty = |value: &Value| match value {
        Value::Object(_) => Value::Object(Map::new()),
        Value::Array(_) => Value::Array(Vec::new()),
        Value::String(_) => Value::String(String::new()),
        // Numbers keep their value: whether one is an integer matters.
        other => other.clone(),
    };
    arguments
        .into_iter()
        .flatten()
        .map(|(name, value)| (name.clone(), empty(value)))
        .collect()
}

/// rmcp's refusal of `tool`'s arguments as a tool error: one line per faulty argument, then
/// the hint. When the input schema explains nothing, rmcp's message is kept.
pub(super) fn error_result(
    tool: &str,
    input_schema: Option<&JsonObject>,
    outline: &JsonObject,
    refusal: &CallToolResult,
) -> CallToolResult {
    let mut faults = input_schema
        .map(|schema| faults(tool, schema, outline))
        .unwrap_or_default();
    if faults.is_empty() {
        let message = refusal
            .content
            .iter()
            .find_map(|block| block.as_text())
            .map(|text| text.text.as_str())
            .unwrap_or("invalid arguments");
        faults.push(
            message
                .strip_prefix(RMCP_PREFIX)
                .unwrap_or(message)
                .to_owned(),
        );
    }

    let mut text = format!("Invalid arguments for {tool}");
    for fault in faults {
        text.push_str("\n- ");
        text.push_str(&fault);
    }
    text.push('\n');
    text.push_str(HINT);
    CallToolResult::error(vec![ContentBlock::text(text)])
}

/// Every fault of the arguments against the tool's input schema: unknown arguments, missing
/// required ones, then arguments whose JSON type is not the declared one.
fn faults(tool: &str, schema: &JsonObject, arguments: &JsonObject) -> Vec<String> {
    let empty = Map::new();
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let mut faults = Vec::new();

    let mut known: Vec<&str> = properties.keys().map(String::as_str).collect();
    known.sort_unstable();
    let takes = match known.as_slice() {
        [] => format!("{tool} takes no arguments"),
        known => format!("{tool} takes {}", known.join(", ")),
    };
    for name in arguments
        .keys()
        .filter(|name| !properties.contains_key(*name))
    {
        faults.push(format!("unknown argument `{name}`; {takes}"));
    }

    let mut required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|name| !arguments.contains_key(*name))
        .collect();
    required.sort_unstable();
    for name in required {
        faults.push(format!("missing argument `{name}`"));
    }

    for (name, value) in arguments {
        let declared: Vec<&str> = match properties.get(name).and_then(|p| p.get("type")) {
            Some(Value::String(declared)) => vec![declared.as_str()],
            Some(Value::Array(declared)) => declared.iter().filter_map(Value::as_str).collect(),
            _ => continue,
        };
        if !declared.iter().any(|declared| has_type(value, declared)) {
            let expected: Vec<&str> = declared.iter().map(|d| type_phrase(d)).collect();
            faults.push(format!(
                "argument `{name}` must be {}, not {}",
                expected.join(" or "),
                value_phrase(value)
            ));
        }
    }
    faults
}

/// Whether `value` is of the JSON Schema type `declared`; an unknown type name matches.
fn has_type(value: &Value, declared: &str) -> bool {
    match declared {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.is_i64() || value.is_u64(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    }
}

/// A JSON Schema type name, with its article.
fn type_phrase(declared: &str) -> &str {
    match declared {
        "object" => "an object",
        "array" => "an array",
        "string" => "a string",
        "number" => "a number",
        "integer" => "an integer",
        "boolean" => "a boolean",
        other => other,
    }
}

/// The JSON type of `value`, with its article; the value itself is never quoted.
fn value_phrase(value: &Value) -> &'static str {
    match value {
        Value::Object(_) => "an object",
        Value::Array(_) => "an array",
        Value::String(_) => "a string",
        Value::Number(_) => "a number",
        Value::Bool(_) => "a boolean",
        Value::Null => "null",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn object(value: Value) -> JsonObject {
        value.as_object().unwrap().clone()
    }

    fn text(result: &CallToolResult) -> &str {
        &result.content[0].as_text().expect("a text block").text
    }

    #[test]
    fn outline_keeps_the_types_but_not_the_values() {
        let arguments = object(json!({
            "data": { "title": "Secret" },
            "items": [1, 2],
            "templateId": "sample",
            "count": 3,
            "ratio": 1.5,
            "flag": true,
            "none": null
        }));
        assert_eq!(
            Value::Object(outline(Some(&arguments))),
            json!({
                "data": {},
                "items": [],
                "templateId": "",
                "count": 3,
                "ratio": 1.5,
                "flag": true,
                "none": null
            })
        );
        assert!(outline(None).is_empty());
    }

    #[test]
    fn declared_types_are_checked() {
        let schema = object(json!({
            "type": "object",
            "properties": {
                "count": { "type": "integer" },
                "version": { "type": ["string", "null"] },
                "any": {}
            }
        }));
        let faults = |arguments: Value| faults("tool", &schema, &object(arguments));

        assert!(faults(json!({ "count": 2, "version": null, "any": [] })).is_empty());
        assert!(faults(json!({ "version": "1" })).is_empty());
        assert_eq!(
            faults(json!({ "count": 2.5, "version": 1 })),
            [
                "argument `count` must be an integer, not a number",
                "argument `version` must be a string or null, not a number",
            ]
        );
    }

    #[test]
    fn without_explained_fault_rmcp_message_is_kept() {
        let refusal = CallToolResult::error(vec![ContentBlock::text(
            "failed to deserialize parameters: something odd",
        )]);
        let schema = object(json!({ "type": "object", "properties": {} }));

        let result = error_result("tool", Some(&schema), &JsonObject::new(), &refusal);

        assert_eq!(result.is_error, Some(true));
        assert!(result.structured_content.is_none());
        assert_eq!(
            text(&result),
            "Invalid arguments for tool\n- something odd\n\
             Fix the arguments and call the tool again."
        );
    }
}
