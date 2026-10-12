---

description: "Task list for feature 004: MCP server"
---

# Tasks: MCP server

**Input**: Design documents from `specs/004-mcp-server/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/mcp-tools.md](./contracts/mcp-tools.md),
[quickstart.md](./quickstart.md)

**Tests**: INCLUDED. The constitution requires contract tests, end-to-end rendering and sandbox
guarantees, and SC-002, SC-003, SC-004 and SC-006 need automated checks. In each story, tests are
written first and MUST fail before the implementation.

**Organization**: one phase per user story, in priority order: US1 (P1), US2 (P1), US3 (P2). The
shared render pipeline and the MCP service skeleton are foundational.

**Cross-cutting reminders**:
- Everything is written in English: code, docs, commit messages.
- One new dependency only (`rmcp`), without its client, child-process or `reqwest` features.
- `openapi/openapi.json` MUST stay byte-identical; the HTTP API behaves exactly as before.
- No input data in logs (FR-022 of feature 001).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelisable (different files, no dependency on an unfinished task)
- **[Story]**: US1, US2 or US3
- Paths are relative to the repository root.

---

## Phase 1: Setup

- [X] T001 Amend the constitution with `/speckit-constitution` in `.specify/memory/constitution.md`: version 1.0.3 → 1.1.0 (MINOR).
  - Principle VI allows an optional MCP interface, off by default, offering the same read-only operations (list, detail and schema, render) through the same pipeline and limits, documented in the documentation site and covered by contract tests rather than by the OpenAPI document (research R12).
  - The workflow rules require MCP contract tests when the interface exists, and split the "public API → OpenAPI" rule between the REST API (OpenAPI and documentation) and the MCP interface (documentation and contract tests).
  - The Sync Impact Report is updated.
- [X] T002 Add `rmcp = { version = "3.5", default-features = false, features = ["server", "macros", "transport-streamable-http-server"] }` to `crates/inkpdf/Cargo.toml` and update `Cargo.lock`. Check with `cargo tree -p inkpdf -e normal` that neither `reqwest` nor `tokio/process` is pulled in.

---

## Phase 2: Foundational (blocks every story)

**Purpose**: one render pipeline for both interfaces, the setting, and an MCP service that answers `initialize`.

- [X] T003 [P] Add `mcp_enabled: bool` (default `false`) to `Config` in `crates/inkpdf/src/config.rs`, read from `INKPDF_MCP_ENABLED` in `from_lookup`. Accept `true`, `false`, `1`, `0`; any other value returns a `ConfigError` naming the variable. Add unit tests in the same file for the default, the four accepted values and a rejected value.
- [X] T004 Extract the shared pipeline into `crates/inkpdf/src/render/mod.rs`: `pub async fn run(state: &AppState, template_id: &str, body: serde_json::Value, interface: Interface) -> Result<Vec<u8>, ApiError>` with `pub enum Interface { Http, Mcp }`. It moves from `api/render.rs`, in order:
  - template id parsing and lookup (`template-not-found`);
  - status check (`template-invalid`);
  - `metadata` extraction and schema preparation, with all violations merged (`validation-failed`);
  - the `render` log event, now with `interface = "http" | "mcp"`;
  - the call to `render::render`.

  Keep `ApiError::for_template` on every error.
- [X] T005 Reduce `render_template` in `crates/inkpdf/src/api/render.rs` to HTTP concerns: query parsing, content type, body size, JSON parsing, then `render::run(.., Interface::Http)` and the response headers. Make the file-name cleaning (`clean_filename` and the `<name or id>.pdf` rule of `content_disposition`) a `pub(crate) fn pdf_file_name(id, filename) -> String` so MCP reuses it. `cargo test --workspace` MUST pass with no test changed.
- [X] T006 [P] In `crates/inkpdf/src/api/templates.rs`, make `find` `pub(crate)` and add `pub(crate) fn template_list(state: &AppState) -> TemplateList`, used by the `list_templates` handler, so MCP returns the same JSON. No behaviour change.
- [X] T007 Create `crates/inkpdf/src/mcp/mod.rs` and declare `pub mod mcp` in `crates/inkpdf/src/lib.rs`:
  - `struct InkpdfMcp { state: AppState }` implementing `rmcp::ServerHandler`:
    - `serverInfo` name `inkpdf` with the crate version;
    - `tools` capability only;
    - the `instructions` text of research R9.
  - `pub fn service(state: AppState, shutdown: CancellationToken) -> StreamableHttpService<…>` with `StreamableHttpServerConfig`:
    - `legacy_session_mode = false`, `json_response = true`;
    - `enforce_origin_validation()` with no allowed origin;
    - `disable_allowed_hosts()`;
    - `max_request_body_bytes = config.max_body_bytes + 64 * 1024`;
    - the shutdown token as `cancellation_token`.
- [X] T008 Mount the service in `build_app` (`crates/inkpdf/src/api/mod.rs`) with `nest_service("/mcp", mcp::service(..))` only when `config.mcp_enabled`.
  - **Resolve the open point of research R5.** Check whether `DefaultBodyLimit` applies to the nested service. Order the layers so that rmcp's limit governs `/mcp` and the HTTP routes keep theirs. Record the outcome in a code comment.
  - Add the shutdown `CancellationToken` to `AppState` (or pass it to `build_app`) and cancel it in `shutdown_signal` in `crates/inkpdf/src/main.rs`.
- [X] T009 Create `crates/inkpdf/tests/mcp.rs` with helpers in the file:
  - `mcp_app(volume)`, which is `test_app` with `mcp_enabled: true`;
  - `rpc(app, id, method, params, protocol_version) -> (StatusCode, Value)`, which posts JSON-RPC with `accept: application/json, text/event-stream`;
  - `call_tool(app, name, args) -> Value`, which returns the `result` of `tools/call` under protocol `2026-07-28`;
  - `decode_pdf(result) -> Vec<u8>`, which base64-decodes the embedded resource.

  First test: `initialize` with protocol `2025-06-18` returns `serverInfo.name == "inkpdf"`, a `tools` capability and non-empty `instructions`.

**Checkpoint**: existing tests green, HTTP behaviour unchanged, `/mcp` answers `initialize` when enabled.

---

## Phase 3: User Story 1 — Generate a document by asking an assistant (P1) 🎯 MVP

**Goal**: an assistant lists the templates, reads one template's schema and gets the rendered PDF back.

**Independent Test**: with MCP on and the example templates loaded, `tools/list` shows the three tools, and `render_template` on each example returns the same PDF as the HTTP API.

### Tests (write first, must fail)

- [X] T010 [P] [US1] In `crates/inkpdf/tests/mcp.rs`, test `tools/list`:
  - it returns exactly `list_templates`, `get_template` and `render_template`, each with a non-empty description;
  - their input schemas match [contracts/mcp-tools.md](./contracts/mcp-tools.md) (required `templateId`; required `data`; `additionalProperties: false`);
  - every tool has the annotations `readOnlyHint`, `idempotentHint` set to `true` and `openWorldHint` set to `false`.
- [X] T011 [P] [US1] In `crates/inkpdf/tests/mcp.rs`, test the reading tools against the HTTP API, using the `examples/templates` volume plus the `invalid-schema` fixture:
  - `list_templates`: its `structuredContent` equals the body of `GET /templates`, and its text block parses to the same JSON;
  - `get_template`: for each template, its `structuredContent` equals the body of `GET /templates/{id}`.
  - With an empty volume, `list_templates` returns an empty list and its text says "No template is loaded." (spec edge case).
- [X] T012 [P] [US1] In `crates/inkpdf/tests/mcp.rs`, test `render_template` (SC-002):
  - **Same PDF as HTTP.** For `sample` (`examples/requests/sample.json`), `packages-demo` (`examples/requests/packages-demo.json`) and `progress-invoice` (a request from `examples/requests/progress-invoice/`), the decoded PDF is byte-identical to `POST /templates/{id}/render` with the same body.
  - **Shape of the result.**
    - `isError` is false.
    - Block 1 is text naming the file name and the template.
    - Block 2 is a resource with `mimeType: application/pdf` and `uri: inkpdf://templates/<id>/render/<id>.pdf`.
  - **Options.**
    - `filename: "certificate"` gives `certificate.pdf`.
    - Omitting `layout` gives the same PDF as the HTTP call without `layout`.
    - `metadata.title` reaches the PDF (`pdf_text` or the PDF info).

