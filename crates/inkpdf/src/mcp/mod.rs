//! Optional MCP interface, served at `/mcp` when `INKPDF_MCP_ENABLED=true`.
//!
//! Streamable HTTP transport, stateless, JSON responses. The tools offer the HTTP API's
//! read-only operations through the same pipeline and limits (constitution VI).

mod arguments;
mod result;
mod tools;

use std::sync::Arc;
use std::time::Instant;

use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, Implementation, ServerCapabilities,
    ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData, RoleServer, ServerHandler, tool_handler};
use tokio_util::sync::CancellationToken;
use tracing::{Level, Metadata};
use tracing_subscriber::filter::{FilterFn, filter_fn};

use crate::AppState;

/// Room left above `INKPDF_MAX_BODY_BYTES` for the JSON-RPC envelope, so that an oversized
/// input still reaches the render tool and comes back as a `payload-too-large` tool error
/// instead of a transport refusal (research R5).
const ENVELOPE_HEADROOM_BYTES: usize = 64 * 1024;

/// Flow expected from the assistant, sent in the `initialize` answer (research R4 of feature
/// 005): the tools as the only source, the variable roles and their fallback, the example.
const INSTRUCTIONS: &str = "Everything you need is available through these tools; do not look \
for templates, schemas or examples in local files. Use `list_templates` to find a template \
matching the user's request, then `get_template` to read its schema and its example. Each \
variable has a description and may have an `x-role`: `ask` — ask the user unless they already \
gave the value; `fill` — fill it in yourself (for example today's date), and ask if you cannot \
work it out; `default` — leave the default unless the user mentions it, or leave the variable \
out if it has none. Without a role, treat a required variable as `ask` and an optional one with \
a default as `default`. Never invent a value you should have asked for. Start from the \
example's structure. Then call `render_template`; if it returns validation errors, ask the user \
about those fields only and call it again. Use a template's default version unless the user \
asks for a specific one; `get_template` and `render_template` take an optional `version`.";

/// MCP server handler. The service is stateless: one handler per request, all sharing the
/// service's state (registry snapshot, render slots, configuration).
#[derive(Clone)]
pub struct InkpdfMcp {
    state: AppState,
}

impl InkpdfMcp {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Tools of [`tools`]; `tools/list` and `tools/call` come from its router.
#[tool_handler]
impl ServerHandler for InkpdfMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("inkpdf", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }

    /// The router's `tools/call`, except for arguments that do not fit the tool's input schema
    /// (unknown key, missing `templateId` or `data`, wrongly typed argument). rmcp refuses
    /// them with serde's message; they come back as a tool error naming each faulty argument,
    /// so that the assistant sees it and fixes the call (FR-014 of feature 005). Such a call is
    /// still logged, without its arguments. An unknown tool stays a JSON-RPC "invalid params"
    /// error (`-32602`), from the router.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let started = Instant::now();
        let tool = request.name.clone();
        let outline = arguments::outline(request.arguments.as_ref());
        let router = Self::tool_router();
        let response = router
            .call(ToolCallContext::new(self, request, context))
            .await?;
        match response {
            CallToolResponse::Complete(result) if is_argument_refusal(&result) => {
                // The router only gets here for one of the three tools: the name is ours.
                tools::log_call(&tool, None, None, started, "invalid-arguments");
                let input_schema = router.get(&tool).map(|tool| tool.input_schema.as_ref());
                let result = arguments::error_result(&tool, input_schema, &outline, &result);
                Ok(CallToolResponse::Complete(result))
            }
            response => Ok(response),
        }
    }
}

/// rmcp's refusal of a call's arguments: a tool error without structured content. Every tool
/// error of the tools themselves carries the HTTP problem as structured content
/// (`result::error_result`).
fn is_argument_refusal(result: &CallToolResult) -> bool {
    result.is_error == Some(true) && result.structured_content.is_none()
}

/// Global log filter that keeps only rmcp's `error` events, whatever `RUST_LOG` says, because
/// the input is never logged (FR-022 of feature 001): its `debug` and `trace` events dump
/// whole requests and responses (tool arguments, PDFs), and its `warn` events quote every
/// JSON-RPC error answered, whose message can quote an argument value (`invalid type: string
/// "…"`). This also drops the three `info` lines it writes for every request.
pub fn log_cap() -> FilterFn<fn(&Metadata<'_>) -> bool> {
    filter_fn(|metadata| {
        let target = metadata.target();
        let rmcp = target == "rmcp" || target.starts_with("rmcp::");
        !rmcp || *metadata.level() == Level::ERROR
    })
}

/// Tower service mounted at `/mcp`.
pub type McpService = StreamableHttpService<InkpdfMcp, NeverSessionManager>;

/// Builds the MCP service; `shutdown` stops it when the server shuts down.
///
/// - Stateless (no session) and JSON responses: no stream is held open, and `GET /mcp` is a
///   `405`.
/// - Every request carrying an `Origin` header is refused (`403`): MCP clients send none, a
///   web page always does, which also blocks DNS rebinding (research R6).
/// - The `Host` check is off, so that `http://inkpdf:3000/mcp` works inside a private
///   network; with origins refused it adds nothing (research R6).
/// - Body limit: `INKPDF_MAX_BODY_BYTES` plus [`ENVELOPE_HEADROOM_BYTES`]; above it, the
///   transport answers `413`.
pub fn service(state: AppState, shutdown: CancellationToken) -> McpService {
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .enforce_origin_validation()
        .disable_allowed_hosts()
        .with_max_request_body_bytes(
            state
                .config
                .max_body_bytes
                .saturating_add(ENVELOPE_HEADROOM_BYTES),
        )
        .with_cancellation_token(shutdown);
    StreamableHttpService::new(
        move || Ok(InkpdfMcp::new(state.clone())),
        Arc::new(NeverSessionManager::default()),
        config,
    )
}
