# Implementation Plan: Templates that guide AI assistants

**Branch**: `005-mcp-agent-efficiency` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-mcp-agent-efficiency/spec.md`

## Summary

Templates tell assistants what each variable is and who provides it, and the repository ships the
tools an assistant needs to use and write them.

- **Roles.** An `x-role` annotation (`ask`, `fill`, `default`) next to each property's description
  in `schema.json`, checked when the template is loaded, never used to validate input.
- **Example.** An optional `example.json` (a render body) per template, checked at load, returned
  as `example` in the template detail (HTTP and `get_template`) and in the live OpenAPI document.
- **Dates.** Template schemas are validated with JSON Schema `format` assertion, so
  `format: "date"` refuses impossible dates with the usual 422 and path.
- **Guidance.** New MCP server instructions explaining roles, the example, and "use the tools, not
  local files".
- **One demo template.** `demo` replaces `sample`, `packages-demo` and `progress-invoice`. The first
  two move to test fixtures so test coverage stays identical; the invoice is deleted.
- **Skills.** A Claude Code plugin `inkpdf` in the repository (also a marketplace) bundles the MCP
  connection and two portable skills: `generate-document` and `write-template`.
- **Argument faults.** Faults in the MCP tool arguments come back as tool errors (FR-014).
- **Versions.** A template folder may hold one sub-folder per version (`facture/1.0.0/`). Reads and
  renders take an optional `version` (exact or major); the default is the highest valid version;
  every render states the version used.
- **Loading.** Templates are loaded once at startup; `INKPDF_HOT_RELOAD=true` (development, on in
  `compose.yaml`) brings back the watcher and rescan. Constitution amended to 2.0.0.

Details: [research.md](./research.md).

## Technical Context

**Language/Version**: Rust 1.99, edition 2024, unchanged.

**Primary Dependencies**: no new dependency. `jsonschema` 0.58.6 gains
`should_validate_formats(true)`; `rmcp` 3.5 unchanged.

**Storage**: none.

**Testing**:
- `cargo test --workspace`, plus new tests:
  - unit tests for role checking and for the date message;
  - loader tests for `example.json` (valid, invalid, malformed);
  - API tests for `TemplateDetail.example`, for `format` violations, and for the live OpenAPI
    example;
  - an MCP test for the instructions and the example in `get_template`;
  - `examples.rs`, which now checks the `demo` volume.
- `cargo xtask docs check` extended (R6, R7).
- `claude plugin validate` in the quickstart.

**Target Platform**: unchanged (Linux amd64/arm64 image). The plugin is consumed by Claude Code; the
skills are portable.

**Project Type**: web service (Cargo workspace) plus repository content (example template, plugin,
docs).

**Performance Goals**: format assertion adds negligible validation cost; rendering is unchanged.

**Constraints**:
- Templates without roles, example or formats behave exactly as before, except that standard
  `format` values are now asserted (R2, release note).
- `openapi/openapi.json` changes only by `TemplateDetail.example`.

**Scale/Scope**:
- About 12 changed source files: `template/schema.rs`, `registry/{loader,mod,fingerprint}.rs`,
  `api/{templates,render,live}.rs`, `render/mod.rs`, `mcp/{mod,tools,result}.rs`, `error.rs`,
  `config.rs`, `main.rs`.
- 1 new template, 2 moved to fixtures, 1 deleted.
- 1 plugin with 2 skills, about 8 documentation pages.

No open unknowns: decisions R1–R8 are in [research.md](./research.md).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | How the design complies |
|---|-----------|--------|-------------------------|
| I | Embedded Typst, no browser | ✅ | Unchanged. The plugin only configures the client side. |
| II | Self-contained template | ✅ (2.0.0) | `example.json` is one more file inside the template folder. Each version folder is self-contained, and is the Typst root of its version. Nothing is read outside it. |
| III | Two-dimension input, validated | ✅ (strengthened) | Roles annotate `data` and `layout`; the example is a two-dimension body; `format` assertion validates more before compiling. |
| IV | Security by construction | ✅ | The example is data, checked like a request. Roles are inert annotations. No new input path. |
| V | Templates on a volume, frozen binary | ✅ (2.0.0) | Loaded at startup; hot reload only with `INKPDF_HOT_RELOAD=true` (development). An invalid version or example never hides the others. |
| VI | Self-describing API (REST and optional MCP) | ✅ (2.0.0) | The example, the `version` parameter and the version stated in the response exist in REST and MCP alike. OpenAPI and the locked contract are regenerated. The MCP gets no operation that REST lacks. |
| VII | Simplicity and scope | ✅ | No new dependency, no state, no new tool. The plugin and skills are repository content, not service code. |
| Workflow | Neutral demo template; fmt, clippy, tests; docs in the same change | ✅ | `demo` is the neutral template the workflow asks for. Test coverage is kept through fixtures. Docs, API reference and skills ship with the change. |

**Re-check after Phase 1**: ✅ under constitution 2.0.0. The contracts
([template-format.md](./contracts/template-format.md), [demo-template.md](./contracts/demo-template.md),
[plugin-skills.md](./contracts/plugin-skills.md),
[versions-and-loading.md](./contracts/versions-and-loading.md)) add no write operation, no network
access and no state.

## Project Structure

### Documentation (this feature)

```text
specs/005-mcp-agent-efficiency/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/  template-format.md · demo-template.md · plugin-skills.md · versions-and-loading.md
├── checklists/requirements.md
└── tasks.md            (/speckit-tasks)
```

### Repository (after this feature)

```text
crates/inkpdf/
├── src/
│   ├── template/schema.rs      # format assertion, date message, x-role check
│   ├── registry/loader.rs      # TemplateEntry.example/version; example.json; version folders
│   ├── registry/mod.rs         # TemplateVersions, selection
│   ├── config.rs, main.rs      # INKPDF_HOT_RELOAD; watcher only when on
│   ├── error.rs                # template-version-not-found
│   ├── api/render.rs           # version query parameter, Inkpdf-Template-Version header
│   ├── api/templates.rs        # TemplateDetail.example
│   ├── api/live.rs             # request body example per template
│   └── mcp/{mod.rs,tools.rs}   # instructions, get_template description
└── tests/
    ├── fixtures/templates/{sample,packages-demo}/   # moved from examples/
    ├── fixtures/requests/{sample.json,packages-demo.json}
    ├── examples.rs             # checks the demo volume
    └── …                       # common/mod.rs paths updated; new role/example/format tests
