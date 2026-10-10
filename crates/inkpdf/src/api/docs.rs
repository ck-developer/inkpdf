//! Interactive API documentation (`GET /docs`).
//!
//! A static page that loads Scalar (pinned version) in the reader's browser and points it at
//! `/openapi.json`, so every visit shows the live, template-aware document.

use axum::http::header;
use axum::response::IntoResponse;

/// Scalar API reference, pinned to an exact version.
const SCALAR: &str = "https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.72.1";

/// Interactive API documentation page.
pub async fn docs_page() -> impl IntoResponse {
    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>inkpdf API</title>
</head>
<body>
  <script id="api-reference" data-url="/openapi.json"></script>
  <script src="{SCALAR}"></script>
</body>
</html>
"#
    );
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}
