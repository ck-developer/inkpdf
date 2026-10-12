//! SC-001: latency regression guard. Run in release mode by CI:
//! `cargo test --release --test perf -- --ignored`. One-page document: a title and a 20-row table.

mod common;

use std::time::{Duration, Instant};

use axum::http::StatusCode;
use common::mcp::{STATELESS_PROTOCOL, decode_pdf, mcp_app, rpc_request, tool_params};
use common::*;

const RENDERS: usize = 50;
const P95_BUDGET: Duration = Duration::from_millis(200);

#[tokio::test(flavor = "multi_thread")]
#[ignore = "performance test, run with `cargo test --release --test perf -- --ignored`"]
async fn sample_p95_is_below_200ms() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let app = test_app(test_config(&volume));
    let body = sample_request();

    // Warm-up render (fonts, caches).
    let (status, _, _) = post_json(&app, "/templates/sample/render", &body).await;
    assert_eq!(status, StatusCode::OK);

    let mut durations = Vec::with_capacity(RENDERS);
    for i in 0..RENDERS {
        // Different data on each render: the compilation cache cannot serve the whole
        // document.
        let mut body = body.clone();
        body["data"]["title"] = format!("Sample {i}").into();
        let started = Instant::now();
        let (status, _, _) = post_json(&app, "/templates/sample/render", &body).await;
        durations.push(started.elapsed());
        assert_eq!(status, StatusCode::OK);
    }
    durations.sort();
    let p95 = durations[(RENDERS * 95).div_ceil(100) - 1];
    println!("p50 = {:?}, p95 = {p95:?}", durations[RENDERS / 2]);
    assert!(p95 < P95_BUDGET, "p95 = {p95:?} exceeds {P95_BUDGET:?}");
}

/// SC-006 (002): a template using packages (QR code, amount, chart) renders under 1 s at p95.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "performance test, run with `cargo test --release --test perf -- --ignored`"]
async fn packages_demo_p95_is_below_1s() {
    const BUDGET: Duration = Duration::from_secs(1);
    let volume = TestVolume::new();
    volume.copy_template(&fixture("packages-demo"), "packages-demo");
    let app = test_app(test_config(&volume));
    let body = fixture_request("packages-demo.json");

    let (status, _, _) = post_json(&app, "/templates/packages-demo/render", &body).await;
    assert_eq!(status, StatusCode::OK);

    let mut durations = Vec::with_capacity(RENDERS);
    for i in 0..RENDERS {
        let mut body = body.clone();
        body["data"]["reference"] = format!("DEMO-2026-{i:04}").into();
        let started = Instant::now();
        let (status, _, _) = post_json(&app, "/templates/packages-demo/render", &body).await;
        durations.push(started.elapsed());
        assert_eq!(status, StatusCode::OK);
    }
    durations.sort();
    let p95 = durations[(RENDERS * 95).div_ceil(100) - 1];
    println!(
        "packages-demo: p50 = {:?}, p95 = {p95:?}",
        durations[RENDERS / 2]
    );
    assert!(p95 < BUDGET, "p95 = {p95:?} exceeds {BUDGET:?}");
}

/// SC-004 (004): rendering through MCP adds no noticeable delay: its p95 stays within 10 % of
/// the HTTP API's for the same template and input.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "performance test, run with `cargo test --release --test perf -- --ignored`"]
async fn mcp_p95_is_within_10_percent_of_http() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let app = mcp_app(&volume);
    let body = sample_request();
    let mcp_request = |body: &serde_json::Value| {
        let mut args = body.clone();
        args["templateId"] = "sample".into();
        let params = tool_params("render_template", args);
        rpc_request(1, "tools/call", params, STATELESS_PROTOCOL)
    };

    // Warm-up render on each path (fonts, caches).
    let (status, _, _) = post_json(&app, "/templates/sample/render", &body).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, _) = send(&app, mcp_request(&body)).await;
    assert_eq!(status, StatusCode::OK);

    let mut http = Vec::with_capacity(RENDERS);
    let mut mcp = Vec::with_capacity(RENDERS);
    for i in 0..RENDERS {
        // Different data on each render, and on each path: the compilation cache cannot
        // serve the whole document.
        let mut http_body = body.clone();
        http_body["data"]["title"] = format!("Sample H{i:03}").into();
        let mut mcp_body = body.clone();
        mcp_body["data"]["title"] = format!("Sample M{i:03}").into();

        // Alternate the order, so that neither path always runs on a warmer process.
        for path in if i % 2 == 0 { [0, 1] } else { [1, 0] } {
            let started = Instant::now();
            if path == 0 {
                let (status, _, _) = post_json(&app, "/templates/sample/render", &http_body).await;
                http.push(started.elapsed());
                assert_eq!(status, StatusCode::OK);
            } else {
                let (status, _, response) = send(&app, mcp_request(&mcp_body)).await;
                mcp.push(started.elapsed());
                assert_eq!(status, StatusCode::OK);
                let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
                assert_eq!(response["result"]["isError"], false, "{response}");
                assert!(decode_pdf(&response["result"]).starts_with(b"%PDF-"));
            }
        }
    }
    let p95 = |durations: &mut Vec<Duration>| {
        durations.sort();
        (
            durations[RENDERS / 2],
            durations[(RENDERS * 95).div_ceil(100) - 1],
        )
    };
    let (http_p50, http_p95) = p95(&mut http);
    let (mcp_p50, mcp_p95) = p95(&mut mcp);
    println!("http: p50 = {http_p50:?}, p95 = {http_p95:?}");
    println!("mcp:  p50 = {mcp_p50:?}, p95 = {mcp_p95:?}");
    let budget = http_p95.mul_f64(1.10);
    assert!(
        mcp_p95 <= budget,
        "MCP p95 = {mcp_p95:?} exceeds HTTP p95 + 10 % = {budget:?}"
    );
}
