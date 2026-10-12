//! MCP interface (`/mcp`): raw JSON-RPC over HTTP, against the wire contract
//! (`specs/004-mcp-server/contracts/mcp-tools.md`).

mod common;

use std::fs;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderValue, Request, StatusCode, header};
use common::mcp::*;
use common::*;
use inkpdf::error::ApiError;
use inkpdf::registry::watcher;
use inkpdf::{Config, build_app};
use serde_json::{Value, json};

/// The example templates, plus a template whose schema does not load.
fn examples_volume() -> TestVolume {
    let volume = TestVolume::new();
    for template in ["sample", "packages-demo", "progress-invoice"] {
        volume.copy_template(
            &repo_root().join("examples/templates").join(template),
            template,
        );
    }
    volume.copy_template(&fixture("invalid-schema"), "invalid-schema");
    volume
}

/// Request body from `examples/requests/`.
fn example_request(path: &str) -> Value {
    let raw = fs::read(repo_root().join("examples/requests").join(path)).unwrap();
    serde_json::from_slice(&raw).unwrap()
}

/// Every example request with its template: `sample`, `packages-demo` and each
/// `progress-invoice` request.
fn example_cases() -> Vec<(&'static str, Value)> {
    let mut cases = vec![
        ("sample", example_request("sample.json")),
        ("packages-demo", example_request("packages-demo.json")),
    ];
    let mut invoices: Vec<_> = fs::read_dir(repo_root().join("examples/requests/progress-invoice"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    invoices.sort();
    for name in invoices {
        cases.push((
            "progress-invoice",
            example_request(&format!("progress-invoice/{name}")),
        ));
    }
    cases
}

/// `render_template` arguments: an HTTP render body plus the template id.
fn render_args(template_id: &str, body: &Value) -> Value {
    let mut args = body.clone();
    args["templateId"] = template_id.into();
    args
}

/// Text of the result's text blocks, in order.
fn texts(result: &Value) -> Vec<&str> {
    result["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|block| block["type"] == "text")
        .filter_map(|block| block["text"].as_str())
        .collect()
}

/// Input schema of the tool `name` in a `tools/list` result.
fn input_schema<'a>(tools: &'a [Value], name: &str) -> &'a Value {
    let tool = tools.iter().find(|tool| tool["name"] == name);
    &tool.unwrap_or_else(|| panic!("no tool {name}"))["inputSchema"]
}

/// Sorted keys of a JSON object.
fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .into_iter()
        .flatten()
        .map(|(key, _)| key.as_str())
        .collect();
    keys.sort();
    keys
}

