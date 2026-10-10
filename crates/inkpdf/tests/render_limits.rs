//! Bounded render duration and concurrency.

mod common;

use std::time::{Duration, Instant};

use axum::http::StatusCode;
use common::*;
use inkpdf::Config;
use serde_json::json;

/// Iterations for the `slow` fixture: a few seconds of compilation.
const SLOW_ITERATIONS: u64 = 400_000;

fn volume() -> TestVolume {
    let volume = TestVolume::new();
    volume
        .copy_template(&sample_template(), "sample")
        .copy_template(&fixture("slow"), "slow");
    volume
}

fn slow_body() -> serde_json::Value {
    json!({"data": {"iterations": SLOW_ITERATIONS}})
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_render_times_out() {
    let volume = volume();
    let app = test_app(Config {
        render_timeout: Duration::from_secs(1),
        ..test_config(&volume)
    });

    let started = Instant::now();
    let (status, _, body) = post_json(&app, "/templates/slow/render", &slow_body()).await;
    let elapsed = started.elapsed();

    assert_eq!(
        status,
        StatusCode::GATEWAY_TIMEOUT,
        "{}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(problem(&body)["code"], "render-timeout");
    assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn busy_slots_give_overloaded_then_free_up() {
    let volume = volume();
    let app = test_app(Config {
        max_concurrent_renders: 1,
        queue_timeout: Duration::from_secs(1),
        render_timeout: Duration::from_secs(60),
        ..test_config(&volume)
    });

    let slow = {
        let app = app.clone();
        tokio::spawn(async move { post_json(&app, "/templates/slow/render", &slow_body()).await })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;

    let (status, _, body) = post_json(&app, "/templates/sample/render", &sample_request()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(problem(&body)["code"], "overloaded");

    let (status, _, _) = slow.await.unwrap();
    assert_eq!(status, StatusCode::OK);

    let (status, _, _) = post_json(&app, "/templates/sample/render", &sample_request()).await;
    assert_eq!(status, StatusCode::OK);
}