### Implementation

- [X] T013 [US1] Create `crates/inkpdf/src/mcp/tools.rs` with `#[tool_router]` on `InkpdfMcp`, and `#[tool_handler]` on its `ServerHandler` in `mcp/mod.rs`:
  - `list_templates()`: `structuredContent` = `template_list(state)`, plus the same JSON as a text block; an empty list adds "No template is loaded.".
  - `get_template(GetTemplateArgs { template_id })` (`#[serde(rename = "templateId")]`, `deny_unknown_fields`): `find`, then `TemplateDetail::from`, as structured and text content.

  Use the descriptions and annotations of the contract.
- [X] T014 [US1] In `crates/inkpdf/src/mcp/tools.rs`, add `render_template(RenderTemplateArgs { template_id, data, layout, metadata, filename })`:
  - the struct uses `deny_unknown_fields` and `schemars`; `data` is a JSON object; `metadata` is an object described by `render::metadata::schema()` through a `schemars` override;
  - the tool builds `{ data, layout?, metadata? }` and calls `render::run(.., Interface::Mcp)`.
- [X] T015 [US1] Create `crates/inkpdf/src/mcp/result.rs` with `pdf_result(template_id, file_name, bytes) -> CallToolResult`:
  - the text block reads `Rendered <file> from template <id> (<size in KiB, one decimal>).`;
  - the embedded resource is `ResourceContents::blob(base64, uri)` with the `application/pdf` MIME type;
  - the file name comes from `pdf_file_name` (T005).

  Wire it into `render_template`.