/// Sorted strings of a JSON array (absent: empty).
fn strings(value: &Value) -> Vec<&str> {
    let mut items: Vec<&str> = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    items.sort();
    items
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

// --- User story 1: list, read, render -------------------------------------------------------

#[tokio::test]
async fn tools_list_offers_the_three_tools() {
    let volume = TestVolume::new();
    let app = mcp_app(&volume);

    let (status, body) = rpc(&app, 1, "tools/list", Value::Null, STATELESS_PROTOCOL).await;

    assert_eq!(status, StatusCode::OK, "{body}");
    let tools = body["result"]["tools"].as_array().expect("tools");
    let mut names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    names.sort();
    assert_eq!(names, ["get_template", "list_templates", "render_template"]);
    for tool in tools {
        let name = &tool["name"];
        let description = tool["description"].as_str().unwrap_or_default();
        assert!(!description.trim().is_empty(), "{name}: no description");
        let annotations = &tool["annotations"];
        assert_eq!(annotations["readOnlyHint"], true, "{name}: {annotations}");
        assert_eq!(annotations["idempotentHint"], true, "{name}: {annotations}");
        assert_eq!(annotations["openWorldHint"], false, "{name}: {annotations}");
        let schema = &tool["inputSchema"];
        assert_eq!(schema["type"], "object", "{name}: {schema}");
        assert_eq!(schema["additionalProperties"], false, "{name}: {schema}");
    }

    let list = input_schema(tools, "list_templates");
    assert!(keys(&list["properties"]).is_empty(), "{list}");
    assert!(strings(&list["required"]).is_empty(), "{list}");

    let get = input_schema(tools, "get_template");
    assert_eq!(keys(&get["properties"]), ["templateId"], "{get}");
    assert_eq!(strings(&get["required"]), ["templateId"], "{get}");
    assert_eq!(get["properties"]["templateId"]["type"], "string", "{get}");
    assert!(get["properties"]["templateId"]["description"].is_string());

    let render = input_schema(tools, "render_template");
    let properties = &render["properties"];
    assert_eq!(
        keys(properties),
        ["data", "filename", "layout", "metadata", "templateId"],
        "{render}"
    );
    assert_eq!(strings(&render["required"]), ["data", "templateId"]);
    assert_eq!(properties["templateId"]["type"], "string");
    assert_eq!(properties["data"]["type"], "object");
    assert_eq!(properties["layout"]["type"], "object");
    assert_eq!(properties["filename"]["type"], "string");
    for name in ["data", "layout", "filename"] {
        assert!(properties[name]["description"].is_string(), "{name}");
    }
    // `metadata`: the HTTP API's DocumentMetadata schema.
    let metadata = &properties["metadata"];
    assert_eq!(metadata["type"], "object", "{metadata}");
    assert_eq!(metadata["additionalProperties"], false, "{metadata}");
    assert_eq!(
        keys(&metadata["properties"]),
        ["author", "date", "keywords", "subject", "title"],
        "{metadata}"
    );
}

#[tokio::test]
async fn list_templates_returns_the_http_listing() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let (status, expected) = get_json(&app, "/templates").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(expected["templates"].as_array().unwrap().len(), 4);

    let result = call_tool(&app, "list_templates", json!({})).await;

    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(result["structuredContent"], expected);
    let text = texts(&result);
    assert_eq!(text.len(), 1, "{result}");
    assert_eq!(serde_json::from_str::<Value>(text[0]).unwrap(), expected);
}

#[tokio::test]
async fn get_template_returns_the_http_detail() {
    let volume = examples_volume();
    let app = mcp_app(&volume);

    for id in [
        "sample",
        "packages-demo",
        "progress-invoice",
        "invalid-schema",
    ] {
        let (status, expected) = get_json(&app, &format!("/templates/{id}")).await;
        assert_eq!(status, StatusCode::OK, "{id}");

        let result = call_tool(&app, "get_template", json!({ "templateId": id })).await;

        assert_eq!(result["isError"], false, "{id}: {result}");
        assert_eq!(result["structuredContent"], expected, "{id}");
        let text = texts(&result);
        assert_eq!(text.len(), 1, "{id}: {result}");
        assert_eq!(serde_json::from_str::<Value>(text[0]).unwrap(), expected);
    }
}

#[tokio::test]
async fn empty_volume_lists_no_template() {
    let volume = TestVolume::new();
    let app = mcp_app(&volume);

    let result = call_tool(&app, "list_templates", json!({})).await;

    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(result["structuredContent"], json!({ "templates": [] }));
    assert!(
        texts(&result).contains(&"No template is loaded."),
        "{result}"
    );
}

/// SC-002: for every example request, the same PDF as the HTTP API, byte for byte.
#[tokio::test]
async fn render_template_returns_the_http_pdf() {
    let volume = examples_volume();
    let app = mcp_app(&volume);

    for (id, body) in example_cases() {
        let (status, _, expected) =
            post_json(&app, &format!("/templates/{id}/render"), &body).await;
        assert_eq!(status, StatusCode::OK, "{id}");

        let result = call_tool(&app, "render_template", render_args(id, &body)).await;

        assert_eq!(result["isError"], false, "{id}: {result}");
        let pdf = decode_pdf(&result);
        assert!(
            pdf == expected,
            "{id}: the PDF differs from the HTTP render"
        );
    }
}

