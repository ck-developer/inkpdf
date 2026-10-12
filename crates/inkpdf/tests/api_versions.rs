//! Template versions over HTTP (feature 005, US7): one folder per version, selection with
//! `?version=`, the `Inkpdf-Template-Version` header.

mod common;

use std::fs;

use axum::http::StatusCode;
use common::*;
use serde_json::{Value, json};

const VERSIONS: [&str; 3] = ["1.0.0", "1.1.0", "2.0.0"];

/// The `sample` fixture copied as `facture/<version>/`, each with its version in
/// `template.json` and a line naming it in the document.
fn volume() -> TestVolume {
    let volume = TestVolume::new();
    for version in VERSIONS {
        let folder = format!("facture/{version}");
        volume.copy_template(&sample_template(), &folder);
        let manifest = json!({ "name": "Facture", "version": version });
        volume.write_file(&format!("{folder}/template.json"), manifest.to_string());
        let main = volume.path().join(&folder).join("main.typ");
        let mut text = fs::read_to_string(&main).unwrap();
        text.push_str(&format!("\nTemplate version {version}\n"));
        fs::write(main, text).unwrap();
    }
    volume
}

fn app(volume: &TestVolume) -> axum::Router {
    test_app(test_config(volume))
}

/// Renders `facture` with `query`; the version header and the version line of the PDF.
async fn render(app: &axum::Router, query: &str) -> (Option<String>, String) {
    let (status, headers, body) = post_json(
        app,
        &format!("/templates/facture/render{query}"),
        &sample_request(),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{query}: {}",
        String::from_utf8_lossy(&body)
    );
    let header = headers
        .get("inkpdf-template-version")
        .map(|v| v.to_str().unwrap().to_owned());
    let text = pdf_text(&body);
    let line = text
        .lines()
        .find(|line| line.contains("Template version"))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (header, line)
}

#[tokio::test]
async fn list_shows_the_versions_newest_first() {
    let volume = volume();
    let (status, body) = get_json(&app(&volume), "/templates").await;
    assert_eq!(status, StatusCode::OK);
    let facture = &body["templates"][0];
    assert_eq!(facture["id"], "facture");
    assert_eq!(facture["version"], "2.0.0");
    assert_eq!(facture["status"], "valid");
    assert_eq!(
        facture["versions"],
        json!([
            { "version": "2.0.0", "status": "valid" },
            { "version": "1.1.0", "status": "valid" },
            { "version": "1.0.0", "status": "valid" },
        ])
    );
}

#[tokio::test]
async fn detail_and_schema_describe_the_version_asked_for() {
    let volume = volume();
    // 1.0.0 differs by a default the others do not have.
    let schema_path = volume.path().join("facture/1.0.0/schema.json");
    let mut schema: Value = serde_json::from_slice(&fs::read(&schema_path).unwrap()).unwrap();
    schema["properties"]["layout"]["properties"]["primaryColor"]["default"] = json!("#000000");
    fs::write(&schema_path, schema.to_string()).unwrap();
    let app = app(&volume);

    let (status, detail) = get_json(&app, "/templates/facture").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["version"], "2.0.0");
    assert_eq!(detail["versions"].as_array().unwrap().len(), 3);

    let (status, detail) = get_json(&app, "/templates/facture?version=1.0.0").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["version"], "1.0.0");
    let default = "/properties/layout/properties/primaryColor/default";
    assert_eq!(detail["schema"].pointer(default).unwrap(), "#000000");

    let (status, schema) = get_json(&app, "/templates/facture/schema?version=1").await;
    assert_eq!(status, StatusCode::OK);
    // `1` is 1.1.0, the highest 1.x.y.
    assert_eq!(schema.pointer(default).unwrap(), "#1f4e79");
    let (_, schema) = get_json(&app, "/templates/facture/schema?version=1.0.0").await;
    assert_eq!(schema.pointer(default).unwrap(), "#000000");

    // The schema response names the version it served (constitution VI).
    for (query, version) in [
        ("?version=1", "1.1.0"),
        ("?version=1.0.0", "1.0.0"),
        ("", "2.0.0"),
    ] {
        let (status, headers, _) = get(&app, &format!("/templates/facture/schema{query}")).await;
        assert_eq!(status, StatusCode::OK, "{query}");
        assert_eq!(headers["inkpdf-template-version"], version, "{query}");
    }
}

