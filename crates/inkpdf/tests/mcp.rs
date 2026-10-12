//! MCP interface (`/mcp`): raw JSON-RPC over HTTP, against the wire contract
//! (`specs/004-mcp-server/contracts/mcp-tools.md`).

mod common;

use std::fs;

use axum::http::StatusCode;
use common::mcp::*;
use common::*;
use inkpdf::Config;
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
