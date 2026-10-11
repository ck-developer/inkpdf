# Research — MCP server (004)

## R1 — MCP library

**Decision.** Use `rmcp` 3.5.1, the official Rust SDK of the Model Context Protocol, with
`default-features = false` and the features `server`, `macros` and
`transport-streamable-http-server`.

- Tools are declared with its `#[tool_router]` / `#[tool]` / `#[tool_handler]` macros. Each tool
  argument struct derives `serde::Deserialize` and `schemars::JsonSchema`, which produces the
  tool's input schema.
- The child-process, client and `reqwest` features stay off: no subprocess, no outbound HTTP
  (constitution I and IV).
- MSRV of `rmcp` 3.5.1 is 1.88; the workspace is on 1.99.

**Rationale.** The protocol keeps moving (2026-07-28 removed sessions, SEP-2567); the official
SDK tracks it, handles version negotiation with older clients, the JSON-RPC framing, the
Streamable HTTP transport and the `Origin` checks.

**Alternatives.**
- *Hand-written JSON-RPC over an axum route.* Small for three tools, but we would own protocol
  negotiation, batching rules, error codes and every future revision of the spec.
- *A separate MCP process (Node or Python) calling the HTTP API.* Second runtime in the image,
  second deployment; rejected by constitution I and VII.

## R2 — Transport and mounting

**Decision.** When `INKPDF_MCP_ENABLED=true`, `build_app` adds
`Router::nest_service("/mcp", StreamableHttpService::new(...))` to the existing router. When it
is off, nothing is mounted and `/mcp` falls through to the router's 404, like any unknown path
(FR-002).

`StreamableHttpServerConfig`:
- `legacy_session_mode = false`: stateless; every request stands alone. Required anyway by
  protocol `2026-07-28`, and it keeps the service free of state (constitution VII). Older
  clients still work: rmcp answers their `initialize` without keeping a session.
- `json_response = true`: tools answer with a single JSON body, no SSE stream to hold open.
- `cancellation_token`: a child of the service's shutdown, so graceful shutdown also stops the
  MCP service.
- Origin and Host: see R6.
- `max_request_body_bytes`: see R5.

The MCP service is outside the `utoipa` router, so the OpenAPI document and the locked contract
(`openapi/openapi.json`) do not change (FR-016).

**Alternatives.**
- *A second port for MCP.* One more thing to publish and document, no benefit.
- *Stateful sessions.* Removed from the current protocol and would add in-memory state.

## R3 — One pipeline for HTTP and MCP

**Decision.** Extract from `api/render.rs` the part that goes from a parsed JSON body to a PDF
into one function, for example:

```text
render::run(state, template_id, body: serde_json::Value, interface) -> Result<Vec<u8>, ApiError>
```

It does, in order: template lookup and status, `metadata` extraction, schema preparation and
validation (all violations together), the render log, and `render::render` (semaphore, queue
timeout, render timeout, sandbox). The HTTP handler keeps only what is HTTP-specific:
content type, body size, query parameters, headers. The MCP render tool builds the same
`{ data, layout?, metadata? }` object and calls the same function.

Template listing and details reuse the existing `TemplateSummary` / `TemplateDetail` builders
of `api/templates.rs`.

**Rationale.** FR-008 and SC-002 (identical PDF) hold by construction, and the "no duplicated
logic" requirement of the spec is met.

## R4 — Tool results

**Decision.**
- `list_templates` and `get_template` return the same JSON as `GET /templates` and
  `GET /templates/{id}` (the detail already includes the schema), as `structuredContent`, plus
  the same JSON as a text block for clients that ignore structured content.
- `render_template` returns two content blocks:
  1. text: `Rendered <file name> from template <id> (<size>).`;
  2. an embedded resource: `blob` = the PDF in base64, `mimeType: application/pdf`,
     `uri: inkpdf://templates/<id>/render/<file name>`.
- The file name follows the HTTP rule: `<filename or template id>.pdf`, same cleaning function.
- Tool annotations: all three are `readOnlyHint: true`, `idempotentHint: true`,
  `openWorldHint: false`. Rendering has no side effect: nothing is stored.

**Rationale.** An embedded resource with a MIME type is how MCP returns a file inline; clients
such as Claude can offer it as a download.

**Alternative.** *A link to download the PDF later.* Needs storage; that is feature 005 (S3).

## R5 — Errors and limits