#[tokio::test]
async fn renders_use_the_version_asked_for() {
    let volume = volume();
    let app = app(&volume);
    for (query, version) in [
        ("", "2.0.0"),
        ("?version=1", "1.1.0"),
        ("?version=1.0.0", "1.0.0"),
        ("?version=2", "2.0.0"),
        ("?download=1&version=1.1.0", "1.1.0"),
    ] {
        let (header, line) = render(&app, query).await;
        assert_eq!(header.as_deref(), Some(version), "{query}");
        assert_eq!(line, format!("Template version {version}"), "{query}");
    }
}

#[tokio::test]
async fn unknown_version_lists_the_available_ones() {
    let volume = volume();
    let app = app(&volume);
    for uri in [
        "/templates/facture?version=3",
        "/templates/facture/schema?version=3",
    ] {
        let (status, problem) = get_json(&app, uri).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(problem["code"], "template-version-not-found", "{uri}");
    }
    let (status, _, body) = post_json(
        &app,
        "/templates/facture/render?version=3",
        &sample_request(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        problem(&body),
        json!({
            "type": "https://github.com/ck-developer/inkpdf/errors/template-version-not-found",
            "code": "template-version-not-found",
            "title": "Template version not found",
            "status": 404,
            "detail": "template facture has no version matching \"3\"; available: 2.0.0, 1.1.0, 1.0.0",
            "templateId": "facture",
            "availableVersions": ["2.0.0", "1.1.0", "1.0.0"],
        })
    );
}

#[tokio::test]
async fn malformed_version_is_an_invalid_parameter() {
    let volume = volume();
    let app = app(&volume);
    for version in ["latest", "v2", "1.2", "", "..%2F1.0.0", "1.0.0%2F.."] {
        for uri in [
            format!("/templates/facture?version={version}"),
            format!("/templates/facture/schema?version={version}"),
        ] {
            let (status, problem) = get_json(&app, &uri).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
            assert_eq!(problem["code"], "invalid-parameter", "{uri}");
            assert_eq!(problem["templateId"], "facture", "{uri}");
        }
        let uri = format!("/templates/facture/render?version={version}");
        let (status, _, body) = post_json(&app, &uri, &sample_request()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        let problem = problem(&body);
        assert_eq!(problem["code"], "invalid-parameter", "{uri}");
        assert!(
            problem["detail"]
                .as_str()
                .unwrap()
                .starts_with("invalid query parameter `version`"),
            "{problem}"
        );
    }
    // An unknown template is reported first.
    let (status, problem) = get_json(&app, "/templates/nope?version=latest").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(problem["code"], "template-not-found");
}

#[tokio::test]
async fn an_invalid_newest_version_leaves_the_previous_one_as_default() {
    let volume = volume();
    volume.write_file("facture/2.0.0/schema.json", "{");
    let app = app(&volume);

    let (_, body) = get_json(&app, "/templates").await;
    let facture = &body["templates"][0];
    assert_eq!(facture["version"], "1.1.0");
    assert_eq!(facture["status"], "valid");
    assert_eq!(facture["versions"][0]["version"], "2.0.0");
    assert_eq!(facture["versions"][0]["status"], "invalid");
    assert!(
        facture["versions"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("schema.json")
    );

    let (header, _) = render(&app, "").await;
    assert_eq!(header.as_deref(), Some("1.1.0"));
    // Asked for exactly, the invalid version is refused as invalid.
    let (status, problem) = get_json(&app, "/templates/facture/schema?version=2.0.0").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(problem["code"], "template-invalid");
}

#[tokio::test]
async fn flat_template_keeps_working_with_its_manifest_version() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let app = app(&volume);

    let (_, body) = get_json(&app, "/templates").await;
    let sample = &body["templates"][0];
    assert_eq!(sample["version"], "1.0.0");
    assert_eq!(
        sample["versions"],
        json!([{ "version": "1.0.0", "status": "valid" }])
    );
    let (status, headers, _) = post_json(
        &app,
        "/templates/sample/render?version=1",
        &sample_request(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["inkpdf-template-version"], "1.0.0");

    // Without a manifest version: no header, no versions.
    volume.remove("sample/template.json");
    let app = test_app(test_config(&volume));
    let (status, headers, _) = post_json(&app, "/templates/sample/render", &sample_request()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers.get("inkpdf-template-version").is_none());
    let (_, body) = get_json(&app, "/templates").await;
    assert_eq!(body["templates"][0]["versions"], json!([]));
    let (status, problem) = get_json(&app, "/templates/sample?version=1").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(problem["availableVersions"], json!([]));
}
