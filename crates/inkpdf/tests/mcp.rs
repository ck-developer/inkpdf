//! MCP interface (`/mcp`): raw JSON-RPC over HTTP, against the wire contract
//! (`specs/004-mcp-server/contracts/mcp-tools.md`).

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use inkpdf::Config;
use serde_json::{Value, json};

/// Protocol of the stateless path: no `initialize`, per-request `_meta` and headers.
const STATELESS_PROTOCOL: &str = "2026-07-28";

/// Last protocol with an `initialize` handshake, as sent by current clients.
const HANDSHAKE_PROTOCOL: &str = "2025-06-18";

/// `test_config` with the MCP interface on.
fn mcp_config(volume: &TestVolume) -> Config {
    Config {
        mcp_enabled: true,
        ..test_config(volume)
    }
}

/// `test_app` with the MCP interface on.
fn mcp_app(volume: &TestVolume) -> Router {
    test_app(mcp_config(volume))
}

/// Posts one JSON-RPC request to `/mcp` as an MCP client does, and returns the HTTP status
/// with the JSON body; a body that is not JSON (a transport refusal) comes back as a string.
///
/// From protocol `2026-07-28` on, adds what the stateless path requires: the `Mcp-Method`
/// and `Mcp-Name` headers and the request `_meta` (protocol version, client capabilities).
async fn rpc(
    app: &Router,
    id: u64,
    method: &str,
    params: Value,
    protocol_version: &str,
) -> (StatusCode, Value) {
    let mut params = params;
    let mut request = Request::post("/mcp")
        // rmcp needs a `Host` header even with the host check off; hyper always sets one.
        .header("host", "localhost:3000")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("mcp-protocol-version", protocol_version);
    if protocol_version >= STATELESS_PROTOCOL {
        request = request.header("mcp-method", method);
        if let Some(name) = params.get("name").and_then(Value::as_str) {
            request = request.header("mcp-name", name);
        }
        if params.is_null() {
            params = json!({});
        }
        params["_meta"] = json!({
            "io.modelcontextprotocol/protocolVersion": protocol_version,
            "io.modelcontextprotocol/clientCapabilities": {},
            "io.modelcontextprotocol/clientInfo": { "name": "inkpdf-tests", "version": "0" },
        });
    }
    let mut message = json!({ "jsonrpc": "2.0", "id": id, "method": method });
    if !params.is_null() {
        message["params"] = params;
    }
    let request = request
        .body(Body::from(serde_json::to_vec(&message).unwrap()))
        .unwrap();
    let (status, _, body) = send(app, request).await;
    let body = serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
    (status, body)
}

/// `initialize` params of a client speaking `protocol_version`.
fn initialize_params(protocol_version: &str) -> Value {
    json!({
        "protocolVersion": protocol_version,
        "capabilities": {},
        "clientInfo": { "name": "inkpdf-tests", "version": "0" },
    })
}

/// `tools/call` on the stateless path (protocol `2026-07-28`); returns the JSON-RPC `result`.
#[allow(dead_code, reason = "used by the tool tests")]
async fn call_tool(app: &Router, name: &str, args: Value) -> Value {
    let params = json!({ "name": name, "arguments": args });
    let (status, body) = rpc(app, 1, "tools/call", params, STATELESS_PROTOCOL).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    match body.get("result") {
        Some(result) => result.clone(),
        None => panic!("no result: {body}"),
    }
}

/// PDF of a `render_template` result: the base64 `blob` of its embedded resource.
#[allow(dead_code, reason = "used by the render tests")]
fn decode_pdf(result: &Value) -> Vec<u8> {
    let blob = result["content"]
        .as_array()
        .into_iter()
        .flatten()
        .find_map(|block| block["resource"]["blob"].as_str());
    match blob {
        Some(blob) => base64_decode(blob),
        None => panic!("no embedded resource: {result}"),
    }
}

/// Standard base64 with padding (RFC 4648), enough for the tests.
fn base64_decode(text: &str) -> Vec<u8> {
    fn sextet(c: u8) -> u32 {
        match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("invalid base64 character {:?}", c as char),
        }
    }
    let text = text.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    for chunk in text.chunks(4) {
        let bits = chunk.iter().fold(0, |acc, &c| acc << 6 | sextet(c)) << (6 * (4 - chunk.len()));
        let bytes = [(bits >> 16) as u8, (bits >> 8) as u8, bits as u8];
        out.extend_from_slice(&bytes[..chunk.len().saturating_sub(1)]);
    }
    out
}

#[test]
fn base64_helper_decodes_every_padding() {
    assert_eq!(base64_decode("aW5rcGRm"), b"inkpdf");
    assert_eq!(base64_decode("aW5rcGQ="), b"inkpd");
    assert_eq!(base64_decode("aW5rcA=="), b"inkp");
    assert_eq!(base64_decode("JVBERi0="), b"%PDF-");
}

#[tokio::test]
async fn initialize_describes_the_server() {
    let volume = TestVolume::new();
    let app = mcp_app(&volume);

    let params = initialize_params(HANDSHAKE_PROTOCOL);
    let (status, body) = rpc(&app, 1, "initialize", params, HANDSHAKE_PROTOCOL).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let result = &body["result"];
    assert_eq!(result["protocolVersion"], HANDSHAKE_PROTOCOL);
    assert_eq!(result["serverInfo"]["name"], "inkpdf");
    assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    // Tools only, without `listChanged`.
    assert_eq!(result["capabilities"], json!({ "tools": {} }), "{result}");
    let instructions = result["instructions"].as_str().unwrap_or_default();
    assert!(instructions.contains("list_templates"), "{instructions}");
}

/// Research R5: axum's `DefaultBodyLimit` does not apply to `/mcp`; rmcp's limit,
/// `max_body_bytes` + 64 KiB, does.
#[tokio::test]
async fn mcp_body_limit_leaves_room_for_the_envelope() {
    let volume = TestVolume::new();
    let app = test_app(Config {
        max_body_bytes: 1024,
        ..mcp_config(&volume)
    });
    let padded = |size: usize| {
        let mut params = initialize_params(HANDSHAKE_PROTOCOL);
        params["clientInfo"]["description"] = json!("x".repeat(size));
        params
    };

    // Above the HTTP API's limit, within the headroom: reaches the server.
    let (status, body) = rpc(&app, 1, "initialize", padded(8 * 1024), HANDSHAKE_PROTOCOL).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["serverInfo"]["name"], "inkpdf");

    // Above the headroom: refused by the transport, with rmcp's limit.
    let (status, body) = rpc(&app, 2, "initialize", padded(65 * 1024), HANDSHAKE_PROTOCOL).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{body}");
    let limit = (1024 + 64 * 1024).to_string();
    assert!(
        body.as_str().is_some_and(|text| text.contains(&limit)),
        "{body}"
    );
}