/// Clients of protocol `2025-06-18` take rmcp's other request path: same results, a success
/// and a tool error.
#[tokio::test]
async fn tool_calls_answer_the_same_on_the_handshake_protocol() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let body = sample_request();
    let (status, _, expected) = post_json(&app, "/templates/sample/render", &body).await;
    assert_eq!(status, StatusCode::OK);

    for id in ["sample", "unknown"] {
        let args = render_args(id, &body);
        let stateless = call_tool(&app, "render_template", args.clone()).await;

        let result = call_tool_with(&app, "render_template", args, HANDSHAKE_PROTOCOL).await;

        for key in ["content", "isError", "structuredContent"] {
            assert_eq!(result.get(key), stateless.get(key), "{id}: {key}");
        }
        if id == "sample" {
            assert_eq!(result["isError"], false, "{result}");
            assert!(decode_pdf(&result) == expected, "the PDF differs");
        } else {
            assert_eq!(result["isError"], true, "{result}");
            assert_eq!(result["structuredContent"]["code"], "template-not-found");
        }
    }
}

#[tokio::test]
async fn render_result_carries_a_summary_and_the_pdf() {
    let volume = examples_volume();
    let app = mcp_app(&volume);

    let result = call_tool(
        &app,
        "render_template",
        render_args("sample", &sample_request()),
    )
    .await;

    assert_eq!(result["isError"], false, "{result}");
    let content = result["content"].as_array().unwrap();
    assert_eq!(content.len(), 2, "{result}");
    assert_eq!(content[0]["type"], "text");
    let summary = content[0]["text"].as_str().unwrap();
    assert!(
        summary.starts_with("Rendered sample.pdf from template sample (")
            && summary.ends_with(" KiB)."),
        "{summary}"
    );
    assert_eq!(content[1]["type"], "resource");
    let resource = &content[1]["resource"];
    assert_eq!(resource["mimeType"], "application/pdf");
    assert_eq!(
        resource["uri"],
        "inkpdf://templates/sample/render/sample.pdf"
    );
    let pdf = decode_pdf(&result);
    assert!(pdf.starts_with(b"%PDF-"));
    let size = format!("({:.1} KiB)", pdf.len() as f64 / 1024.0);
    assert!(summary.contains(&size), "{summary}, expected {size}");
}

#[tokio::test]
async fn render_options_follow_the_http_api() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let render = |args: Value| {
        let app = app.clone();
        async move {
            let result = call_tool(&app, "render_template", args).await;
            assert_eq!(result["isError"], false, "{result}");
            result
        }
    };
    let http = |body: Value| {
        let app = app.clone();
        async move {
            let (status, _, pdf) = post_json(&app, "/templates/sample/render", &body).await;
            assert_eq!(status, StatusCode::OK);
            pdf.to_vec()
        }
    };

    // `filename`: cleaned like the HTTP query parameter; `.pdf` appended.
    let mut args = render_args("sample", &sample_request());
    args["filename"] = "certificate".into();
    let result = render(args.clone()).await;
    assert!(texts(&result)[0].starts_with("Rendered certificate.pdf "));
    assert_eq!(
        result["content"][1]["resource"]["uri"],
        "inkpdf://templates/sample/render/certificate.pdf"
    );
    args["filename"] = "../Invoice 042.pdf".into();
    let result = render(args).await;
    assert!(texts(&result)[0].starts_with("Rendered Invoice 042.pdf "));
    assert_eq!(
        result["content"][1]["resource"]["uri"],
        "inkpdf://templates/sample/render/Invoice%20042.pdf"
    );

    // `layout` omitted: its defaults, as over HTTP; given: applied, as over HTTP.
    let without_layout = render(render_args("sample", &sample_request())).await;
    assert!(decode_pdf(&without_layout) == http(sample_request()).await);
    let mut centered = sample_request();
    centered["layout"] = json!({ "align": "center" });
    let with_layout = decode_pdf(&render(render_args("sample", &centered)).await);
    assert!(with_layout == http(centered).await);
    assert!(with_layout != decode_pdf(&without_layout));

    // `metadata`: reaches the PDF, as over HTTP.
    let mut titled = sample_request();
    titled["metadata"] = json!({ "title": "Employer certificate", "date": "2026-10-01" });
    let pdf = decode_pdf(&render(render_args("sample", &titled)).await);
    assert!(String::from_utf8_lossy(&pdf).contains("Employer certificate"));
    assert!(pdf == http(titled).await);
}