**Decision.**
- Every `ApiError` of the pipeline becomes a **tool error** (`isError: true`), not a JSON-RPC
  error, so the assistant sees it and can react. The result carries:
  - `structuredContent`: the same problem object as the HTTP API (`type`, `title`, `status`,
    `detail`, `templateId`, `violations`);
  - a text block in plain language: the title, the detail, one line per violation
    (`<path>: <message>`), then a closing hint:
    - *fixable* (`validation-failed`, `template-not-found`, `invalid-parameter`,
      `payload-too-large`): "Fix the input and call again" (for an unknown template: "call
      `list_templates`");
    - *not fixable by the caller* (`template-invalid`, `overloaded`, `render-timeout`,
      `render-failed`): "Try again later or tell the operator".
  This is FR-009 and FR-010.
- JSON-RPC errors are left to rmcp for protocol faults only: unknown tool, arguments that do
  not match the tool's input schema.
- Size: the render tool measures the serialised `{ data, layout, metadata }` object and refuses
  it with `payload-too-large` above `INKPDF_MAX_BODY_BYTES`, like the HTTP API. The transport
  limit (`max_request_body_bytes`) is set to `INKPDF_MAX_BODY_BYTES + 64 KiB` to leave room for
  the JSON-RPC envelope, so the tool sees oversized input and reports it properly.
  **Confirmed during implementation**: rmcp's limit alone governs `/mcp`. axum's
  `DefaultBodyLimit` layer only stores the limit in the request extensions, where axum's body
  extractors (`Bytes`, `Json`) read it; rmcp's service reads the raw body against its own
  `max_request_body_bytes` and never consults it, whatever the layer order. A body above
  `INKPDF_MAX_BODY_BYTES` but within the 64 KiB headroom reaches the server; a larger one gets
  rmcp's `413` (`tests/mcp.rs`, `mcp_body_limit_leaves_room_for_the_envelope`). `/mcp` is
  mounted before the router's layers, so `TraceLayer` covers it too (R8).
- Concurrency, queue timeout and render timeout: shared by construction (R3).

## R6 — Browser origins and hosts

**Decision.**
- `enforce_origin_validation()` with an empty allow-list: any request that carries an `Origin`
  header is refused (403). MCP clients (Claude Code, desktop apps, SDKs) send no `Origin`; a web
  page always does. This blocks a malicious page, including through DNS rebinding (FR-012).
- `disable_allowed_hosts()`: the `Host` check is off. rmcp's default accepts only loopback
  hosts, which would refuse `http://inkpdf:3000/mcp` inside a Compose network or a VPC. With
  origins refused, the `Host` check adds nothing for a private service.
- No authentication (FR-013, clarification of 2026-10-11). The documentation warns against
  exposing `/mcp`, like the rest of the API, on the Internet.

**Alternatives.**
- *`INKPDF_MCP_ALLOWED_ORIGINS` / `INKPDF_MCP_ALLOWED_HOSTS` settings.* No current use case;
  can be added with the Internet-access feature.

## R7 — Configuration

**Decision.** One new variable, `INKPDF_MCP_ENABLED`, boolean, default `false`. Accepted
values: `true`, `false`, `1`, `0` (same set as the `download` query parameter); anything else
fails startup with a configuration error, like the other variables. The startup log line
`server.listening` gains `mcp: true|false`; when on, an extra line gives the MCP path.

`xtask docs check` already fails when an `INKPDF_*` variable is missing from
`docs/src/reference/configuration.md`, which keeps the reference in sync.

## R8 — Logs

**Decision.**
- The existing `render` event gains `interface = "http" | "mcp"` (FR-014). The field is added
  to HTTP renders too, so dashboards can filter both.
- Each MCP tool call logs `event = "mcp.tool"`, `tool`, `templateId` (when relevant),
  `durationMs`, `outcome` (`ok` or the error slug). No input data is logged (FR-022 of 001).
- HTTP requests to `/mcp` are also traced by the existing `TraceLayer`.

## R9 — Server instructions for the assistant

**Decision.** The `initialize` answer carries `instructions` that describe the expected flow:

> Use `list_templates` to find a template matching the user's request. Call `get_template` to
> read its schema. Ask the user for every required field you do not know; never invent
> values. Use the defaults of `layout` unless the user asks otherwise. Then call
> `render_template`. If it returns validation errors, ask the user about those fields and call
> it again.

Each tool's description repeats the part that concerns it. This drives SC-001 without any
prompt or resource feature.

## R10 — Tests

**Decision.** Integration tests in `crates/inkpdf/tests/mcp.rs`, in the existing style: build
the app with `build_app` and send raw HTTP requests with `tower::ServiceExt::oneshot`.

- Disabled by default: `POST /mcp` → 404.
- Enabled: `initialize` (protocol `2025-06-18`), then `tools/list` returns exactly the three
  tools with their input schemas.
- Stateless path (`MCP-Protocol-Version: 2026-07-28`, no `initialize`): `tools/call`.
- `list_templates` and `get_template` match `GET /templates` and `GET /templates/{id}`.
- `render_template` on every example template with its sample request: the decoded PDF is
  byte-identical to the HTTP render of the same input (SC-002; renders are deterministic).
- Error cases: missing and wrong fields (all paths listed, SC-003), unknown template, invalid
  template, payload too large, overloaded (one render slot held), render timeout.
- A request with an `Origin` header → 403.
- The OpenAPI contract test keeps passing unchanged.
- `perf.rs` gains an MCP-versus-HTTP timing comparison (SC-004, within 10 %).

Raw JSON-RPC keeps the tests on the wire contract and avoids pulling `rmcp`'s client and
`reqwest` into dev-dependencies.

## R11 — Connecting Claude

**Decision.** The documentation (`docs/src/guides/mcp.md`) covers:
- **Claude Code**, the primary client: it connects over HTTP from the user's computer, so it
  reaches a private address directly:
  `claude mcp add --transport http inkpdf http://<host>:3000/mcp`, with `--scope user` or
  `--scope project` (a `.mcp.json` entry with `"type": "http"` and `"url"`; an entry with a
  `url` and no `type` is read as a stdio server), checked with `claude mcp list` and `/mcp`.
- **Claude Desktop**, the secondary client, **through a stdio-to-HTTP bridge**, `mcp-remote`,
  started by the app from `claude_desktop_config.json`:

  ```json
  { "mcpServers": { "inkpdf": { "command": "npx",
      "args": ["-y", "mcp-remote", "http://<host>:3000/mcp", "--allow-http",
               "--transport", "http-only"] } } }
  ```

  It needs Node.js 18 or later on the user's computer.
- **MCP Inspector** and a `curl` call for operators who want to try the tools by hand.

**Resolved (2026-10-12): Claude Desktop needs a bridge.** Official sources:
- Custom connectors (Settings > Connectors, added by URL) connect "from Anthropic's cloud
  infrastructure, rather than from your local device"; servers "hosted on a private corporate
  network, behind a VPN, or blocked by a firewall won't connect". Local servers configured in
  `claude_desktop_config.json` "are a separate mechanism" and "do use your local network".
  <https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp>
- A remote server is added "by its URL as a custom connector"; the desktop app itself only
  installs local servers (desktop extensions). A server inside a private network is pointed to
  MCP tunnels. <https://claude.com/docs/connectors/custom/remote-mcp>
- `claude_desktop_config.json` declares servers the app starts, by `command` and `args`; it
  documents no `url` entry. <https://modelcontextprotocol.io/docs/develop/connect-local-servers>
- MCP tunnels, the official way for claude.ai to reach a private network, are a research
  preview for the Enterprise plan only, and require OAuth on each MCP server, so they do not
  apply to inkpdf (no authentication). <https://claude.com/docs/connectors/mcp-tunnels/overview>
- `mcp-remote` (0.14.3, checked in its source) runs as a stdio server for the app and forwards
  to a Streamable HTTP server. It refuses a non-HTTPS URL other than `localhost` or `127.0.0.1`
  unless `--allow-http` is given; `--transport http-only` skips the SSE fallback; it needs
  Node.js 18 or later, and the app runs it with the system's Node.js. Checked against inkpdf
  on 2026-10-12: driven over stdio as the app does, `mcp-remote@0.14.3 <url> --allow-http
  --transport http-only` relays `initialize` (negotiated at `2025-06-18`) and `tools/list`; a
  LAN address without `--allow-http` is refused at startup.
  <https://github.com/geelen/mcp-remote>, <https://www.npmjs.com/package/mcp-remote>
- Claude Code: `claude mcp add --transport http <name> <url>`, scopes `local` (default),
  `project` and `user`; `.mcp.json` uses `"type": "http"`. <https://code.claude.com/docs/en/mcp>

FR-015 and SC-005 of the spec say that Claude Desktop goes through this bridge.

**Alternatives.**
- *Custom connector in Claude Desktop or claude.ai.* Needs a server reachable from the Internet:
  out of scope until the access-key feature.
- *A `url` entry in `claude_desktop_config.json`.* Not documented by Anthropic or by the MCP
  documentation, so not offered.
- *A local stdio transport in inkpdf.* Rejected in the spec: inkpdf runs as a service in a
  container; the bridge is a client-side tool and needs nothing from inkpdf.

## R12 — Constitution

**Decision.** MINOR amendment 1.0.3 → 1.1.0 of principle VI, made in this feature:
"in addition to the REST API, the service MAY expose an optional MCP interface, off by default,
offering the same read-only operations (list, detail and schema, render) through the same
pipeline and limits; it is documented in the documentation site, not in the OpenAPI document."
Principles IV and VII are unchanged and apply to the MCP interface.
