//! 005/US6: the example volume holds the `demo` template only; it guides assistants (a
//! description and a role on every variable), its example renders, and so does every layout
//! setting.

mod common;

use std::fs;
use std::path::PathBuf;

use axum::http::StatusCode;
use common::*;
use serde_json::{Value, json};

fn examples() -> PathBuf {
    repo_root().join("examples")
}

/// A copy of `examples/templates`, as `compose.yaml` mounts it.
fn volume() -> TestVolume {
    let volume = TestVolume::new();
    copy_dir(&examples().join("templates"), volume.path());
    volume
}

/// The `example.json` of `demo`, read from the repository.
fn demo_example() -> Value {
    let path = examples().join("templates/demo/1.0.0/example.json");
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

async fn render(app: &axum::Router, body: &Value) -> Vec<u8> {
    let (status, _, pdf) = post_json(app, "/templates/demo/render", body).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{body}: {}",
        String::from_utf8_lossy(&pdf)
    );
    assert!(pdf.starts_with(b"%PDF"));
    pdf.to_vec()
}

/// PDF text with every whitespace removed.
fn compact_text(pdf: &[u8]) -> String {
    pdf_text(pdf)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

#[tokio::test]
async fn the_volume_holds_only_demo() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, list) = get_json(&app, "/templates").await;
    assert_eq!(status, StatusCode::OK);
    let templates: Vec<(&str, &str)> = list["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t["id"].as_str().unwrap(), t["status"].as_str().unwrap()))
        .collect();
    assert_eq!(templates, [("demo", "valid")], "{list}");
    assert_eq!(list["templates"][0]["version"], "1.0.0");
}

/// Walks every property schema below `schema` (properties, array items, `$defs`, combinators)
/// and records the pointer of each one that lacks a `description` or an `x-role`.
fn missing_guidance(schema: &Value, pointer: &str, out: &mut Vec<String>) {
    let Some(map) = schema.as_object() else {
        return;
    };
    if let Some(properties) = map.get("properties").and_then(Value::as_object) {
        for (name, property) in properties {
            let here = format!("{pointer}/properties/{name}");
            for key in ["description", "x-role"] {
                if property.get(key).is_none() {
                    out.push(format!("{here} has no {key}"));
                }
            }
            missing_guidance(property, &here, out);
        }
    }
    if let Some(items) = map.get("items") {
        missing_guidance(items, &format!("{pointer}/items"), out);
    }
    for keyword in ["$defs", "definitions"] {
        if let Some(defs) = map.get(keyword).and_then(Value::as_object) {
            for (name, def) in defs {
                missing_guidance(def, &format!("{pointer}/{keyword}/{name}"), out);
            }
        }
    }
    for keyword in ["allOf", "anyOf", "oneOf"] {
        for (i, sub) in map
            .get(keyword)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            missing_guidance(sub, &format!("{pointer}/{keyword}/{i}"), out);
        }
    }
}

/// SC-002: every variable of `data` and `layout`, at any depth, has a description and a role.
#[tokio::test]
async fn every_variable_has_a_description_and_a_role() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, detail) = get_json(&app, "/templates/demo").await;
    assert_eq!(status, StatusCode::OK);
    let schema = &detail["schema"];
    let mut missing = Vec::new();
    for part in ["data", "layout"] {
        let sub = &schema["properties"][part];
        assert!(sub.is_object(), "no {part} in {schema}");
        missing_guidance(sub, &format!("/properties/{part}"), &mut missing);
    }
    assert!(missing.is_empty(), "{missing:#?}");
    // The walk reaches the nested object and the array items.
    for pointer in [
        "/properties/data/properties/recipient/properties/email/x-role",
        "/properties/data/properties/lines/items/properties/amount/x-role",
    ] {
        assert!(schema.pointer(pointer).is_some(), "{pointer}");
    }
}

/// SC-003: the detail carries the example, which renders unchanged.
#[tokio::test]
async fn the_example_renders_unchanged() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, detail) = get_json(&app, "/templates/demo").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["example"], demo_example());

    let pdf = render(&app, &detail["example"]).await;
    let text = compact_text(&pdf);
    for expected in [
        "Officesupplies",
        "DEMO-2026-001",
        "October12,2026",
        "JaneDoe",
        "jane.doe@example.com",
        "Notebooks",
        "42.00",
        "Total:122.90EUR",
        "page1/1",
    ] {
        assert!(text.contains(expected), "{expected} missing in:\n{text}");
    }
}

#[tokio::test]
async fn every_layout_setting_renders() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let default = render(&app, &demo_example()).await;

    for layout in [
        json!({ "font": "Libertinus Serif" }),
        json!({ "font": "New Computer Modern" }),
        json!({ "density": "normal" }),
        json!({ "density": "compact" }),
        json!({ "showTotal": true }),
        json!({ "showTotal": false }),
        json!({ "accentColor": "#2e7d32" }),
        json!({
            "accentColor": "#7a1f5c",
            "font": "New Computer Modern",
            "density": "compact",
            "showTotal": false
        }),
    ] {
        let mut body = demo_example();
        body["layout"] = layout.clone();
        let pdf = render(&app, &body).await;
        // The values that are the defaults give the default document; the others change it.
        let is_default = layout == json!({ "font": "Libertinus Serif" })
            || layout == json!({ "density": "normal" })
            || layout == json!({ "showTotal": true });
        assert_eq!(pdf == default, is_default, "{layout}");
        let has_total = compact_text(&pdf).contains("Total:122.90EUR");
        assert_eq!(has_total, layout["showTotal"] != false, "{layout}");
    }
}