// --- User story 2: readable errors ----------------------------------------------------------

/// Closing hints of the contract's error table.
const FIX_INPUT: &str = "Fix the input and call render_template again.";
const LIST_TEMPLATES: &str = "Call list_templates to see the available templates.";
const OPERATOR_FIXES: &str = "This template cannot be used until the operator fixes it.";
const TRY_AGAIN: &str = "Try again in a moment.";
const TELL_OPERATOR: &str = "Tell the operator; the template failed to render.";

/// Problem body of the HTTP render of `body`; `path_id` is the template id as it appears in
/// the URL, `query` starts with `?` when present.
async fn http_problem(app: &Router, path_id: &str, query: &str, body: &Value) -> Value {
    let uri = format!("/templates/{path_id}/render{query}");
    let (status, _, bytes) = post_json(app, &uri, body).await;
    assert!(!status.is_success(), "{uri}: {status}");
    problem(&bytes)
}

/// A tool error carrying the HTTP problem `expected`: `isError`, the problem as
/// `structuredContent`, a text ending with `hint`, and no PDF.
fn assert_tool_error(result: &Value, expected: &Value, hint: &str) {
    assert_eq!(result["isError"], true, "{result}");
    assert_eq!(result["structuredContent"], *expected, "{result}");
    let text = texts(result).join("\n");
    let title = expected["title"].as_str().unwrap();
    assert!(text.starts_with(title), "{text}");
    assert!(text.ends_with(hint), "{text}");
    let blocks = result["content"].as_array().unwrap();
    assert!(blocks.iter().all(|b| b["type"] == "text"), "{result}");
}

/// Paths of a problem's violations.
fn violation_paths(problem: &Value) -> Vec<&str> {
    problem["violations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v["path"].as_str())
        .collect()
}

/// SC-003: every faulty field the HTTP API names, all at once, with a hint to fix the input.
#[tokio::test]
async fn validation_errors_name_every_faulty_field() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let mut body = sample_request();
    body["data"].as_object_mut().unwrap().remove("title");
    body["layout"] = json!({ "align": "justify" });
    let expected = http_problem(&app, "sample", "", &body).await;
    assert_eq!(expected["code"], "validation-failed");
    let paths = violation_paths(&expected);
    assert!(
        paths.contains(&"/data") && paths.contains(&"/layout/align"),
        "{paths:?}"
    );

    let result = call_tool(&app, "render_template", render_args("sample", &body)).await;

    assert_tool_error(&result, &expected, FIX_INPUT);
    let text = texts(&result).join("\n");
    for path in paths {
        assert!(
            text.contains(&format!("\n- {path}: ")),
            "{path} missing: {text}"
        );
    }
    assert!(
        text.trim_end_matches('.')
            .ends_with("call render_template again"),
        "{text}"
    );
}

#[tokio::test]
async fn wrong_metadata_date_is_a_validation_error() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let mut body = sample_request();
    body["metadata"] = json!({ "date": "01/10/2026" });
    let expected = http_problem(&app, "sample", "", &body).await;
    assert_eq!(expected["code"], "validation-failed");
    assert_eq!(violation_paths(&expected), ["/metadata/date"]);

    let result = call_tool(&app, "render_template", render_args("sample", &body)).await;

    assert_tool_error(&result, &expected, FIX_INPUT);
    assert!(texts(&result)[0].contains("\n- /metadata/date: "));
}