- [X] T016 [US1] Log every tool call in `crates/inkpdf/src/mcp/tools.rs`: `event = "mcp.tool"`, `tool`, `templateId` when present, `durationMs`, `outcome` (`ok` or the error code). No argument values.
- [X] T017 [US1] In `crates/inkpdf/tests/perf.rs`, add a test that renders `sample` 50 times through MCP and 50 times through HTTP, and asserts that MCP's p95 is within 10 % of HTTP's (SC-004). Mark it `#[ignore]` like the existing perf tests, so that it runs in release mode with `cargo test --release --test perf -- --ignored`, and follow their warm-up and varying data.

**Checkpoint**: T010–T012 pass; an MCP client can list, read and render.

---

## Phase 4: User Story 2 — Recover from incomplete or wrong data (P1)

**Goal**: every failure reaches the assistant as a readable tool error, with every faulty field and a hint on what to do.

**Independent Test**: a render with a missing required field and a wrong enum value returns `isError: true`, naming both paths, and no PDF.

### Tests (write first, must fail)

- [X] T018 [P] [US2] In `crates/inkpdf/tests/mcp.rs`, test the validation errors (SC-003):
  - **Several faulty fields.** For `sample`, a body missing a required `data` field and holding an out-of-enum `layout` value gives a result where:
    - `isError` is true;
    - `structuredContent` equals the HTTP problem body for the same input (`code`, `status`, `violations`);
    - the text lists every violation path and ends with "call render_template again".
  - **Wrong `metadata.date`.** It is reported the same way.
  - **Invalid `filename`.** It gives `invalid-parameter`.
  - **Unknown argument key** (for example `{ templateId, data, colour }`). It is refused as a JSON-RPC "invalid params" error, not as a tool result, as stated in the contract.
- [X] T019 [P] [US2] In `crates/inkpdf/tests/mcp.rs`, test the other errors, each comparing `structuredContent` with the HTTP problem and checking the hint of the contract:
  - unknown template id, and a traversal id (`../x`): `template-not-found`, with a hint to call `list_templates`;
  - the `invalid-schema` fixture: `template-invalid`;
  - the `broken-typst` fixture: `render-failed`;
  - `data` larger than `max_body_bytes`: `payload-too-large`, as a tool error, not a transport refusal;
  - one slot held and a short `queue_timeout`: `overloaded` (reuse the technique of `tests/render_limits.rs`);
  - the slow template of `tests/render_limits.rs` with a 1 s `render_timeout`: `render-timeout`;
  - `tools/call` with an unknown tool name: a JSON-RPC error, not a tool result;
  - sandbox through MCP (constitution, workflow tests), with the fixtures of `tests/sandbox.rs`: `evil-parent` and `evil-absolute` give `render-failed` and no file content; `evil-package` gives the same error as through HTTP; Typst code in `data` for `sample` is rendered literally (`pdf_text` contains `#import`).

### Implementation

- [X] T020 [US2] In `crates/inkpdf/src/mcp/result.rs`, add `error_result(error: ApiError) -> CallToolResult`:
  - `isError: true`;
  - `structuredContent` = the same problem JSON as the HTTP response (reuse the serialisation of `ApiError`/`Problem` in `crates/inkpdf/src/error.rs`, making it `pub(crate)` if needed);
  - a text block made of the title, the detail, one `- <path>: <message>` line per violation, and the closing hint per error code from the contract's table.

  Use it in all three tools.
