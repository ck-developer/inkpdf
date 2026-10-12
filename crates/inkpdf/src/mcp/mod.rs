//! Optional MCP interface, served at `/mcp` when `INKPDF_MCP_ENABLED=true`.
//!
//! Streamable HTTP transport, stateless, JSON responses. The tools offer the HTTP API's
//! read-only operations through the same pipeline and limits (constitution VI).

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

use crate::AppState;

/// Room left above `INKPDF_MAX_BODY_BYTES` for the JSON-RPC envelope, so that an oversized
/// input still reaches the render tool and comes back as a `payload-too-large` tool error
/// instead of a transport refusal (research R5).
const ENVELOPE_HEADROOM_BYTES: usize = 64 * 1024;

/// Flow expected from the assistant, sent in the `initialize` answer (research R9).
const INSTRUCTIONS: &str = "Use `list_templates` to find a template matching the user's \
request. Call `get_template` to read its schema. Ask the user for every required field you do \
not know; never invent values. Use the defaults of `layout` unless the user asks otherwise. \
Then call `render_template`. If it returns validation errors, ask the user about those fields \
and call it again.";

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
    /// (unknown key, missing `templateId` or `data`, `data` not an object). rmcp returns them
    /// as a tool error; the contract makes them a JSON-RPC "invalid params" error (`-32602`),
    /// with rmcp's message. Such a call is still logged, without its arguments (FR-014).
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let started = Instant::now();
        let tool = request.name.clone();
        let response = Self::tool_router()
            .call(ToolCallContext::new(self, request, context))
            .await?;
        match response {
            CallToolResponse::Complete(result) if is_argument_refusal(&result) => {
                // The router only gets here for one of the three tools: the name is ours.
                tools::log_call(&tool, None, started, "invalid-arguments");
                let message = result
                    .content
                    .iter()
                    .find_map(|block| block.as_text())
                    .map(|text| text.text.clone())
                    .unwrap_or_else(|| "invalid arguments".into());
                Err(ErrorData::invalid_params(message, None))
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
