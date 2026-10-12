# Implementation Plan: MCP server

**Branch**: `004-mcp-server` | **Date**: 2026-10-11 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-mcp-server/spec.md`

## Summary

inkpdf gains an optional MCP interface, so that an AI assistant can find a template, learn the
data it needs, ask the user for it and render the PDF.

- **Off by default.** `INKPDF_MCP_ENABLED=true` mounts it at `/mcp`, on the same address and
  port as the HTTP API. Off, `/mcp` is a plain 404.
- **Official SDK.** `rmcp` 3.5 (Streamable HTTP, stateless, JSON responses). No second process,
  no second port.
- **Three tools.** `list_templates`, `get_template`, `render_template`. They reuse the HTTP
  API's listing, detail and render code.
- **One render pipeline.** The validation-and-render steps of the HTTP handler are extracted
  into one function that both interfaces call, so both return the same PDF and the same
  errors.
- **Results.** The PDF comes back inline as an embedded `application/pdf` resource. Errors come
  back as tool errors carrying the HTTP problem object and a plain-language hint.
- **Security.**
  - Requests carrying an `Origin` header are refused, which blocks web pages and DNS
    rebinding.
  - No authentication, private network only (clarification of 2026-10-11).
- **Docs and constitution.**
  - A new documentation page, the setting in the configuration reference and the Compose
    example.
  - A MINOR amendment of constitution principle VI.

Details: [research.md](./research.md).

## Technical Context

**Language/Version**: Rust 1.99, edition 2024, unchanged.

**Primary Dependencies**:
- New: `rmcp = { version = "3.5", default-features = false, features = ["server", "macros",
  "transport-streamable-http-server"] }`, which brings `schemars` 1.
- Existing: `axum` 0.8, `tokio`, `serde_json`, `tracing`, `tower`.
- No client, child-process or `reqwest` feature.

**Storage**: none.

**Testing**:
- `cargo test --workspace`.
- New `crates/inkpdf/tests/mcp.rs`: raw JSON-RPC over `oneshot`, see R10.
- `perf.rs` gains an MCP-versus-HTTP comparison.
- `cargo xtask docs check` and `mdbook build docs` in CI, unchanged.

**Target Platform**: unchanged (Linux amd64/arm64 image).

**Project Type**: web service (Cargo workspace, service crate `crates/inkpdf`).

**Performance Goals**: MCP render time within 10 % of the HTTP render for the same input
(SC-004). The only extra cost is base64 encoding.

**Constraints**:
- HTTP API behaviour unchanged whether MCP is on or off.
- `openapi/openapi.json` unchanged.
- Same limits as the HTTP API.
- No state.

**Scale/Scope**:
- 3 tools, 1 setting, about 4 new source files.
- 1 refactor of `api/render.rs`.
- 1 test file, 1 documentation page.

No open unknowns: decisions R1–R12 are in [research.md](./research.md).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | How the design complies |
|---|-----------|--------|-------------------------|
| I | Embedded Typst, no browser | ✅ | Rendering still goes through the embedded engine. `rmcp` is a library in the same binary; its child-process feature is off. |
| II | Self-contained template | ✅ | Unchanged. Same registry, same resolution. |
| III | Two-dimension input, validated | ✅ | `render_template` takes `data` and `layout` separately, and both go through the same schema validation before any compilation. |
| IV | Security by construction | ✅ | <ul><li>The assistant sends JSON only: no Typst code, no paths.</li><li>Data is injected as data, through the same pipeline.</li><li>Same size, time and concurrency limits.</li><li>Cross-origin browser requests are refused.</li></ul> |
| V | Hot templates, frozen binary | ✅ | Tools read the current registry snapshot, so hot reload applies. The setting is read at startup, like the others. |
| VI | Self-describing REST API | ✅ (amended, 1.1.0) | Constitution 1.1.0 (2026-10-12) allows an optional MCP interface over the same read-only operations and pipeline, documented in the docs site and covered by contract tests rather than by OpenAPI. The REST API and its OpenAPI document are unchanged. |
| VII | Simplicity and scope | ✅ | <ul><li>No authentication, no users, no state: the MCP server is stateless.</li><li>One dependency.</li><li>No new operation: read and render only.</li><li>Document storage stays out (feature 005).</li></ul> |
| Workflow | fmt, clippy, tests; docs in the same change | ✅ | Tests cover the tool contract, errors and sandbox limits through MCP. The docs page and the configuration reference ship in the same change. |

**Re-check after Phase 1**: ✅ (constitution 1.1.0).
[contracts/mcp-tools.md](./contracts/mcp-tools.md) adds no write operation, no network access
and no stored state.

## Project Structure

### Documentation (this feature)

```text
specs/004-mcp-server/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/mcp-tools.md
├── checklists/requirements.md
└── tasks.md            (/speckit-tasks)
```

### Source Code (repository root)

```text
crates/inkpdf/
├── Cargo.toml                  # + rmcp
├── src/
│   ├── config.rs               # + mcp_enabled (INKPDF_MCP_ENABLED)
│   ├── lib.rs                  # + pub mod mcp
│   ├── api/mod.rs              # build_app: nest /mcp when enabled
│   ├── main.rs                 # startup log: mcp on/off; shutdown token
│   ├── render/mod.rs           # + run(): shared validate-and-render pipeline, Interface
│   ├── api/render.rs           # handler reduced to HTTP concerns, calls render::run
│   ├── api/templates.rs        # summary/detail builders made reusable
│   └── mcp/
│       ├── mod.rs              # service construction, transport config (origins, hosts, limits)
│       ├── tools.rs            # the three tools, instructions, annotations
│       └── result.rs           # PDF → embedded resource; ApiError → tool error + hint
└── tests/
    ├── mcp.rs                  # new: tool contract, parity with HTTP, errors, origin, off by default
    └── perf.rs                 # + MCP vs HTTP timing
docs/src/
├── guides/mcp.md               # new: use with an AI assistant
├── SUMMARY.md                  # + the page
├── reference/configuration.md  # + INKPDF_MCP_ENABLED (checked by xtask)
├── operations/deployment.md    # Compose example: + INKPDF_MCP_ENABLED
└── operations/security.md      # + /mcp: private network only, origins refused
.specify/memory/constitution.md # 1.1.0: principle VI amendment
compose.yaml                    # + INKPDF_MCP_ENABLED (commented example)
```

**Structure Decision**: one new module, `src/mcp/`, in the existing service crate. A separate
crate would only make sense with a second consumer. The shared pipeline lives in `render/`,
where the engine already is, so neither interface depends on the other.

## Complexity Tracking

| Addition | Why Needed | Simpler Alternative Rejected Because |
|----------|------------|--------------------------------------|
| `rmcp` dependency | Protocol negotiation across versions, Streamable HTTP, origin checks, tool schemas. | Hand-written JSON-RPC: we would own every protocol revision (2026-07-28 already changed sessions). |
| Constitution VI amendment (MINOR) | The MCP interface is a second public interface. | Running MCP as a separate process outside the constitution: second runtime and deployment, against principles I and VII. |
