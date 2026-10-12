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

/// The `sample` and `packages-demo` fixtures, the example template `demo`, plus a template whose
/// schema does not load.
fn examples_volume() -> TestVolume {
    let volume = TestVolume::new();
    for template in ["sample", "packages-demo"] {
        volume.copy_template(&fixture(template), template);
    }
    volume.copy_template(&repo_root().join("examples/templates/demo"), "demo");
    volume.copy_template(&fixture("invalid-schema"), "invalid-schema");
    volume
}

/// The `example.json` of the `demo` template.
fn demo_example() -> Value {
    let raw = fs::read(repo_root().join("examples/templates/demo/1.0.0/example.json")).unwrap();
    serde_json::from_slice(&raw).unwrap()
}

/// Every example request with its template: `sample`, `packages-demo`, and `demo` with its
/// example as is and with every layout setting changed.
fn example_cases() -> Vec<(&'static str, Value)> {
    let mut demo_layout = demo_example();
    demo_layout["layout"] = json!({
        "accentColor": "#2e7d32",
        "font": "New Computer Modern",
        "density": "compact",
        "showTotal": false
    });
    vec![
        ("sample", fixture_request("sample.json")),
        ("packages-demo", fixture_request("packages-demo.json")),
        ("demo", demo_example()),
        ("demo", demo_layout),
    ]
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

/// US1 (005): the instructions explain the roles, their fallback and that the tools are the
/// only source; the `get_template` description announces the roles and the example.
#[tokio::test]
async fn instructions_explain_the_roles() {
    let volume = TestVolume::new();
    let app = mcp_app(&volume);

    let params = initialize_params(HANDSHAKE_PROTOCOL);
    let (_, body) = rpc(&app, 1, "initialize", params, HANDSHAKE_PROTOCOL).await;

    let instructions = body["result"]["instructions"].as_str().unwrap_or_default();
    for expected in [
        "Everything you need is available through these tools; do not look for templates, \
         schemas or examples in local files.",
        "`x-role`",
        "`ask` — ask the user unless they already gave the value",
        "`fill` — fill it in yourself",
        "`default` — leave the default unless the user mentions it",
        "Without a role, treat a required variable as `ask` and an optional one with a default \
         as `default`.",
        "Never invent a value you should have asked for.",
        "Start from the example's structure.",
        "optional `version`",
    ] {
        assert!(
            instructions.contains(expected),
            "{expected}: {instructions}"
        );
    }

    let (_, body) = rpc(&app, 2, "tools/list", Value::Null, STATELESS_PROTOCOL).await;
    let tools = body["result"]["tools"].as_array().expect("tools");
    let get = tools.iter().find(|tool| tool["name"] == "get_template");
    let description = get.unwrap()["description"].as_str().unwrap_or_default();
    for expected in ["`x-role`", "`ask`, `fill` or `default`", "`example`"] {
        assert!(description.contains(expected), "{expected}: {description}");
    }
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
    assert_eq!(keys(&get["properties"]), ["templateId", "version"], "{get}");
    assert_eq!(strings(&get["required"]), ["templateId"], "{get}");
    assert_eq!(get["properties"]["templateId"]["type"], "string", "{get}");
    assert!(get["properties"]["templateId"]["description"].is_string());
    assert_eq!(get["properties"]["version"]["type"], "string", "{get}");
    assert!(get["properties"]["version"]["description"].is_string());

    let render = input_schema(tools, "render_template");
    let properties = &render["properties"];
    assert_eq!(
        keys(properties),
        [
            "data",
            "filename",
            "layout",
            "metadata",
            "templateId",
            "version"
        ],
        "{render}"
    );
    assert_eq!(strings(&render["required"]), ["data", "templateId"]);
    assert_eq!(properties["templateId"]["type"], "string");
    assert_eq!(properties["version"]["type"], "string");
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

    for id in ["sample", "packages-demo", "demo", "invalid-schema"] {
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

/// US4 (005): `get_template` gives the template's example, which `render_template` accepts as
/// is.
#[tokio::test]
async fn get_template_gives_an_example_that_renders() {
    let volume = TestVolume::new();
    let example = json!({
        "data": { "title": "Example", "items": [{ "label": "Alpha", "value": 1.5 }] },
        "layout": { "align": "center" },
        "metadata": { "title": "Example document" }
    });
    volume
        .copy_template(&sample_template(), "sample")
        .write_file("sample/example.json", example.to_string());
    let app = mcp_app(&volume);

    let result = call_tool(&app, "get_template", json!({ "templateId": "sample" })).await;

    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(result["structuredContent"]["example"], example);
    let result = call_tool(&app, "render_template", render_args("sample", &example)).await;
    assert_eq!(result["isError"], false, "{result}");
    let (status, _, expected) = post_json(&app, "/templates/sample/render", &example).await;
    assert_eq!(status, StatusCode::OK);
    assert!(decode_pdf(&result) == expected, "the PDF differs");
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
        // The `sample` fixture's manifest gives version 1.0.0.
        summary.starts_with("Rendered sample.pdf from template sample 1.0.0 (")
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

/// US5 (005): an impossible date in a `format: date` property is a validation error on its
/// path, with the hint to fix the input, never `render-failed`. The template's Typst code
/// builds a `datetime` from the date, so it would fail to render with it.
#[tokio::test]
async fn impossible_date_is_a_validation_error() {
    let volume = TestVolume::new();
    let schema = json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "object",
                "required": ["date"],
                "properties": { "date": { "type": "string", "format": "date" } }
            }
        }
    });
    let main = "#let (y, m, d) = sys.inputs.data.date.split(\"-\").map(int)\n\
                #datetime(year: y, month: m, day: d).display()\n";
    volume
        .write_file("dated/schema.json", schema.to_string())
        .write_file("dated/main.typ", main);
    let app = mcp_app(&volume);
    let body = json!({ "data": { "date": "2026-02-30" } });
    let expected = http_problem(&app, "dated", "", &body).await;
    assert_eq!(expected["code"], "validation-failed", "{expected}");
    assert_eq!(violation_paths(&expected), ["/data/date"]);

    let result = call_tool(&app, "render_template", render_args("dated", &body)).await;

    assert_tool_error(&result, &expected, FIX_INPUT);
    assert!(
        texts(&result)[0]
            .contains("\n- /data/date: \"2026-02-30\" is not a valid calendar date (YYYY-MM-DD)\n"),
        "{result}"
    );

    let leap_day = json!({ "data": { "date": "2024-02-29" } });
    let result = call_tool(&app, "render_template", render_args("dated", &leap_day)).await;
    assert_eq!(result["isError"], false, "{result}");
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

/// Closing hint of a tool error on the arguments themselves.
const FIX_ARGUMENTS: &str = "Fix the arguments and call the tool again.";

/// FR-014 (005): faults in the arguments themselves (unknown key, missing `templateId` or
/// `data`, wrongly typed argument) are refused before the tool runs, as a tool error the
/// assistant sees: one line per faulty argument, naming it, then the hint; no structured
/// content, since no HTTP problem matches.
#[tokio::test]
async fn malformed_arguments_are_tool_errors() {
    let volume = examples_volume();
    let app = mcp_app(&volume);
    let data = sample_request()["data"].clone();
    let render_arguments =
        "render_template takes data, filename, layout, metadata, templateId, version";
    let cases = [
        (
            "render_template",
            json!({ "templateId": "sample", "data": data, "colour": "blue" }),
            vec![format!("unknown argument `colour`; {render_arguments}")],
        ),
        (
            "render_template",
            json!({ "data": data }),
            vec!["missing argument `templateId`".to_owned()],
        ),
        (
            "render_template",
            json!({ "templateId": "sample" }),
            vec!["missing argument `data`".to_owned()],
        ),
        (
            "render_template",
            json!({ "templateId": "sample", "data": [] }),
            vec!["argument `data` must be an object, not an array".to_owned()],
        ),
        (
            "render_template",
            json!({ "templateId": "sample", "data": "Confidential" }),
            vec!["argument `data` must be an object, not a string".to_owned()],
        ),
        // Every fault at once.
        (
            "render_template",
            json!({ "templateId": 7, "data": null, "colour": "blue" }),
            vec![
                format!("unknown argument `colour`; {render_arguments}"),
                "argument `data` must be an object, not null".to_owned(),
                "argument `templateId` must be a string, not a number".to_owned(),
            ],
        ),
        (
            "get_template",
            json!({}),
            vec!["missing argument `templateId`".to_owned()],
        ),
        // An argument given to `list_templates`, which takes none.
        (
            "list_templates",
            json!({ "templateId": "sample" }),
            vec!["unknown argument `templateId`; list_templates takes no arguments".to_owned()],
        ),
    ];
    for (tool, args, faults) in cases {
        for protocol in [STATELESS_PROTOCOL, HANDSHAKE_PROTOCOL] {
            let params = tool_params(tool, args.clone());
            let (status, body) = rpc(&app, 1, "tools/call", params, protocol).await;

            assert_eq!(status, StatusCode::OK, "{protocol} {args}: {body}");
            assert!(body.get("error").is_none(), "{protocol} {args}: {body}");
            let result = &body["result"];
            assert_eq!(result["isError"], true, "{protocol} {args}: {result}");
            assert!(
                result.get("structuredContent").is_none(),
                "{protocol} {args}: {result}"
            );
            let mut expected = format!("Invalid arguments for {tool}");
            for fault in &faults {
                expected.push_str(&format!("\n- {fault}"));
            }
            expected.push_str(&format!("\n{FIX_ARGUMENTS}"));
            assert_eq!(texts(result), [expected.as_str()], "{protocol} {args}");
        }
    }

    // `arguments` is optional in MCP: without it, `list_templates` runs and `render_template`
    // misses both of its required arguments.
    let params = json!({ "name": "list_templates" });
    let (status, body) = rpc(&app, 1, "tools/call", params, STATELESS_PROTOCOL).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"]["isError"], false, "{body}");
    let params = json!({ "name": "render_template" });
    let (_, body) = rpc(&app, 1, "tools/call", params, STATELESS_PROTOCOL).await;
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert_eq!(
        texts(&body["result"]),
        [format!(
            "Invalid arguments for render_template\n\
             - missing argument `data`\n\
             - missing argument `templateId`\n\
             {FIX_ARGUMENTS}"
        )
        .as_str()]
    );
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

/// The `sample` fixture copied as `facture/1.0.0/` and `facture/2.0.0/`, each naming its
/// version in the document.
fn versions_volume() -> TestVolume {
    let volume = TestVolume::new();
    for version in ["1.0.0", "2.0.0"] {
        let folder = format!("facture/{version}");
        volume.copy_template(&sample_template(), &folder);
        let manifest = json!({ "name": "Facture", "version": version });
        volume.write_file(&format!("{folder}/template.json"), manifest.to_string());
        let main = volume.path().join(&folder).join("main.typ");
        let text = fs::read_to_string(&main).unwrap();
        fs::write(main, format!("{text}\nTemplate version {version}\n")).unwrap();
    }
    volume
}

const NO_VERSION_HINT: &str = "Call get_template without a version to see the available versions.";

#[tokio::test]
async fn tools_take_an_optional_version() {
    let volume = versions_volume();
    let app = mcp_app(&volume);

    for (version, expected) in [
        (None, "2.0.0"),
        (Some("1"), "1.0.0"),
        (Some("2.0.0"), "2.0.0"),
    ] {
        let mut args = json!({ "templateId": "facture" });
        if let Some(version) = version {
            args["version"] = version.into();
        }
        let result = call_tool(&app, "get_template", args.clone()).await;
        assert_eq!(result["isError"], false, "{result}");
        let detail = &result["structuredContent"];
        assert_eq!(detail["version"], expected, "{version:?}");
        assert_eq!(detail["versions"].as_array().unwrap().len(), 2);

        let mut args = render_args("facture", &sample_request());
        if let Some(version) = version {
            args["version"] = version.into();
        }
        let result = call_tool(&app, "render_template", args).await;
        assert_eq!(result["isError"], false, "{result}");
        let summary = texts(&result)[0];
        assert!(
            summary.starts_with(&format!(
                "Rendered facture.pdf from template facture {expected} ("
            )),
            "{summary}"
        );
        let pdf = base64_decode(result["content"][1]["resource"]["blob"].as_str().unwrap());
        let text = pdf_text(&pdf);
        assert!(
            text.contains(&format!("Template version {expected}")),
            "{text}"
        );
    }
}

#[tokio::test]
async fn unknown_version_invites_to_read_the_versions() {
    let volume = versions_volume();
    let app = mcp_app(&volume);

    let expected = http_problem(&app, "facture", "?version=3", &sample_request()).await;
    assert_eq!(expected["code"], "template-version-not-found");
    assert_eq!(expected["availableVersions"], json!(["2.0.0", "1.0.0"]));
    let mut args = render_args("facture", &sample_request());
    args["version"] = "3".into();
    let result = call_tool(&app, "render_template", args).await;
    assert_tool_error(&result, &expected, NO_VERSION_HINT);

    let (_, expected) = get_json(&app, "/templates/facture?version=3").await;
    let args = json!({ "templateId": "facture", "version": "3" });
    let result = call_tool(&app, "get_template", args).await;
    assert_tool_error(&result, &expected, NO_VERSION_HINT);

    // A malformed version is an invalid parameter, as over HTTP.
    let expected = http_problem(&app, "facture", "?version=latest", &sample_request()).await;
    assert_eq!(expected["code"], "invalid-parameter");
    let mut args = render_args("facture", &sample_request());
    args["version"] = "latest".into();
    let result = call_tool(&app, "render_template", args).await;
    assert_tool_error(&result, &expected, FIX_INPUT);
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