/// A `layout` or `metadata` that is `null` or not an object reaches the shared pipeline as it
/// came, and is refused with the HTTP API's problem, next to every other faulty field.
#[tokio::test]
async fn non_object_layout_or_metadata_is_refused_as_over_http() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    for (key, value) in [
        ("layout", Value::Null),
        ("layout", json!("big")),
        ("metadata", Value::Null),
        ("metadata", json!("x")),
    ] {
        let mut body = sample_request();
        body[key] = value.clone();
        body["data"]["items"] = json!("none");
        let expected = http_problem(&app, "sample", "", &body).await;
        assert_eq!(expected["code"], "validation-failed", "{key}: {value}");
        let paths = violation_paths(&expected);
        assert!(
            paths.contains(&format!("/{key}").as_str()) && paths.contains(&"/data/items"),
            "{key}: {value}: {paths:?}"
        );

        let result = call_tool(&app, "render_template", render_args("sample", &body)).await;

        assert_tool_error(&result, &expected, FIX_INPUT);
    }
}

#[tokio::test]
async fn too_long_filename_is_an_invalid_parameter() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let name = "x".repeat(201);
    let expected = http_problem(
        &app,
        "sample",
        &format!("?filename={name}"),
        &sample_request(),
    )
    .await;
    assert_eq!(expected["code"], "invalid-parameter");

    let mut args = render_args("sample", &sample_request());
    args["filename"] = name.into();
    let result = call_tool(&app, "render_template", args).await;

    assert_tool_error(&result, &expected, FIX_INPUT);
}

/// Faults in the arguments themselves (unknown key, missing `templateId` or `data`, `data`
/// not an object) are refused before the tool runs, as a JSON-RPC "invalid params" error
/// (`-32602`), not as a tool result (contract, "Protocol faults"). The message names the
/// faulty argument.
#[tokio::test]
async fn malformed_arguments_are_refused_before_the_tool_runs() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let data = sample_request()["data"].clone();
    let cases = [
        (
            json!({ "templateId": "sample", "data": data, "colour": "blue" }),
            "colour",
        ),
        (json!({ "data": data }), "templateId"),
        (json!({ "templateId": "sample" }), "data"),
        (
            json!({ "templateId": "sample", "data": [] }),
            "expected a map",
        ),
    ];
    for (args, mention) in cases {
        for protocol in [STATELESS_PROTOCOL, HANDSHAKE_PROTOCOL] {
            let params = tool_params("render_template", args.clone());
            let (status, body) = rpc(&app, 1, "tools/call", params, protocol).await;

            // The transport answers a JSON-RPC error with HTTP 200.
            assert_eq!(status, StatusCode::OK, "{protocol} {args}: {body}");
            assert!(body.get("result").is_none(), "{protocol} {args}: {body}");
            assert_eq!(body["error"]["code"], -32602, "{protocol} {args}: {body}");
            let message = body["error"]["message"].as_str().unwrap_or_default();
            assert!(message.contains(mention), "{protocol} {args}: {message}");
        }
    }

    // An argument given to `list_templates`, which takes none.
    let params = tool_params("list_templates", json!({ "templateId": "sample" }));
    let (_, body) = rpc(&app, 1, "tools/call", params, STATELESS_PROTOCOL).await;
    assert_eq!(body["error"]["code"], -32602, "{body}");
}

#[tokio::test]
async fn unknown_tool_is_a_protocol_error() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let params = tool_params("delete_template", json!({ "templateId": "sample" }));

    let (status, body) = rpc(&app, 1, "tools/call", params, STATELESS_PROTOCOL).await;

    // JSON-RPC "invalid params" (-32602), which the transport answers with HTTP 200.
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.get("result").is_none(), "{body}");
    assert_eq!(body["error"]["code"], -32602, "{body}");
}

