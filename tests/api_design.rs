//! US2 : variation du rendu par les paramètres de design.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{Value, json};

const FOOTER: &str = "inkpdf sample footer";

fn app() -> (TestVolume, axum::Router) {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let app = test_app(test_config(&volume));
    (volume, app)
}

fn with_design(design: Value) -> Value {
    let mut body = sample_request();
    body["design"] = design;
    body
}

fn violation_paths(body: &[u8]) -> Vec<String> {
    let problem: Value = serde_json::from_slice(body).unwrap();
    assert_eq!(problem["code"], "validation-failed");
    problem["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["path"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn primary_color_changes_the_pdf_but_not_the_text() {
    let (_volume, app) = app();
    let (s1, _, blue) = post_json(
        &app,
        "/templates/sample/render",
        &with_design(json!({"primaryColor": "#1f4e79"})),
    )
    .await;
    let (s2, _, red) = post_json(
        &app,
        "/templates/sample/render",
        &with_design(json!({"primaryColor": "#c0392b"})),
    )
    .await;
    assert_eq!((s1, s2), (StatusCode::OK, StatusCode::OK));
    assert_eq!(pdf_text(&blue), pdf_text(&red));
    assert_ne!(blue, red);
}

#[tokio::test]
async fn show_footer_false_removes_the_footer() {
    let (_volume, app) = app();
    let (_, _, default) = post_json(&app, "/templates/sample/render", &sample_request()).await;
    assert!(pdf_text(&default).contains(FOOTER));

    let (status, _, hidden) = post_json(
        &app,
        "/templates/sample/render",
        &with_design(json!({"showFooter": false})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!pdf_text(&hidden).contains(FOOTER));
}

#[tokio::test]
async fn align_out_of_enum_is_rejected() {
    let (_volume, app) = app();
    let (status, _, body) = post_json(
        &app,
        "/templates/sample/render",
        &with_design(json!({"align": "top"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(violation_paths(&body), ["/design/align"]);
}

#[tokio::test]
async fn color_not_matching_pattern_is_rejected() {
    let (_volume, app) = app();
    let (status, _, body) = post_json(
        &app,
        "/templates/sample/render",
        &with_design(json!({"primaryColor": "red"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(violation_paths(&body), ["/design/primaryColor"]);
}

#[tokio::test]
async fn unknown_design_property_is_rejected() {
    let (_volume, app) = app();
    let (status, _, body) = post_json(
        &app,
        "/templates/sample/render",
        &with_design(json!({"fontSize": 12})),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(violation_paths(&body), ["/design"]);
}

#[tokio::test]
async fn data_and_design_violations_are_all_listed() {
    let (_volume, app) = app();
    let body = json!({
        "data": {"title": "T", "items": [{"label": "a", "value": -1}]},
        "design": {"align": "top"}
    });
    let (status, _, body) = post_json(&app, "/templates/sample/render", &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let mut paths = violation_paths(&body);
    paths.sort();
    assert_eq!(paths, ["/data/items/0/value", "/design/align"]);
}
