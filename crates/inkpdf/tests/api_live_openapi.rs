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
    for id in ["sample", "packages-demo", "progress-invoice"] {
        volume.copy_template(&repo_root().join("examples/templates").join(id), id);
    }
    volume.copy_template(&fixture("invalid-schema"), "invalid-schema");
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
        (
            "progress-invoice",
            "render_progress_invoice",
            "Construction progress invoice",
        ),
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

    let op = &doc["paths"]["/templates/progress-invoice/render"]["post"];
    assert_eq!(
        op["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ProgressInvoiceRenderRequest"
    );
    let request = &schemas["ProgressInvoiceRenderRequest"];
    assert_eq!(request["required"], serde_json::json!(["data"]));
    assert_eq!(request["additionalProperties"], false);
    assert_eq!(
        request["properties"]["data"]["$ref"],
        "#/components/schemas/ProgressInvoiceData"
    );
    assert_eq!(
        request["properties"]["layout"]["$ref"],
        "#/components/schemas/ProgressInvoiceLayout"
    );
    assert_eq!(
        request["properties"]["metadata"]["$ref"],
        "#/components/schemas/DocumentMetadata"
    );

    // Layout defaults are published, `$defs` are hoisted into prefixed components.
    assert_eq!(
        schemas["ProgressInvoiceLayout"]["properties"]["primaryColor"]["default"],
        "#1d3557"
    );
    assert!(schemas["ProgressInvoice_amount"].is_object());
    assert!(schemas["ProgressInvoiceData"].get("$schema").is_none());

    // A template without `layout` has no layout property.
    assert!(schemas["SampleRenderRequest"]["properties"]["layout"].is_object());
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
        volume.copy_template(&repo_root().join("examples/templates/sample"), id);
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
    volume.copy_template(&repo_root().join("examples/templates/sample"), "sample");
    let live = live_app(&volume);
    let path = "/templates/added/render";

    volume.copy_template(&repo_root().join("examples/templates/sample"), "added");
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