/// Unknown or malformed ids: `template-not-found`, from both tools, as over HTTP (a malformed
/// id is not echoed in the problem).
#[tokio::test]
async fn unknown_template_invites_to_list_the_templates() {
    let volume = examples_volume();
    let app = mcp_app(&volume);

    for (id, path_id) in [("unknown", "unknown"), ("../x", "..%2Fx")] {
        let expected = http_problem(&app, path_id, "", &sample_request()).await;
        assert_eq!(expected["code"], "template-not-found", "{id}");
        let result = call_tool(&app, "render_template", render_args(id, &sample_request())).await;
        assert_tool_error(&result, &expected, LIST_TEMPLATES);

        let (status, expected) = get_json(&app, &format!("/templates/{path_id}")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
        let result = call_tool(&app, "get_template", json!({ "templateId": id })).await;
        assert_tool_error(&result, &expected, LIST_TEMPLATES);
    }
}

#[tokio::test]
async fn invalid_template_cannot_be_used() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let expected = http_problem(&app, "invalid-schema", "", &json!({ "data": {} })).await;
    assert_eq!(expected["code"], "template-invalid");

    let args = json!({ "templateId": "invalid-schema", "data": {} });
    let result = call_tool(&app, "render_template", args).await;

    assert_tool_error(&result, &expected, OPERATOR_FIXES);
}

#[tokio::test]
async fn typst_error_is_a_render_failure() {
    let volume = TestVolume::new();
    volume.copy_template(&fixture("broken-typst"), "broken-typst");
    let app = mcp_app(&volume);
    let expected = http_problem(&app, "broken-typst", "", &json!({ "data": {} })).await;
    assert_eq!(expected["code"], "render-failed");

    let args = json!({ "templateId": "broken-typst", "data": {} });
    let result = call_tool(&app, "render_template", args).await;

    assert_tool_error(&result, &expected, TELL_OPERATOR);
    // The diagnostics, with their position, for the operator.
    let diagnostic = &expected["diagnostics"][0];
    let line = format!(
        "\n- {}:{}:{}: {}",
        diagnostic["file"].as_str().unwrap(),
        diagnostic["line"],
        diagnostic["column"],
        diagnostic["message"].as_str().unwrap()
    );
    assert!(texts(&result)[0].contains(&line), "{result}");
}

/// The input is measured like an HTTP body; too large, it is a tool error, not a transport
/// refusal.
#[tokio::test]
async fn oversized_input_is_a_tool_error() {
    let volume = examples_volume();
    let app = test_app(Config {
        max_body_bytes: 1024,
        ..mcp_config(&volume)
    });
    let mut body = sample_request();
    body["data"]["title"] = "x".repeat(2048).into();
    let expected = http_problem(&app, "sample", "", &body).await;
    assert_eq!(expected["code"], "payload-too-large");

    let result = call_tool(&app, "render_template", render_args("sample", &body)).await;

    assert_tool_error(&result, &expected, FIX_INPUT);
}

