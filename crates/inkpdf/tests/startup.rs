//! SC-002 (startup with 50 templates) and SC-007 (20 concurrent renders).

mod common;

use std::time::{Duration, Instant};

use axum::http::StatusCode;
use common::*;
use inkpdf::AppState;

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
