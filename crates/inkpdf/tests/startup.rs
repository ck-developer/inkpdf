//! SC-002 (startup with 50 templates) and SC-007 (20 concurrent renders); feature 005, US8:
//! templates loaded at startup, hot reload only when turned on.

mod common;

use std::time::{Duration, Instant};

use axum::Router;
use axum::http::StatusCode;
use common::*;
use inkpdf::{AppState, Config, build_app, spawn_background_tasks};

#[test]
fn registry_with_50_templates_is_ready_within_2_seconds() {
    let volume = TestVolume::new();
    for i in 0..50 {
        volume.copy_template(&sample_template(), &format!("sample-{i:02}"));
    }
    let state = AppState::new(test_config(&volume));

    let started = Instant::now();
    state.registry.scan_all();
    state.registry.set_ready();
    let elapsed = started.elapsed();

    assert_eq!(state.registry.valid_count(), 50);
    assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn twenty_simultaneous_renders_all_succeed() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let app = test_app(test_config(&volume));

    let renders: Vec<_> = (0..20)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move {
                post_json(&app, "/templates/sample/render", &sample_request())
                    .await
                    .0
            })
        })
        .collect();
    for render in renders {
        assert_eq!(render.await.unwrap(), StatusCode::OK);
    }
}

/// Ids of the listed templates.
async fn listed(app: &Router) -> Vec<String> {
    let (_, body) = get_json(app, "/templates").await;
    body["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap().to_owned())
        .collect()
}

/// SC-008: by default, nothing watches the volume; a template added after startup is not
/// listed, even once the rescan interval has passed (FR-020).
#[tokio::test(flavor = "multi_thread")]
async fn without_hot_reload_templates_are_loaded_once() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let state = test_state(Config {
        rescan_interval: Duration::from_secs(1),
        ..test_config(&volume)
    });

    let watcher = spawn_background_tasks(&state);

    assert!(watcher.is_none());
    let app = build_app(state);
    assert_eq!(listed(&app).await, ["sample"]);
    volume.copy_template(&sample_template(), "sample-copy");
    // Past the rescan interval, the debounce and the confirming observation.
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    assert_eq!(listed(&app).await, ["sample"]);
}

/// SC-008: with hot reload on, the watcher runs and an added template is listed (FR-021).
#[tokio::test(flavor = "multi_thread")]
async fn with_hot_reload_added_templates_are_listed() {
    const DEADLINE: Duration = Duration::from_secs(5);
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let state = test_state(Config {
        hot_reload: true,
        rescan_interval: Duration::from_secs(1),
        ..test_config(&volume)
    });

    let watcher = spawn_background_tasks(&state);

    assert!(watcher.is_some());
    let app = build_app(state);
    assert_eq!(listed(&app).await, ["sample"]);
    volume.copy_template(&sample_template(), "sample-copy");
    let started = Instant::now();
    while listed(&app).await != ["sample", "sample-copy"] {
        assert!(
            started.elapsed() < DEADLINE,
            "sample-copy not listed within {DEADLINE:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    drop(watcher);
}