/// Problem the HTTP API returns for `error` on template `id`.
fn expected_problem(error: ApiError, id: &str) -> Value {
    serde_json::to_value(error.into_problem(Some(id.into()))).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn busy_slots_give_overloaded() {
    let volume = TestVolume::new();
    volume
        .copy_template(&sample_template(), "sample")
        .copy_template(&fixture("slow"), "slow");
    let app = test_app(Config {
        max_concurrent_renders: 1,
        queue_timeout: Duration::from_secs(1),
        render_timeout: Duration::from_secs(60),
        ..mcp_config(&volume)
    });
    let slow = {
        let app = app.clone();
        tokio::spawn(async move { post_json(&app, "/templates/slow/render", &slow_body()).await })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;

    let result = call_tool(
        &app,
        "render_template",
        render_args("sample", &sample_request()),
    )
    .await;

    assert_tool_error(
        &result,
        &expected_problem(ApiError::Overloaded, "sample"),
        TRY_AGAIN,
    );
    let (status, _, _) = slow.await.unwrap();
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_render_times_out() {
    let volume = TestVolume::new();
    volume.copy_template(&fixture("slow"), "slow");
    let app = test_app(Config {
        render_timeout: Duration::from_secs(1),
        ..mcp_config(&volume)
    });

    let started = Instant::now();
    let result = call_tool(&app, "render_template", render_args("slow", &slow_body())).await;

    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    let expected = expected_problem(ApiError::RenderTimeout, "slow");
    assert_tool_error(&result, &expected, TRY_AGAIN);
}

/// Body of the `slow` fixture: a few seconds of compilation (`tests/render_limits.rs`).
fn slow_body() -> Value {
    json!({ "data": { "iterations": 400_000 } })
}

/// Constitution IV through MCP: no read outside the template, no package the service does
/// not offer, data rendered as data.
#[tokio::test]
async fn sandbox_holds_through_mcp() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    for id in ["evil-parent", "evil-absolute", "evil-package"] {
        volume.copy_template(&fixture(id), id);
    }
    let app = mcp_app(&volume);
    let empty = json!({ "data": {} });

    for id in ["evil-parent", "evil-absolute"] {
        let expected = http_problem(&app, id, "", &empty).await;
        assert_eq!(expected["code"], "render-failed", "{id}");
        let result = call_tool(&app, "render_template", render_args(id, &empty)).await;
        assert_tool_error(&result, &expected, TELL_OPERATOR);
        // `/etc/passwd` starts with the `root` account; none of it leaks.
        assert!(!result.to_string().contains("root:"), "{id}: {result}");
    }

    let expected = http_problem(&app, "evil-package", "", &empty).await;
    assert_eq!(expected["code"], "template-invalid");
    let result = call_tool(&app, "render_template", render_args("evil-package", &empty)).await;
    assert_tool_error(&result, &expected, OPERATOR_FIXES);

    let injected = "#import \"/etc/passwd\"";
    let body = json!({ "data": { "title": injected, "items": [{ "label": "a", "value": 1 }] } });
    let result = call_tool(&app, "render_template", render_args("sample", &body)).await;
    assert_eq!(result["isError"], false, "{result}");
    let text = pdf_text(&decode_pdf(&result));
    assert!(
        text.contains("#import") && text.contains("/etc/passwd"),
        "{text}"
    );
}

// --- User story 3: turning MCP on -----------------------------------------------------------

/// SC-006: off by default, `/mcp` answers like any unknown path.
#[tokio::test]
async fn mcp_is_off_by_default() {
    let volume = TestVolume::new();
    let app = test_app(test_config(&volume));
    let initialize = || {
        let params = initialize_params(HANDSHAKE_PROTOCOL);
        rpc_request(1, "initialize", params, HANDSHAKE_PROTOCOL)
    };

    let (status, headers, body) = send(&app, initialize()).await;
    let (unknown_status, unknown_headers, unknown_body) =
        post_json(&app, "/does-not-exist", &json!({})).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        (status, headers.get(header::CONTENT_TYPE), body),
        (
            unknown_status,
            unknown_headers.get(header::CONTENT_TYPE),
            unknown_body
        )
    );
}

/// FR-012 and research R6: a request from a web page (any `Origin`) is refused; the `Host`
/// of a private network is accepted.
#[tokio::test]
async fn browser_origins_are_refused_and_private_hosts_accepted() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let app = mcp_app(&volume);
    let initialize = || {
        let params = initialize_params(HANDSHAKE_PROTOCOL);
        rpc_request(1, "initialize", params, HANDSHAKE_PROTOCOL)
    };

    let mut request = initialize();
    request.headers_mut().insert(
        header::ORIGIN,
        HeaderValue::from_static("https://evil.example"),
    );
    let (status, body) = send_rpc(&app, request).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // A Compose service name, without `Origin`, on both protocol paths.
    let mut request = initialize();
    request
        .headers_mut()
        .insert(header::HOST, HeaderValue::from_static("inkpdf:3000"));
    let (status, body) = send_rpc(&app, request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["serverInfo"]["name"], "inkpdf");

    let params = tool_params("list_templates", json!({}));
    let mut request = rpc_request(2, "tools/call", params, STATELESS_PROTOCOL);
    request
        .headers_mut()
        .insert(header::HOST, HeaderValue::from_static("inkpdf:3000"));
    let (status, body) = send_rpc(&app, request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["result"]["structuredContent"]["templates"][0]["id"],
        "sample"
    );
}

