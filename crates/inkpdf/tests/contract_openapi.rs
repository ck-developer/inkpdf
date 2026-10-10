//! The OpenAPI document of a service with **no template loaded** (the generic routes) must
//! match the committed file `openapi/openapi.json`; `info.version` is ignored so that release
//! bumps do not break the contract. `INKPDF_UPDATE_OPENAPI=1 cargo test --test
//! contract_openapi` regenerates the file.

mod common;

use axum::http::StatusCode;
use common::*;

#[tokio::test]
async fn generated_document_matches_versioned_file() {
    let volume = TestVolume::new();
    let app = test_app(test_config(&volume));
    let (status, mut generated) = get_json(&app, "/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    generated["info"]["version"] = "0.0.0".into();

    let path = repo_root().join("openapi/openapi.json");
    let pretty = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    if std::env::var("INKPDF_UPDATE_OPENAPI").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &pretty).unwrap();
        return;
    }

    let versioned = std::fs::read_to_string(&path)
        .expect("openapi/openapi.json is missing; run INKPDF_UPDATE_OPENAPI=1 cargo test --test contract_openapi");
    let mut versioned: serde_json::Value = serde_json::from_str(&versioned).unwrap();
    versioned["info"]["version"] = "0.0.0".into();
    assert!(
        versioned == generated,
        "the OpenAPI document changed; review the diff and run \
         INKPDF_UPDATE_OPENAPI=1 cargo test --test contract_openapi"
    );
}
