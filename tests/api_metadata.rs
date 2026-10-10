//! 002/US6: PDF metadata (optional `metadata`, configurable default author).

mod common;

use axum::http::StatusCode;
use common::*;
use inkpdf::Config;
use serde_json::{Value, json};

fn volume() -> TestVolume {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    volume
}

async fn render(config: Config, body: &Value) -> (StatusCode, Vec<u8>) {
    let app = test_app(config);
    let (status, _, body) = post_json(&app, "/templates/sample/render", body).await;
    (status, body.to_vec())
}

/// Text of the XMP stream (Typst does not compress it).
fn xmp(pdf: &[u8]) -> String {
    let text = String::from_utf8_lossy(pdf);
    let start = text.find("<x:xmpmeta").expect("XMP metadata");
    let end = text[start..].find("</x:xmpmeta>").expect("end of XMP") + start;
    text[start..end].to_string()
}

#[tokio::test]
async fn request_metadata_is_written_to_the_pdf() {
    let volume = volume();
    let mut body = sample_request();
    body["metadata"] = json!({
        "title": "Quarterly report",
        "author": ["ACME Corp", "Accounting department"],
        "subject": "Quarter summary",
        "keywords": ["report", "Q3"],
        "date": "2026-10-01"
    });
    let (status, pdf) = render(test_config(&volume), &body).await;
    assert_eq!(status, StatusCode::OK);
    let xmp = xmp(&pdf);
    assert!(xmp.contains("Quarterly report"), "{xmp}");
    assert!(
        xmp.contains("ACME Corp") && xmp.contains("Accounting department"),
        "{xmp}"
    );
    assert!(xmp.contains("Quarter summary"), "{xmp}");
    assert!(xmp.contains("report") && xmp.contains("Q3"), "{xmp}");
    assert!(xmp.contains("2026-10-01"), "{xmp}");
}

#[tokio::test]
async fn defaults_apply_without_metadata() {
    let volume = volume();
    let (status, pdf) = render(test_config(&volume), &sample_request()).await;
    assert_eq!(status, StatusCode::OK);
    let xmp = xmp(&pdf);
    // Title = template name (template.json), author = the service's default author.
    assert!(
        xmp.contains("<dc:title>") && xmp.contains("Sample"),
        "{xmp}"
    );
    assert!(
        xmp.contains("<dc:creator>") && xmp.contains("inkpdf"),
        "{xmp}"
    );
}

#[tokio::test]
async fn default_author_is_configurable() {
    let volume = volume();
    let config = Config {
        default_author: "O'Neill & Partners".into(),
        ..test_config(&volume)
    };
    let (_, pdf) = render(config, &sample_request()).await;
    let xmp = xmp(&pdf);
    assert!(xmp.contains("Neill") && !xmp.contains(">inkpdf<"), "{xmp}");
}

#[tokio::test]
async fn invalid_metadata_is_rejected_before_compilation() {
    let volume = volume();
    let mut body = sample_request();
    body["metadata"] = json!({ "title": 42, "producer": "x", "date": "01/10/2026" });
    let app = test_app(test_config(&volume));
    let (status, _, body) = post_json(&app, "/templates/sample/render", &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let problem = problem(&body);
    assert_eq!(problem["code"], "validation-failed");
    let paths: Vec<&str> = problem["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["path"].as_str())
        .collect();
    assert!(paths.contains(&"/metadata/title"), "{paths:?}");
    assert!(paths.contains(&"/metadata/date"), "{paths:?}");
    assert!(
        paths.contains(&"/metadata"),
        "unknown key `producer`: {paths:?}"
    );
}

#[tokio::test]
async fn rendering_with_metadata_is_deterministic() {
    let volume = volume();
    let mut body = sample_request();
    body["metadata"] = json!({ "title": "T", "author": "A", "date": "2026-01-02" });
    let app = test_app(test_config(&volume));
    let (_, _, first) = post_json(&app, "/templates/sample/render", &body).await;
    let (_, _, second) = post_json(&app, "/templates/sample/render", &body).await;
    assert!(first == second);
}
