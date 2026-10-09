//! Utilitaires partagés des tests d'intégration.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use inkpdf::{AppState, Config, build_app};
use serde_json::Value;
use tower::ServiceExt;

/// Racine du dépôt.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Template de démonstration livré dans `examples/`.
pub fn sample_template() -> PathBuf {
    repo_root().join("examples/templates/sample")
}

/// Fixture de `tests/fixtures/templates/`.
pub fn fixture(name: &str) -> PathBuf {
    repo_root().join("tests/fixtures/templates").join(name)
}

/// Corps d'exemple `examples/requests/sample.json`.
pub fn sample_request() -> Value {
    let raw = fs::read(repo_root().join("examples/requests/sample.json")).unwrap();
    serde_json::from_slice(&raw).unwrap()
}

/// Volume de templates jetable.
pub struct TestVolume {
    dir: tempfile::TempDir,
}

impl TestVolume {
    pub fn new() -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Copie récursivement `src` sous l'identifiant `id`.
    pub fn copy_template(&self, src: &Path, id: &str) -> &Self {
        copy_dir(src, &self.path().join(id));
        self
    }

    pub fn write_file(&self, relative: &str, content: impl AsRef<[u8]>) -> &Self {
        let path = self.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
        self
    }

    pub fn remove(&self, relative: &str) -> &Self {
        let path = self.path().join(relative);
        if path.is_dir() {
            fs::remove_dir_all(path).unwrap();
        } else {
            fs::remove_file(path).unwrap();
        }
        self
    }
}

pub fn copy_dir(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Configuration de test : jamais lue depuis l'environnement (tests parallèles isolés).
pub fn test_config(volume: &TestVolume) -> Config {
    Config {
        templates_dir: volume.path().to_path_buf(),
        ..Config::default()
    }
}

/// État scanné et prêt.
pub fn test_state(config: Config) -> AppState {
    let state = AppState::new(config);
    state.registry.scan_all();
    state.registry.set_ready();
    state
}

/// Application en mémoire, registre scanné et prêt.
pub fn test_app(config: Config) -> Router {
    build_app(test_state(config))
}

pub type Response = (StatusCode, HeaderMap, Bytes);

pub async fn send(app: &Router, request: Request<Body>) -> Response {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, headers, body)
}

pub async fn get(app: &Router, uri: &str) -> Response {
    send(app, Request::get(uri).body(Body::empty()).unwrap()).await
}

pub async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let (status, _, body) = get(app, uri).await;
    (status, serde_json::from_slice(&body).unwrap())
}

pub async fn post_raw(
    app: &Router,
    uri: &str,
    content_type: &str,
    body: impl Into<Body>,
) -> Response {
    let request = Request::post(uri)
        .header("content-type", content_type)
        .body(body.into())
        .unwrap();
    send(app, request).await
}

pub async fn post_json(app: &Router, uri: &str, body: &Value) -> Response {
    post_raw(
        app,
        uri,
        "application/json",
        serde_json::to_vec(body).unwrap(),
    )
    .await
}

/// Corps d'une réponse problem+json.
pub fn problem(body: &Bytes) -> Value {
    serde_json::from_slice(body).unwrap()
}

/// Texte extrait d'un PDF.
pub fn pdf_text(bytes: &[u8]) -> String {
    pdf_extract::extract_text_from_mem(bytes).unwrap()
}
