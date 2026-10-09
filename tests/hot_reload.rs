//! US4 : templates ajoutés, modifiés et supprimés à chaud.

mod common;

use std::future::Future;
use std::time::{Duration, Instant};

use axum::Router;
use axum::http::StatusCode;
use common::*;
use inkpdf::registry::watcher::{self, WatcherHandle};
use inkpdf::{Config, build_app};
use serde_json::Value;

/// Délai maximal de prise en compte (SC-003).
const DEADLINE: Duration = Duration::from_secs(5);

struct LiveApp {
    app: Router,
    _watcher: WatcherHandle,
}

fn live_app(volume: &TestVolume) -> LiveApp {
    let state = test_state(Config {
        rescan_interval: Duration::from_secs(1),
        ..test_config(volume)
    });
    let watcher = watcher::spawn(state.registry.clone(), state.config.rescan_interval);
    LiveApp {
        app: build_app(state),
        _watcher: watcher,
    }
}

/// Sonde toutes les 100 ms jusqu'à ce que `check` soit vrai ; échoue au-delà de 5 s.
async fn eventually<F, Fut>(what: &str, mut check: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let started = Instant::now();
    while started.elapsed() < DEADLINE {
        if check().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("{what}: not observed within {DEADLINE:?}");
}

async fn template_status(app: &Router, id: &str) -> Option<String> {
    let (status, body) = get_json(app, &format!("/templates/{id}")).await;
    (status == StatusCode::OK).then(|| body["status"].as_str().unwrap().to_owned())
}

async fn render_text(app: &Router, id: &str) -> Result<String, (StatusCode, Value)> {
    let (status, _, body) =
        post_json(app, &format!("/templates/{id}/render"), &sample_request()).await;
    if status == StatusCode::OK {
        Ok(pdf_text(&body))
    } else {
        Err((status, serde_json::from_slice(&body).unwrap_or_default()))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn added_template_becomes_listed_and_renderable() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let live = live_app(&volume);

    volume.copy_template(&sample_template(), "sample-copy");
    eventually("sample-copy listed", || async {
        let (_, body) = get_json(&live.app, "/templates").await;
        body["templates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == "sample-copy")
    })
    .await;
    assert!(render_text(&live.app, "sample-copy").await.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn modified_main_typ_is_used_by_later_renders() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let live = live_app(&volume);

    let original = std::fs::read_to_string(sample_template().join("main.typ")).unwrap();
    volume.write_file(
        "sample/main.typ",
        format!("{original}\nHot reloaded marker\n"),
    );
    eventually("new main.typ used", || async {
        render_text(&live.app, "sample")
            .await
            .is_ok_and(|text| text.contains("Hot reloaded marker"))
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn corrupted_schema_marks_only_that_template_invalid() {
    let volume = TestVolume::new();
    volume
        .copy_template(&sample_template(), "sample")
        .copy_template(&sample_template(), "other");
    let live = live_app(&volume);

    volume.write_file("other/schema.json", "{");
    eventually("other invalid", || async {
        template_status(&live.app, "other").await.as_deref() == Some("invalid")
    })
    .await;
    let (_, body) = get_json(&live.app, "/templates").await;
    let other = body["templates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "other")
        .unwrap();
    assert!(other["reason"].as_str().unwrap().contains("schema.json"));
    assert_eq!(
        template_status(&live.app, "sample").await.as_deref(),
        Some("valid")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn removed_template_is_not_found() {
    let volume = TestVolume::new();
    volume
        .copy_template(&sample_template(), "sample")
        .copy_template(&sample_template(), "doomed");
    let live = live_app(&volume);

    volume.remove("doomed");
    eventually("doomed removed", || async {
        template_status(&live.app, "doomed").await.is_none()
    })
    .await;
    let (status, _, _) = post_json(&live.app, "/templates/doomed/render", &sample_request()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        template_status(&live.app, "sample").await.as_deref(),
        Some("valid")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn initially_empty_volume_picks_up_new_templates() {
    let volume = TestVolume::new();
    let live = live_app(&volume);
    let (_, body) = get_json(&live.app, "/templates").await;
    assert!(body["templates"].as_array().unwrap().is_empty());

    volume.copy_template(&sample_template(), "sample");
    eventually("sample visible", || async {
        template_status(&live.app, "sample").await.as_deref() == Some("valid")
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn copy_in_progress_never_serves_a_truncated_file() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let live = live_app(&volume);

    let original = std::fs::read_to_string(sample_template().join("main.typ")).unwrap();
    let updated = format!("{original}\nSecond version marker\n");
    // Première moitié : fichier tronqué, syntaxiquement invalide.
    let (head, _) = updated.split_at(updated.find("#table(").unwrap() + "#table(".len());

    let writer = {
        let path = volume.path().join("sample/main.typ");
        let head = head.to_owned();
        let updated = updated.clone();
        tokio::spawn(async move {
            std::fs::write(&path, head).unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;
            std::fs::write(&path, updated).unwrap();
        })
    };

    let started = Instant::now();
    let mut saw_new = false;
    while started.elapsed() < DEADLINE {
        match render_text(&live.app, "sample").await {
            Ok(text) => {
                if text.contains("Second version marker") {
                    saw_new = true;
                    break;
                }
            }
            Err(error) => panic!("render failed during copy: {error:?}"),
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    writer.await.unwrap();
    assert!(saw_new, "the complete new version was never served");
}