examples/
├── templates/demo/1.0.0/       # template.json · schema.json · main.typ · example.json
└── bruno/                      # points to demo; progress invoice and package demo removed
.claude-plugin/marketplace.json
plugins/inkpdf/
├── .claude-plugin/plugin.json
├── .mcp.json
└── skills/{generate-document,write-template}/
xtask/src/docs.rs               # skill/plugin checks, packages.md for the skill, removed-template check
docs/src/                       # template-format, writing-a-template, best-practices, assistant-skills (new), mcp, …
openapi/openapi.json            # TemplateDetail.example
release-please-config.json      # also bumps plugins/inkpdf/.claude-plugin/plugin.json version
```

**Structure Decision**: the service changes stay in the existing modules. The plugin sits under
`plugins/inkpdf/`, with the marketplace file at the root, so `/plugin marketplace add
ck-developer/inkpdf` works on the repository itself. The test templates move under
`crates/inkpdf/tests/fixtures/`, next to the existing fixtures.

## Complexity Tracking

| Addition | Why Needed | Simpler Alternative Rejected Because |
|----------|------------|--------------------------------------|
| A plugin and marketplace in the repository | Installs the two skills and the MCP connection in one step, versioned with the service. | Copying skill files by hand gives no versioning and no MCP connection. |
| Format assertion for every standard format, not only `date` | A single option, with exact checks from the validator. | A date-only custom check needs more code and leaves `email` and `date-time` unchecked. |
