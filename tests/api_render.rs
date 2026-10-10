//! US1 : génération d'un PDF par `POST /templates/{id}/render`.

mod common;

use axum::http::{StatusCode, header};
use common::*;
use inkpdf::Config;
use serde_json::json;

fn volume() -> TestVolume {
    let volume = TestVolume::new();
    volume
        .copy_template(&sample_template(), "sample")
        .copy_template(&fixture("broken-typst"), "broken-typst")
        .copy_template(&fixture("invalid-schema"), "invalid-schema");
    volume
}

#[tokio::test]
async fn sample_request_renders_a_pdf() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, headers, body) =
        post_json(&app, "/templates/sample/render", &sample_request()).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "application/pdf");
    assert_eq!(
        headers[header::CONTENT_DISPOSITION],
        "inline; filename=\"sample.pdf\""
    );
    assert!(body.starts_with(b"%PDF-"));
    let text = pdf_text(&body);
    assert!(text.contains("Exemple"), "{text}");
    assert!(text.contains("Alpha"), "{text}");
}

#[tokio::test]
async fn body_without_design_uses_defaults() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let body = json!({"data": {"title": "T", "items": [{"label": "a", "value": 1}]}});
    let (status, _, _) = post_json(&app, "/templates/sample/render", &body).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn missing_title_is_rejected_before_compilation() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let body = json!({"data": {"items": [{"label": "a", "value": 1}]}});
    let (status, headers, body) = post_json(&app, "/templates/sample/render", &body).await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(headers[header::CONTENT_TYPE], "application/problem+json");
    let problem = problem(&body);
    assert_eq!(problem["code"], "validation-failed");
    assert_eq!(problem["templateId"], "sample");
    let violations = problem["violations"].as_array().unwrap();
    assert!(
        violations.iter().any(|v| v["path"] == "/data"),
        "{violations:?}"
    );
}

#[tokio::test]
async fn unknown_template_is_not_found() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, headers, body) =
        post_json(&app, "/templates/unknown/render", &sample_request()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(headers[header::CONTENT_TYPE], "application/problem+json");
    assert_eq!(problem(&body)["code"], "template-not-found");
}

#[tokio::test]
async fn traversal_id_is_not_found() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, _, body) = post_json(&app, "/templates/..%2Fetc/render", &sample_request()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(problem(&body)["code"], "template-not-found");
}

#[tokio::test]
async fn non_json_body_is_invalid_json() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, _, body) = post_raw(
        &app,
        "/templates/sample/render",
        "application/json",
        "{not json",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(problem(&body)["code"], "invalid-json");

    let (status, _, body) = post_raw(
        &app,
        "/templates/sample/render",
        "application/json",
        "[1, 2]",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(problem(&body)["code"], "invalid-json");
}

#[tokio::test]
async fn non_json_content_type_is_unsupported() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, _, body) = post_raw(&app, "/templates/sample/render", "text/plain", "{}").await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(problem(&body)["code"], "unsupported-media-type");
}

#[tokio::test]
async fn oversized_body_is_payload_too_large() {
    let volume = volume();
    let app = test_app(Config {
        max_body_bytes: 1024,
        ..test_config(&volume)
    });
    let big = json!({"data": {"title": "x".repeat(2048), "items": [{"label": "a", "value": 1}]}});
    let (status, headers, body) = post_json(&app, "/templates/sample/render", &big).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(headers[header::CONTENT_TYPE], "application/problem+json");
    assert_eq!(problem(&body)["code"], "payload-too-large");
}

#[tokio::test]
async fn invalid_template_is_a_conflict() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, _, body) = post_json(
        &app,
        "/templates/invalid-schema/render",
        &json!({"data": {}}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let problem = problem(&body);
    assert_eq!(problem["code"], "template-invalid");
    assert!(problem["detail"].as_str().unwrap().contains("schema.json"));
}

#[tokio::test]
async fn typst_error_is_render_failed_with_diagnostics() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (status, _, body) =
        post_json(&app, "/templates/broken-typst/render", &json!({"data": {}})).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let problem = problem(&body);
    assert_eq!(problem["code"], "render-failed");
    let diagnostic = &problem["diagnostics"][0];
    assert_eq!(diagnostic["file"], "main.typ", "{problem}");
    assert_eq!(diagnostic["line"], 3, "{problem}");
    assert!(
        diagnostic["message"]
            .as_str()
            .unwrap()
            .contains("unknown variable")
    );
}

#[tokio::test]
async fn identical_renders_are_byte_identical() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let (_, _, first) = post_json(&app, "/templates/sample/render", &sample_request()).await;
    let (_, _, second) = post_json(&app, "/templates/sample/render", &sample_request()).await;
    assert!(!first.is_empty());
    assert_eq!(first, second);
}

#[tokio::test]
async fn template_using_bundled_packages_renders() {
    let volume = TestVolume::new();
    volume.copy_template(
        &repo_root().join("examples/templates/packages-demo"),
        "packages-demo",
    );
    let app = test_app(test_config(&volume));
    let request: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo_root().join("examples/requests/packages-demo.json")).unwrap(),
    )
    .unwrap();
    let (status, headers, body) =
        post_json(&app, "/templates/packages-demo/render", &request).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert_eq!(headers["content-type"], "application/pdf");
    let text = pdf_text(&body);
    assert!(text.contains("Démonstration des paquets"), "{text}");
    // `zero` compose le nombre en mode mathématique : l'extraction espace chaque chiffre.
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(compact.contains("1234,56€"), "{text}");
    assert!(text.contains("DEMO-2026-0001"), "{text}");
}

/// FR-012 : une erreur dans le code d'un paquet indique le paquet, le fichier et la ligne.
#[tokio::test]
async fn error_inside_a_package_points_to_the_package_file() {
    let volume = TestVolume::new();
    volume.copy_template(&fixture("package-error"), "package-error");
    let app = test_app(test_config(&volume));
    let (status, _, body) = post_json(
        &app,
        "/templates/package-error/render",
        &json!({"data": {}}),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let problem = problem(&body);
    assert_eq!(problem["code"], "render-failed");
    let version = inkpdf::packages::selected("zero").unwrap().version();
    let prefix = format!("@preview/zero:{version}/");
    let diagnostics = problem["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics.iter().any(|d| {
            d["file"].as_str().is_some_and(|f| f.starts_with(&prefix)) && d["line"].is_u64()
        }),
        "{problem}"
    );
}
