//! US3 : liste, détail et schéma des templates.

mod common;

use axum::http::{StatusCode, header};
use common::*;

fn app() -> (TestVolume, axum::Router) {
    let volume = TestVolume::new();
    volume
        .copy_template(&sample_template(), "sample")
        .copy_template(&fixture("invalid-schema"), "invalid-schema");
    let app = test_app(test_config(&volume));
    (volume, app)
}

#[tokio::test]
async fn list_is_sorted_with_status_and_reason() {
    let (_volume, app) = app();
    let (status, body) = get_json(&app, "/templates").await;
    assert_eq!(status, StatusCode::OK);
    let templates = body["templates"].as_array().unwrap();
    assert_eq!(templates.len(), 2);

    let invalid = &templates[0];
    assert_eq!(invalid["id"], "invalid-schema");
    assert_eq!(invalid["name"], "invalid-schema");
    assert_eq!(invalid["status"], "invalid");
    assert!(invalid["reason"].as_str().unwrap().contains("schema.json"));

    let sample = &templates[1];
    assert_eq!(sample["id"], "sample");
    assert_eq!(sample["name"], "Exemple");
    assert_eq!(sample["version"], "1.0.0");
    assert!(sample["description"].is_string());
    assert_eq!(sample["status"], "valid");
    assert!(sample.get("reason").is_none());
}

#[tokio::test]
async fn detail_exposes_schema_and_links() {
    let (_volume, app) = app();
    let (status, body) = get_json(&app, "/templates/sample").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], "sample");
    assert!(body["loadedAt"].as_str().unwrap().contains('T'));
    let properties = &body["schema"]["properties"];
    assert!(properties.get("data").is_some());
    assert_eq!(
        properties["design"]["properties"]["primaryColor"]["default"],
        "#1f4e79"
    );
    assert_eq!(body["links"]["schema"], "/templates/sample/schema");
    assert_eq!(body["links"]["render"], "/templates/sample/render");
}

#[tokio::test]
async fn detail_of_invalid_template_has_reason_but_no_schema() {
    let (_volume, app) = app();
    let (status, body) = get_json(&app, "/templates/invalid-schema").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "invalid");
    assert!(body["reason"].is_string());
    assert!(body.get("schema").is_none());
}

#[tokio::test]
async fn raw_schema_is_byte_identical_to_the_file() {
    let (volume, app) = app();
    let (status, headers, body) = get(&app, "/templates/sample/schema").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "application/schema+json");
    let file = std::fs::read(volume.path().join("sample/schema.json")).unwrap();
    assert_eq!(body.as_ref(), file.as_slice());
}

#[tokio::test]
async fn raw_schema_without_additional_properties_is_not_modified() {
    let volume = TestVolume::new();
    let schema = "{ \"type\": \"object\",\n  \"properties\": { \"data\": {} } }\n";
    volume
        .write_file("plain/main.typ", "= Plain")
        .write_file("plain/schema.json", schema);
    let app = test_app(test_config(&volume));
    let (status, _, body) = get(&app, "/templates/plain/schema").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_ref(), schema.as_bytes());
}

#[tokio::test]
async fn schema_of_invalid_template_is_a_conflict() {
    let (_volume, app) = app();
    let (status, _, body) = get(&app, "/templates/invalid-schema/schema").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(problem(&body)["code"], "template-invalid");
}

#[tokio::test]
async fn unknown_template_is_not_found_on_both_routes() {
    let (_volume, app) = app();
    for uri in [
        "/templates/unknown",
        "/templates/unknown/schema",
        "/templates/..%2F..%2Fetc",
    ] {
        let (status, headers, body) = get(&app, uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(headers[header::CONTENT_TYPE], "application/problem+json");
        assert_eq!(problem(&body)["code"], "template-not-found");
    }
}

/// US2 (002) : un import de paquet incorrect rend le template invalide dès le chargement.
#[tokio::test]
async fn incorrect_package_imports_make_templates_invalid() {
    let volume = TestVolume::new();
    for name in ["version-written", "unknown-package", "other-namespace", "dynamic-import"] {
        volume.copy_template(&fixture(name), name);
    }
    let app = test_app(test_config(&volume));

    let expected = [
        (
            "version-written",
            "main.typ:2: remove the version: write @preview/zero (inkpdf uses its installed version)",
        ),
        (
            "unknown-package",
            "main.typ:2: package @preview/does-not-exist is not available in inkpdf (see GET /packages)",
        ),
        (
            "other-namespace",
            "main.typ:2: only @preview packages are available: @local/zero",
        ),
    ];
    for (id, reason) in expected {
        let (status, body) = get_json(&app, &format!("/templates/{id}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["status"], "invalid", "{id}");
        assert_eq!(body["reason"], reason, "{id}");

        let (status, _, body) = post_json(
            &app,
            &format!("/templates/{id}/render"),
            &serde_json::json!({"data": {}}),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT, "{id}");
        assert_eq!(problem(&body)["code"], "template-invalid", "{id}");
    }

    // Import calculé : invisible au chargement, refusé au rendu (règle 4 du contrat).
    let (_, body) = get_json(&app, "/templates/dynamic-import").await;
    assert_eq!(body["status"], "valid");
    let (status, _, body) = post_json(
        &app,
        "/templates/dynamic-import/render",
        &serde_json::json!({"data": {}}),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(problem(&body)["code"], "render-failed");
}
