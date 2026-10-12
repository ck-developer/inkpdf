//! MCP client side of the tests: raw JSON-RPC over HTTP to `/mcp`, as an MCP client sends it
//! (`specs/004-mcp-server/contracts/mcp-tools.md`). Raw requests keep the tests on the wire
//! contract, without rmcp's client.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use inkpdf::Config;
use serde_json::{Value, json};

use super::{TestVolume, send, test_app, test_config};

/// Protocol of the stateless path: no `initialize`, per-request `_meta` and headers.
pub const STATELESS_PROTOCOL: &str = "2026-07-28";

/// Last protocol with an `initialize` handshake, as sent by current clients.
pub const HANDSHAKE_PROTOCOL: &str = "2025-06-18";

/// `test_config` with the MCP interface on.
pub fn mcp_config(volume: &TestVolume) -> Config {
    Config {
        mcp_enabled: true,
        ..test_config(volume)
    }
}

/// `test_app` with the MCP interface on.
pub fn mcp_app(volume: &TestVolume) -> Router {
    test_app(mcp_config(volume))
}

/// One JSON-RPC request to `/mcp` as an MCP client sends it, with `Host: localhost:3000` and
/// no `Origin`; tests change its headers to probe the transport.
///
/// From protocol `2026-07-28` on, adds what the stateless path requires: the `Mcp-Method`
/// and `Mcp-Name` headers and the request `_meta` (protocol version, client capabilities).
pub fn rpc_request(id: u64, method: &str, params: Value, protocol_version: &str) -> Request<Body> {
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
    request
        .body(Body::from(serde_json::to_vec(&message).unwrap()))
        .unwrap()
}

/// Sends a request to `/mcp` and returns the HTTP status with the JSON body; a body that is
/// not JSON (a transport refusal) comes back as a string.
pub async fn send_rpc(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let (status, _, body) = send(app, request).await;
    let body = serde_json::from_slice(&body)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
    (status, body)
}

/// Posts one JSON-RPC request to `/mcp` (see [`rpc_request`]).
pub async fn rpc(
    app: &Router,
    id: u64,
    method: &str,
    params: Value,
    protocol_version: &str,
) -> (StatusCode, Value) {
    send_rpc(app, rpc_request(id, method, params, protocol_version)).await
}

/// `initialize` params of a client speaking `protocol_version`.
pub fn initialize_params(protocol_version: &str) -> Value {
    json!({
        "protocolVersion": protocol_version,
        "capabilities": {},
        "clientInfo": { "name": "inkpdf-tests", "version": "0" },
    })
}

/// `tools/call` params.
pub fn tool_params(name: &str, args: Value) -> Value {
    json!({ "name": name, "arguments": args })
}

/// `tools/call` on the stateless path (protocol `2026-07-28`); returns the JSON-RPC `result`.
pub async fn call_tool(app: &Router, name: &str, args: Value) -> Value {
    call_tool_with(app, name, args, STATELESS_PROTOCOL).await
}

/// `tools/call` in `protocol_version`; returns the JSON-RPC `result`.
pub async fn call_tool_with(
    app: &Router,
    name: &str,
    args: Value,
    protocol_version: &str,
) -> Value {
    let params = tool_params(name, args);
    let (status, body) = rpc(app, 1, "tools/call", params, protocol_version).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    match body.get("result") {
        Some(result) => result.clone(),
        None => panic!("no result: {body}"),
    }
}

/// PDF of a `render_template` result: the base64 `blob` of its embedded resource.
pub fn decode_pdf(result: &Value) -> Vec<u8> {
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
pub fn base64_decode(text: &str) -> Vec<u8> {
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