- [X] T021 [US2] In `render_template` (`crates/inkpdf/src/mcp/tools.rs`), measure the serialised `{ data, layout, metadata }` before calling `render::run`. Above `config.max_body_bytes`, return `error_result(ApiError::PayloadTooLarge { limit })`. Validate `filename` with the same rule as the HTTP query parameter, which returns `invalid-parameter`.

**Checkpoint**: T018–T019 pass; US1 tests still pass.

---

## Phase 5: User Story 3 — Turn the MCP server on and connect an assistant (P2)

**Goal**: one setting turns MCP on; the HTTP API is unaffected; the documentation is enough to connect Claude.

**Independent Test**: without the setting `/mcp` is a 404; with it, an MCP client connects by following `docs/src/guides/mcp.md` only.

### Tests (write first, must fail)

- [X] T022 [P] [US3] In `crates/inkpdf/tests/mcp.rs`, test activation and security:
  - **Default config.** `POST /mcp` returns `404`, like `POST /does-not-exist`.
  - **Enabled.** A request with `Origin: https://evil.example` returns `403`. A request with `Host: inkpdf:3000` and no `Origin` succeeds.
  - **No stream.** With MCP on, `GET /mcp` returns `405`, as stated in the contract. If `rmcp` answers differently, align the contract with the observed, standard-conformant behaviour.
  - **HTTP API unaffected.** With MCP on, `GET /openapi.json` without templates equals `openapi/openapi.json` (reuse the comparison of `tests/contract_openapi.rs`).
- [X] T023 [P] [US3] In `crates/inkpdf/tests/mcp.rs`, test hot reload (US3 scenario 4). Start with the `sample` template, call `list_templates`, copy a second template into the `TestVolume`, rescan the way `tests/hot_reload.rs` does, then call `list_templates` again: the new template is listed.

### Implementation

- [X] T024 [US3] In `crates/inkpdf/src/main.rs`:
  - add `mcp = config.mcp_enabled` to the `server.listening` log event;
  - when MCP is on, log `event = "mcp.listening"` with `path = "/mcp"`.
- [X] T025 [P] [US3] Write `docs/src/guides/mcp.md` and add it to `docs/src/SUMMARY.md` under Guides ("Using inkpdf from an AI assistant"). The page covers:
  - what MCP is for, in two sentences;
  - turning it on (`INKPDF_MCP_ENABLED=true`);
  - the three tools and their results;
  - errors and hints;
  - limits (shared with the HTTP API, large PDFs);
  - the private-network warning;
  - a worked conversation that produces a document;
  - connecting **Claude Code** (`claude mcp add --transport http inkpdf http://<host>:3000/mcp`) as the primary client;
  - connecting **Claude Desktop**. **Resolve the open point of research R11** first: check the current Claude Desktop documentation to see whether `claude_desktop_config.json` takes a remote HTTP server directly or needs a stdio bridge (`mcp-remote`). Document what works, and update R11. If Desktop needs a bridge, also update FR-015 and SC-005 in `specs/004-mcp-server/spec.md` to say so.
- [X] T026 [P] [US3] Document `INKPDF_MCP_ENABLED` in `docs/src/reference/configuration.md`, and run `cargo xtask docs check`, which must pass. Add the variable, commented, to the Compose example in `docs/src/operations/deployment.md` and to `compose.yaml` (`INKPDF_MCP_ENABLED: "${INKPDF_MCP_ENABLED:-false}"`).
- [X] T027 [P] [US3] In `docs/src/operations/security.md`, add an MCP section:
  - no authentication;
  - private network only, never exposed on the Internet;
  - requests with an `Origin` header refused;
  - the `Host` check is off, and why;
  - same sandbox and limits as the HTTP API;
  - access from the Internet is a planned feature.

**Checkpoint**: T022–T023 pass; the guide is enough to connect Claude Code.

---

## Phase 6: Polish