/// FR-002: the MCP server answers at `/mcp` only; a path below it is unknown.
#[tokio::test]
async fn paths_below_mcp_are_unknown() {
    let volume = TestVolume::new();
    let app = mcp_app(&volume);
    for path in ["/mcp/", "/mcp/tools"] {
        let mut request = rpc_request(1, "tools/list", Value::Null, STATELESS_PROTOCOL);
        *request.uri_mut() = path.parse().unwrap();

        let (status, body) = send_rpc(&app, request).await;

        assert_eq!(status, StatusCode::NOT_FOUND, "{path}: {body}");
    }
}

/// Stateless with JSON responses: no stream for server-initiated messages.
#[tokio::test]
async fn get_on_mcp_is_not_allowed() {
    let volume = TestVolume::new();
    let app = mcp_app(&volume);
    let request = Request::get("/mcp")
        .header(header::HOST, "localhost:3000")
        .header(header::ACCEPT, "text/event-stream")
        .header("mcp-protocol-version", HANDSHAKE_PROTOCOL)
        .body(Body::empty())
        .unwrap();

    let (status, headers, _) = send(&app, request).await;

    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    let allow = headers.get(header::ALLOW).and_then(|v| v.to_str().ok());
    assert!(allow.is_some_and(|v| v.contains("POST")), "{headers:?}");
}

/// FR-003 and FR-016: with MCP on, the OpenAPI document is still the committed contract
/// (same comparison as `tests/contract_openapi.rs`).
#[tokio::test]
async fn openapi_document_is_unchanged_with_mcp_on() {
    let volume = TestVolume::new();
    let app = mcp_app(&volume);

    let (status, mut generated) = get_json(&app, "/openapi.json").await;

    assert_eq!(status, StatusCode::OK);
    let versioned = fs::read_to_string(repo_root().join("openapi/openapi.json")).unwrap();
    let mut versioned: Value = serde_json::from_str(&versioned).unwrap();
    generated["info"]["version"] = "0.0.0".into();
    versioned["info"]["version"] = "0.0.0".into();
    assert!(
        versioned == generated,
        "the OpenAPI document changed with MCP on"
    );
}

/// US3 scenario 4: a template added to the volume is listed without a restart (rescan as in
/// `tests/hot_reload.rs`).
#[tokio::test(flavor = "multi_thread")]
async fn added_template_is_listed_without_restart() {
    const DEADLINE: Duration = Duration::from_secs(5);
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let state = test_state(Config {
        rescan_interval: Duration::from_secs(1),
        ..mcp_config(&volume)
    });
    let _watcher = watcher::spawn(state.registry.clone(), state.config.rescan_interval);
    let app = build_app(state);
    let listed = || async {
        let result = call_tool(&app, "list_templates", json!({})).await;
        result["structuredContent"]["templates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(listed().await, ["sample"]);

    volume.copy_template(&sample_template(), "sample-copy");

    let started = Instant::now();
    while listed().await != ["sample", "sample-copy"] {
        assert!(
            started.elapsed() < DEADLINE,
            "sample-copy not listed within {DEADLINE:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
