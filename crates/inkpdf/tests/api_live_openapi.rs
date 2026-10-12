//! 003/US1b: the OpenAPI document is built at request time from the loaded templates.

mod common;

use std::time::{Duration, Instant};

use axum::Router;
use common::*;
use inkpdf::registry::watcher::{self, WatcherHandle};
use inkpdf::{Config, build_app};
use serde_json::Value;

fn examples_volume() -> TestVolume {
    let volume = TestVolume::new();
    for id in ["sample", "packages-demo"] {
        volume.copy_template(&fixture(id), id);
    }
    volume.copy_template(&repo_root().join("examples/templates/demo"), "demo");
    volume.copy_template(&fixture("invalid-schema"), "invalid-schema");
    // A template whose `data` uses `$defs`.
    let schema = serde_json::json!({
        "type": "object",
        "required": ["data"],
        "properties": {
            "data": {
                "type": "object",
                "properties": { "total": { "$ref": "#/$defs/amount" } }
            }
        },
        "$defs": { "amount": { "type": "string", "pattern": "^[0-9]+$" } }
    });
    volume
        .write_file("with-defs/main.typ", "= Definitions")
        .write_file("with-defs/schema.json", schema.to_string());
    volume
}

async fn document(app: &Router) -> Value {
    get_json(app, "/openapi.json").await.1
}

/// Every `$ref` string found in `value`.
fn refs(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "$ref"
                    && let Some(r) = child.as_str()
                {
                    out.push(r.to_owned());
                }
                refs(child, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| refs(i, out)),
        _ => {}
    }
}

#[tokio::test]
async fn each_valid_template_gets_a_typed_operation() {
    let volume = examples_volume();
    let app = test_app(test_config(&volume));
    let doc = document(&app).await;

    for (id, operation_id, summary) in [
        ("sample", "render_sample", "Sample"),
        ("packages-demo", "render_packages_demo", "Package demo"),
        ("demo", "render_demo", "Demo document"),
    ] {
        let op = &doc["paths"][format!("/templates/{id}/render")]["post"];
        assert_eq!(op["operationId"], operation_id, "{id}");
        assert_eq!(op["summary"], summary, "{id}");
        assert!(
            op["parameters"].is_array(),
            "{id}: download/filename parameters"
        );
        assert!(op["responses"]["200"].is_object(), "{id}");
    }
    // The generic route is still there.
    assert!(doc["paths"]["/templates/{templateId}/render"]["post"].is_object());
    // An invalid template gets no typed operation.
    assert!(
        doc["paths"]
            .get("/templates/invalid-schema/render")
            .is_none()
    );
}

#[tokio::test]
async fn request_bodies_are_the_template_schemas() {
    let volume = examples_volume();
    let app = test_app(test_config(&volume));
    let doc = document(&app).await;
    let schemas = &doc["components"]["schemas"];

    let op = &doc["paths"]["/templates/demo/render"]["post"];
    assert_eq!(
        op["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/DemoRenderRequest"
    );
    let request = &schemas["DemoRenderRequest"];
    assert_eq!(request["required"], serde_json::json!(["data"]));
    assert_eq!(request["additionalProperties"], false);
    assert_eq!(
        request["properties"]["data"]["$ref"],
        "#/components/schemas/DemoData"
    );
    assert_eq!(
        request["properties"]["layout"]["$ref"],
        "#/components/schemas/DemoLayout"
    );
    assert_eq!(
        request["properties"]["metadata"]["$ref"],
        "#/components/schemas/DocumentMetadata"
    );

    // Layout defaults are published, `$defs` are hoisted into prefixed components.
    assert_eq!(
        schemas["DemoLayout"]["properties"]["accentColor"]["default"],
        "#1f4e79"
    );
    assert!(schemas["WithDefs_amount"].is_object());
    assert_eq!(
        schemas["WithDefsData"]["properties"]["total"]["$ref"],
        "#/components/schemas/WithDefs_amount"
    );
    assert!(schemas["DemoData"].get("$schema").is_none());

    // A template without `layout` has no layout property.
    assert!(schemas["SampleRenderRequest"]["properties"]["layout"].is_object());
}

/// US4 (005): a template's example is the example of its render request body; US1 (005,
/// FR-003): the copied schemas keep their `x-role` annotations.
#[tokio::test]
async fn request_body_example_is_the_template_example_and_roles_are_kept() {
    let volume = TestVolume::new();
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "object",
                "required": ["title"],
                "properties": {
                    "title": { "type": "string", "description": "Title.", "x-role": "ask" },
                    "items": {
                        "type": "array",
                        "items": { "$ref": "#/$defs/item" }
                    }
                }
            },
            "layout": {
                "type": "object",
                "properties": {
                    "dense": { "type": "boolean", "default": false, "x-role": "default" }
                }
            }
        },
        "$defs": {
            "item": {
                "type": "object",
                "properties": { "label": { "type": "string", "x-role": "fill" } }
            }
        }
    });
    let example = serde_json::json!({ "data": { "title": "Hello", "items": [{ "label": "A" }] } });
    volume
        .write_file("roles/main.typ", "= Roles")
        .write_file("roles/schema.json", schema.to_string())
        .write_file("roles/example.json", example.to_string())
        .copy_template(&sample_template(), "sample");
    let app = test_app(test_config(&volume));
    let doc = document(&app).await;

    let content = &doc["paths"]["/templates/roles/render"]["post"]["requestBody"]["content"]["application/json"];
    assert_eq!(content["example"], example);
    // Without `example.json`, no example.
    let sample = &doc["paths"]["/templates/sample/render"]["post"]["requestBody"]["content"]["application/json"];
    assert!(sample.get("example").is_none(), "{sample}");

    let schemas = &doc["components"]["schemas"];
    assert_eq!(schemas["RolesData"]["properties"]["title"]["x-role"], "ask");
    assert_eq!(
        schemas["RolesLayout"]["properties"]["dense"]["x-role"],
        "default"
    );
    assert_eq!(
        schemas["Roles_item"]["properties"]["label"]["x-role"],
        "fill"
    );
}