- [X] T028 [P] Add one line on the optional MCP server to the feature list in `README.md`. Mention `src/mcp/` and the shared `render::run` pipeline in `docs/src/contributing/architecture.md`.
- [ ] T029 [P] Update the user's project memory note `repo-layout-003.md` (outside the repository) on the layout (`src/mcp/`, `INKPDF_MCP_ENABLED`, MCP tests in `tests/mcp.rs`).
- [ ] T030 Run [quickstart.md](./quickstart.md) §1–§6 end to end:
  - `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, perf;
  - `docker compose up --build` with `INKPDF_MCP_ENABLED=true`;
  - the curl checks;
  - a real conversation in Claude Code producing `progress-invoice`, including one corrected mistake;
  - the logs of that conversation (FR-014): one `mcp.tool` event per tool call with `tool`, `templateId`, `durationMs` and `outcome`; `render` events carry `interface = "mcp"`, HTTP renders `interface = "http"`; no input value appears;
  - `cargo xtask docs check` and `mdbook build docs`.

  Fix anything stale.

  **Automated checks passed** (2026-10-12); the conversation in Claude Code and its logs remain:
  - `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean; `cargo test --workspace` passes, existing tests unchanged.
  - `cargo test --release --test perf -- --ignored` passes; on a quiet machine, p95 was 3.54 ms over HTTP and 3.66 ms over MCP.
  - `cargo xtask docs check` passes; `mdbook build docs` (mdBook 0.5.4, as in CI) builds the MCP page.
  - §1–§3 against the release binary with `examples/templates`:
    - off: `POST /mcp` gives `404`, like an unknown path, and `server.listening` logs `mcp: false`;
    - on: `mcp: true` and `mcp.listening` with `path: "/mcp"`; `tools/list` returns the three tools;
    - `render_template` on `sample` returns the same PDF bytes as `POST /templates/sample/render`;
    - `Origin: https://evil.example` gives `403`; `GET /mcp` gives `405`;
    - the log has one `mcp.tool` event per call, `interface` on `render`, and no input value.
  - `INKPDF_MCP_ENABLED=true docker compose up --build -d`: the container is healthy and logs `mcp: true` and `mcp.listening`. `tools/list` returns the three tools, `render_template` on `sample` succeeds, `Origin` gives `403`, and a request without the `accept` header gives `406`.
- [ ] T031 Mark `specs/004-mcp-server/spec.md` as implemented once merged. Write the PR description in English: what changes, how to turn it on, the constitution amendment, and the confirmed answers to R5 and R11.

---

## Dependencies & Execution Order

- **Phase 1**: T001 (constitution) comes before any code. T002 comes before T007.
- **Phase 2** blocks every story:
  - T003 has no dependency;
  - T004 → T005;
  - T006 has no dependency;
  - T007 needs T002;
  - T008 needs T003 and T007;
  - T009 needs T008.
- **US1 (Phase 3)**: after Phase 2.
  - T013 needs T006.
  - T014 needs T004.
  - T015 needs T005.
  - T016 comes after T013–T015.
  - T017 comes after T014.
- **US2 (Phase 4)**: after Phase 2. T020 changes `mcp/result.rs` and the tools, so do it after T013–T015, or merge carefully if run in parallel.
- **US3 (Phase 5)**: the tests need Phase 2 only. T023 needs T013. The docs tasks (T025–T027) can start once the contract is stable, which it is.
- **Polish**: last.

### Parallel Opportunities

- Phase 2: T003 ∥ T004 ∥ T006 (different files).
- US1 tests: T010 ∥ T011 ∥ T012 (same file, different test functions; write them together).
- US2 tests: T018 ∥ T019.
- US3: T025 ∥ T026 ∥ T027 (docs), and they can run alongside US1 and US2 implementation.
- Polish: T028 ∥ T029.

### Parallel Example: User Story 3

```text
Task: "T025 Write docs/src/guides/mcp.md and add it to docs/src/SUMMARY.md"
Task: "T026 Document INKPDF_MCP_ENABLED in docs/src/reference/configuration.md, deployment.md, compose.yaml"
Task: "T027 Add the MCP section to docs/src/operations/security.md"
```

## Implementation Strategy

1. **MVP**: Phase 1, Phase 2 and US1. Listing, schema and rendering already make a full conversation possible.
2. Then US2 (readable errors), which makes the conversation robust when the assistant gets data wrong.
3. Then US3 (logs, docs, Compose), which makes it usable by others.
4. Commit per phase, with conventional English messages (`feat(mcp): …`, `refactor(render): …`, `docs(mcp): …`). One PR for the feature, on branch `004-mcp-server`.

## Notes

- Two open points are settled during implementation and recorded in `research.md`:
  - R5, which body limit governs `/mcp` (T008);
  - R11, how Claude Desktop reaches a private server (T025).
- The refactor T004–T005 changes no HTTP behaviour; the existing test suite is the safety net and must stay untouched.
- Feature 005 (S3 storage, `generate`, document ids) will add a `generate_document` tool; nothing here should anticipate it.
