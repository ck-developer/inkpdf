//! Conversion from `serde_json::Value` to a Typst value (see contracts/template-format.md).
//!
//! Strings stay strings: they are never interpreted as Typst code.

use serde_json::Value as Json;
use typst::foundations::{Str, Value};

pub fn json_to_value(json: &Json) -> Value {
    match json {
        Json::Null => Value::None,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Value::Int(i),
            // Decimals and integers outside the `i64` range.
            None => Value::Float(n.as_f64().unwrap_or(f64::NAN)),
        },
        Json::String(s) => Value::Str(s.as_str().into()),
        Json::Array(items) => Value::Array(items.iter().map(json_to_value).collect()),
        Json::Object(map) => Value::Dict(
            map.iter()
                .map(|(k, v)| (Str::from(k.as_str()), json_to_value(v)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use typst::foundations::{Array, Dict};

    #[test]
    fn scalars() {
        assert_eq!(json_to_value(&json!("a")), Value::Str("a".into()));
        assert_eq!(json_to_value(&json!(42)), Value::Int(42));
        assert_eq!(json_to_value(&json!(-1.5)), Value::Float(-1.5));
        assert_eq!(json_to_value(&json!(true)), Value::Bool(true));
        assert_eq!(json_to_value(&json!(null)), Value::None);
    }

    #[test]
    fn integer_out_of_i64_range_becomes_float() {
        let big = json!(u64::MAX);
        assert_eq!(json_to_value(&big), Value::Float(u64::MAX as f64));
    }

    #[test]
    fn arrays_and_nested_objects() {
        let value = json_to_value(&json!({"a": [1, "x"], "b": {"c": false}}));
        let mut inner = Dict::new();
        inner.insert("c".into(), Value::Bool(false));
        let mut expected = Dict::new();
        expected.insert(
            "a".into(),
            Value::Array(Array::from_iter([Value::Int(1), Value::Str("x".into())])),
        );
        expected.insert("b".into(), Value::Dict(inner));
        assert_eq!(value, Value::Dict(expected));
    }

    #[test]
    fn code_like_strings_stay_strings() {
        let value = json_to_value(&json!("#import \"/etc/passwd\""));
        assert_eq!(value, Value::Str("#import \"/etc/passwd\"".into()));
    }
}
