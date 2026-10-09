//! Le service sert sa propre description OpenAPI et une UI de documentation.

mod common;

use axum::http::{StatusCode, header};
use common::*;

#[tokio::test]
async fn openapi_json_is_served() {
    let volume = TestVolume::new();
    let app = test_app(test_config(&volume));
    let (status, doc) = get_json(&app, "/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(doc["openapi"], "3.1.0");
    assert_eq!(doc["info"]["title"], "inkpdf");
}

#[tokio::test]
async fn docs_ui_is_served() {
    let volume = TestVolume::new();
    let app = test_app(test_config(&volume));
    let (status, headers, body) = get(&app, "/docs").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(String::from_utf8_lossy(&body).contains("<html"));
}
