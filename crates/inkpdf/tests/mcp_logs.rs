//! MCP logs (FR-014), checked on the production JSON format.
//!
//! In a test binary of its own: tracing caches, per log call site, whether a subscriber wants
//! its events. A test running in parallel that first reaches a call site while this test
//! installs its subscriber can leave that call site disabled for it.

mod common;

use std::io;
use std::sync::{Arc, Mutex};

use common::mcp::*;
use common::*;
use serde_json::{Value, json};

/// Log lines written while `scenario` runs, as JSON objects (the production format).
async fn capture_logs<F: std::future::Future<Output = ()>>(scenario: F) -> Vec<Value> {
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);
    impl io::Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let capture = Capture::default();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_writer({
            let capture = capture.clone();
            move || capture.clone()
        })
        .finish();
    // Current-thread runtime: the service's tasks run on this thread, under this subscriber.
    let _guard = tracing::subscriber::set_default(subscriber);
    scenario.await;
    let bytes = capture.0.lock().unwrap().clone();
    String::from_utf8(bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// FR-014: one `mcp.tool` event per call, with the tool, the template id, the duration and
/// the outcome, a call refused for its arguments included; renders say which interface they
/// came through; no input value is logged.
#[tokio::test]
async fn tool_calls_are_logged_without_their_input() {
    const SECRET: &str = "Confidential title 7f3a";
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let app = mcp_app(&volume);
    let mut body = sample_request();
    body["data"]["title"] = SECRET.into();

    let render_args = |id: &str| {
        let mut args = body.clone();
        args["templateId"] = id.into();
        args
    };

    let logs = capture_logs(async {
        call_tool(&app, "list_templates", json!({})).await;
        call_tool(&app, "get_template", json!({ "templateId": "sample" })).await;
        call_tool(&app, "render_template", render_args("sample")).await;
        call_tool(&app, "render_template", render_args("unknown")).await;
        call_tool(&app, "render_template", render_args("../x")).await;
        // Refused for its arguments (protocol fault), before the tool runs.
        let mut args = render_args("sample");
        args["colour"] = SECRET.into();
        let params = tool_params("render_template", args);
        rpc(&app, 1, "tools/call", params, STATELESS_PROTOCOL).await;
        post_json(&app, "/templates/sample/render", &body).await;
    })
    .await;

    let tools: Vec<&Value> = logs.iter().filter(|l| l["event"] == "mcp.tool").collect();
    let summary: Vec<(&str, Option<&str>, &str)> = tools
        .iter()
        .map(|l| {
            assert!(l["durationMs"].is_u64(), "{l}");
            (
                l["tool"].as_str().unwrap(),
                l["templateId"].as_str(),
                l["outcome"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("list_templates", None, "ok"),
            ("get_template", Some("sample"), "ok"),
            ("render_template", Some("sample"), "ok"),
            ("render_template", Some("unknown"), "template-not-found"),
            // A malformed id is not echoed, as over HTTP.
            ("render_template", None, "template-not-found"),
            ("render_template", None, "invalid-arguments"),
        ]
    );
    let renders: Vec<&str> = logs
        .iter()
        .filter(|l| l["event"] == "render")
        .map(|l| l["interface"].as_str().unwrap())
        .collect();
    assert_eq!(renders, ["mcp", "http"]);
    for line in &logs {
        let line = line.to_string();
        assert!(!line.contains(SECRET), "input data logged: {line}");
        assert!(!line.contains("../x"), "raw template id logged: {line}");
    }
}