/// SC-004: the audit's impossible dates are refused with the path of the field.
#[tokio::test]
async fn impossible_dates_are_refused() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    for date in [
        "2026-02-30",
        "2026-13-45",
        "2025-02-29",
        "2026-04-31",
        "0000-00-00",
    ] {
        let mut body = demo_example();
        body["data"]["date"] = date.into();
        let (status, _, bytes) = post_json(&app, "/templates/demo/render", &body).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{date}");
        let problem = problem(&bytes);
        assert_eq!(problem["code"], "validation-failed", "{date}: {problem}");
        let violations = problem["violations"].as_array().unwrap();
        assert_eq!(violations.len(), 1, "{date}: {problem}");
        assert_eq!(violations[0]["path"], "/data/date", "{date}");
        assert_eq!(
            violations[0]["message"],
            format!("\"{date}\" is not a valid calendar date (YYYY-MM-DD)"),
        );
    }
    // A leap day is a valid date.
    let mut body = demo_example();
    body["data"]["date"] = "2024-02-29".into();
    render(&app, &body).await;
}

/// All `.bru` files of the Bruno collection, recursively.
fn bruno_requests() -> Vec<PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().unwrap() != "environments" {
                    walk(&path, out);
                }
            } else if path.extension().is_some_and(|e| e == "bru")
                && !["folder.bru", "collection.bru"]
                    .contains(&path.file_name().unwrap().to_str().unwrap())
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&examples().join("bruno"), &mut out);
    out.sort();
    out
}

/// Content of a top-level `name { … }` block, without its two-space indentation.
fn bru_block(text: &str, name: &str) -> Option<String> {
    let start = text
        .find(&format!("\n{name} {{\n"))
        .or_else(|| text.starts_with(&format!("{name} {{\n")).then_some(0))?;
    let body = &text[start..];
    let body = &body[body.find('\n').unwrap() + 1..];
    let body = &body[body.find(" {\n").map_or(0, |i| i + 3)..];
    let end = body.find("\n}\n")?;
    Some(
        body[..end]
            .lines()
            .map(|l| l.strip_prefix("  ").unwrap_or(l))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// FR-024: the Bruno collection only targets existing routes, and its bodies are the example
/// requests (no silent drift).
#[test]
fn bruno_collection_matches_the_api_and_the_examples() {
    let openapi: Value =
        serde_json::from_slice(&fs::read(repo_root().join("openapi/openapi.json")).unwrap())
            .unwrap();
    let paths = openapi["paths"].as_object().unwrap();
    let matches = |template: &str, path: &str| {
        let (t, p): (Vec<&str>, Vec<&str>) =
            (template.split('/').collect(), path.split('/').collect());
        t.len() == p.len() && t.iter().zip(&p).all(|(t, p)| t.starts_with('{') || t == p)
    };

    let requests = bruno_requests();
    assert!(requests.len() >= 15, "{} requests", requests.len());
    let mut covered = std::collections::BTreeSet::new();
    for file in &requests {
        let text = fs::read_to_string(file).unwrap();
        let (method, block) = ["get", "post"]
            .iter()
            .find_map(|m| bru_block(&text, m).map(|b| (*m, b)))
            .unwrap_or_else(|| panic!("{}: no request block", file.display()));
        let url = block.lines().find_map(|l| l.strip_prefix("url: ")).unwrap();
        let path = url
            .strip_prefix("{{baseUrl}}")
            .unwrap()
            .split('?')
            .next()
            .unwrap();
        let (template, item) = paths
            .iter()
            .find(|(template, _)| matches(template, path))
            .unwrap_or_else(|| panic!("{}: unknown route {path}", file.display()));
        assert!(
            item.get(method).is_some(),
            "{}: {method} {template}",
            file.display()
        );
        covered.insert(template.clone());

        if let Some(source) = bru_block(&text, "docs").and_then(|d| {
            d.lines()
                .find_map(|l| l.strip_prefix("Source: ").map(str::to_owned))
        }) {
            let body: Value =
                serde_json::from_str(&bru_block(&text, "body:json").unwrap()).unwrap();
            let expected: Value =
                serde_json::from_slice(&fs::read(repo_root().join(&source)).unwrap()).unwrap();
            assert_eq!(body, expected, "{} differs from {source}", file.display());
        }
    }
    let all: std::collections::BTreeSet<String> = paths.keys().cloned().collect();
    let missing: Vec<_> = all.difference(&covered).collect();
    assert!(
        missing.is_empty(),
        "routes without a Bruno request: {missing:?}"
    );
}
