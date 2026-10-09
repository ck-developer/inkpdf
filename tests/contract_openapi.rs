//! Le document OpenAPI généré depuis le code doit être identique au fichier versionné
//! `openapi/openapi.json`. `INKPDF_UPDATE_OPENAPI=1 cargo test --test contract_openapi`
//! régénère le fichier.

mod common;

use axum::http::StatusCode;
use common::*;

#[tokio::test]
async fn generated_document_matches_versioned_file() {
    let volume = TestVolume::new();
    let app = test_app(test_config(&volume));
    let (status, generated) = get_json(&app, "/openapi.json").await;
    assert_eq!(status, StatusCode::OK);

    let path = repo_root().join("openapi/openapi.json");
    let pretty = serde_json::to_string_pretty(&generated).unwrap() + "\n";
    if std::env::var("INKPDF_UPDATE_OPENAPI").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &pretty).unwrap();
        return;
    }

    let versioned = std::fs::read_to_string(&path)
        .expect("openapi/openapi.json is missing; run INKPDF_UPDATE_OPENAPI=1 cargo test --test contract_openapi");
    let versioned: serde_json::Value = serde_json::from_str(&versioned).unwrap();
    assert!(
        versioned == generated,
        "the OpenAPI document changed; review the diff and run \
         INKPDF_UPDATE_OPENAPI=1 cargo test --test contract_openapi"
    );
}