#[tokio::test]
async fn typed_operation_describes_the_default_version() {
    let volume = TestVolume::new();
    for (version, title) in [
        ("1.0.0", "Old title"),
        ("2.0.0", "New title"),
        ("3.0.0", ""),
    ] {
        let folder = format!("facture/{version}");
        volume.copy_template(&fixture("sample"), &folder);
        let manifest = serde_json::json!({ "name": "Facture", "version": version });
        volume.write_file(&format!("{folder}/template.json"), manifest.to_string());
        let path = volume.path().join(&folder).join("schema.json");
        let mut schema: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        schema["properties"]["data"]["properties"]["title"]["description"] = title.into();
        std::fs::write(&path, schema.to_string()).unwrap();
    }
    // The newest version is invalid: the default is 2.0.0.
    volume.remove("facture/3.0.0/main.typ");
    let app = test_app(test_config(&volume));
    let doc = document(&app).await;

    let operation = &doc["paths"]["/templates/facture/render"]["post"];
    assert_eq!(operation["description"], "(version 2.0.0)");
    let title = &doc["components"]["schemas"]["FactureData"]["properties"]["title"];
    assert_eq!(title["description"], "New title");

    // The `version` parameter, on the typed operation and the generic ones.
    let version_parameter = |operation: &Value| -> Value {
        operation["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == "version")
            .cloned()
            .unwrap_or_else(|| panic!("no version parameter: {operation}"))
    };
    for operation in [
        operation,
        &doc["paths"]["/templates/{templateId}"]["get"],
        &doc["paths"]["/templates/{templateId}/schema"]["get"],
        &doc["paths"]["/templates/{templateId}/render"]["post"],
    ] {
        let parameter = version_parameter(operation);
        assert_eq!(parameter["in"], "query", "{parameter}");
        assert_eq!(parameter["required"], false, "{parameter}");
        assert_eq!(parameter["schema"]["type"], "string", "{parameter}");
    }
}

#[tokio::test]
async fn every_reference_resolves() {
    let volume = examples_volume();
    let app = test_app(test_config(&volume));
    let doc = document(&app).await;
    let mut all = Vec::new();
    refs(&doc, &mut all);
    assert!(!all.is_empty());
    for r in all {
        assert!(!r.contains("$defs"), "unrewritten reference {r}");
        let name = r
            .strip_prefix("#/components/schemas/")
            .or_else(|| r.strip_prefix("#/components/responses/"))
            .unwrap_or_else(|| panic!("unexpected reference {r}"));
        let kind = if r.contains("/schemas/") {
            "schemas"
        } else {
            "responses"
        };
        assert!(
            doc["components"][kind][name].is_object(),
            "dangling reference {r}"
        );
    }
}

#[tokio::test]
async fn empty_volume_gives_the_static_document() {
    let volume = TestVolume::new();
    let app = test_app(test_config(&volume));
    let doc = document(&app).await;
    assert_eq!(doc, serde_json::to_value(inkpdf::openapi()).unwrap());
}

#[tokio::test]
async fn colliding_prefixes_are_made_unique() {
    let volume = TestVolume::new();
    for id in ["a-b", "a_b"] {
        volume.copy_template(&sample_template(), id);
    }
    let app = test_app(test_config(&volume));
    let doc = document(&app).await;
    let first = &doc["paths"]["/templates/a-b/render"]["post"]["requestBody"]["content"]["application/json"]
        ["schema"]["$ref"];
    let second = &doc["paths"]["/templates/a_b/render"]["post"]["requestBody"]["content"]["application/json"]
        ["schema"]["$ref"];
    assert!(first.is_string() && second.is_string());
    assert_ne!(first, second);
}

struct LiveApp {
    app: Router,
    _watcher: WatcherHandle,
}

fn live_app(volume: &TestVolume) -> LiveApp {
    let state = test_state(Config {
        rescan_interval: Duration::from_secs(1),
        ..test_config(volume)
    });
    let watcher = watcher::spawn(state.registry.clone(), state.config.rescan_interval);
    LiveApp {
        app: build_app(state),
        _watcher: watcher,
    }
}

async fn eventually(app: &Router, what: &str, check: impl Fn(&Value) -> bool) {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(8) {
        if check(&document(app).await) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    panic!("{what}: not observed in the OpenAPI document");
}

#[tokio::test]
async fn document_follows_hot_reload() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let live = live_app(&volume);
    let path = "/templates/added/render";

    volume.copy_template(&sample_template(), "added");
    eventually(&live.app, "template added", |d| {
        d["paths"].get(path).is_some()
    })
    .await;

    let schema = std::fs::read_to_string(volume.path().join("added/schema.json")).unwrap();
    volume.write_file(
        "added/schema.json",
        schema.replace("\"default\": \"left\"", "\"default\": \"right\""),
    );
    eventually(&live.app, "schema changed", |d| {
        d["components"]["schemas"]["AddedLayout"]["properties"]["align"]["default"] == "right"
    })
    .await;

    volume.remove("added");
    eventually(&live.app, "template removed", |d| {
        d["paths"].get(path).is_none()
    })
    .await;
}
