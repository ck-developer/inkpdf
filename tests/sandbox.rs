//! Garanties du bac à sable : paquets (seuls les paquets intégrés, confinés à leur racine),
//! lecture hors du dossier, liens sortants, injection.

mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;

async fn render(volume: &TestVolume, id: &str) -> (StatusCode, serde_json::Value) {
    let app = test_app(test_config(volume));
    let (status, _, body) = post_json(
        &app,
        &format!("/templates/{id}/render"),
        &json!({"data": {}}),
    )
    .await;
    let body = serde_json::from_slice(&body).unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn package_not_offered_by_the_service_is_refused() {
    // `evil-package` importe un paquet absent (et écrit une version) : template invalide,
    // aucune compilation, aucun téléchargement.
    let volume = TestVolume::new();
    volume.copy_template(&fixture("evil-package"), "evil-package");
    let (status, problem) = render(&volume, "evil-package").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(problem["code"], "template-invalid");
}

#[tokio::test]
async fn bundled_package_import_is_accepted() {
    let volume = TestVolume::new();
    volume
        .write_file(
            "with-package/main.typ",
            "#import \"@preview/zero\": num\n#num(\"1234.5\")\n",
        )
        .write_file("with-package/schema.json", MINIMAL_SCHEMA);
    let (status, problem) = render(&volume, "with-package").await;
    assert_eq!(status, StatusCode::OK, "{problem}");
}

#[test]
fn package_cannot_read_template_files() {
    use inkpdf::render::world::SandboxWorld;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use typst::World;
    use typst::syntax::{RootedPath, VirtualPath, VirtualRoot};

    let entry = entry_with_main("confined", "secret");
    let world = SandboxWorld::new(
        entry,
        &json!({"data": {}}),
        Arc::new(AtomicBool::new(false)),
    );
    let zero = inkpdf::packages::selected("zero").unwrap().spec().clone();
    // Dans la racine du paquet, `/main.typ` ne désigne jamais le fichier du template.
    let id = RootedPath::new(
        VirtualRoot::Package(zero),
        VirtualPath::new("main.typ").unwrap(),
    )
    .intern();
    assert!(world.file(id).is_err());
    // Et un paquet inconnu n'expose rien.
    let unknown = "@preview/unknown:1.0.0".parse().unwrap();
    let id = RootedPath::new(
        VirtualRoot::Package(unknown),
        VirtualPath::new("lib.typ").unwrap(),
    )
    .intern();
    assert!(world.file(id).is_err());
}

#[tokio::test]
async fn template_can_pass_an_image_to_a_package() {
    let volume = TestVolume::new();
    volume
        .write_file(
            "pass-image/main.typ",
            "#import \"@preview/showybox\": showybox\n#showybox(image(\"logo.svg\", width: 1cm))\n",
        )
        .write_file(
            "pass-image/logo.svg",
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#,
        )
        .write_file("pass-image/schema.json", MINIMAL_SCHEMA);
    let (status, problem) = render(&volume, "pass-image").await;
    assert_eq!(status, StatusCode::OK, "{problem}");
}

#[tokio::test]
async fn parent_directory_read_is_refused() {
    let volume = TestVolume::new();
    volume.copy_template(&fixture("evil-parent"), "evil-parent");
    let (status, problem) = render(&volume, "evil-parent").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(problem["code"], "render-failed");
}

#[tokio::test]
async fn absolute_path_read_is_refused() {
    let volume = TestVolume::new();
    volume.copy_template(&fixture("evil-absolute"), "evil-absolute");
    let (status, problem) = render(&volume, "evil-absolute").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(problem["code"], "render-failed");
}

#[cfg(unix)]
#[tokio::test]
async fn escaping_symlink_is_not_readable() {
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.txt"), "TOP-SECRET").unwrap();

    let volume = TestVolume::new();
    volume
        .write_file("evil-symlink/main.typ", "#read(\"assets/x\")\n")
        .write_file(
            "evil-symlink/schema.json",
            r#"{"type":"object","properties":{"data":{"type":"object"}}}"#,
        );
    std::fs::create_dir_all(volume.path().join("evil-symlink/assets")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret.txt"),
        volume.path().join("evil-symlink/assets/x"),
    )
    .unwrap();

    let (status, problem) = render(&volume, "evil-symlink").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(problem["code"], "render-failed");
    assert!(!problem.to_string().contains("TOP-SECRET"));
}

#[tokio::test]
async fn code_in_data_is_rendered_literally() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let app = test_app(test_config(&volume));
    let injected = "#import \"/etc/passwd\"";
    let body = json!({"data": {"title": injected, "items": [{"label": "a", "value": 1}]}});
    let (status, _, pdf) = post_json(&app, "/templates/sample/render", &body).await;
    assert_eq!(status, StatusCode::OK);
    let text = pdf_text(&pdf);
    assert!(text.contains("#import"), "{text}");
    assert!(text.contains("/etc/passwd"), "{text}");
}
