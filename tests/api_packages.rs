//! US3 (002) : `GET /packages` liste les paquets mis à disposition des templates.

mod common;

use axum::http::{StatusCode, header};
use common::*;

#[tokio::test]
async fn lists_packages_offered_to_templates() {
    let volume = TestVolume::new();
    let app = test_app(test_config(&volume));
    let (status, headers, body) = get(&app, "/packages").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "application/json");
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let packages = body["packages"].as_array().unwrap();
    assert_eq!(packages.len(), 20);
    let names: Vec<&str> = packages
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "sorted by name");

    for package in packages {
        let object = package.as_object().unwrap();
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(
            keys,
            ["description", "import", "license", "name", "version"]
        );
        assert_eq!(
            package["import"],
            format!("@preview/{}", package["name"].as_str().unwrap())
        );
    }

    // Les dépendances internes ne sont pas importables, donc pas listées.
    for internal in [
        "komet",
        "suiji",
        "elembic",
        "tiptoe",
        "datify-core",
        "rustycure",
    ] {
        assert!(!names.contains(&internal), "{internal}");
    }
    let zero = packages.iter().find(|p| p["name"] == "zero").unwrap();
    assert_eq!(zero["version"], "0.7.1");
    assert_eq!(zero["license"], "MIT");
}
