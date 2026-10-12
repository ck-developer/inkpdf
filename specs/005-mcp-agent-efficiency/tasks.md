---

description: "Task list for feature 005: templates that guide AI assistants"
---

# Tasks: Templates that guide AI assistants

**Input**: Design documents from `specs/005-mcp-agent-efficiency/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: INCLUDED where the service changes. The constitution requires schema-validation and
API-contract tests. SC-002 to SC-006 need automated checks. In each story, tests are written
first and MUST fail before the implementation.

**Organization**: one phase per user story: US1 and US2 (P1) first, then the P2 stories (US3, US4,
US5, US7, US8, US6), ordered by dependency. Moving the test templates to fixtures is foundational. Some stories depend on others: see
Dependencies.

**Cross-cutting reminders**:
- Everything written in the repository is in English: code, docs, skills, commit messages.
- Do not add new runtime dependencies.
- Templates without roles, example or `format` must behave as before; the only exception is
  asserted standard formats (research R2).
- Regenerate the OpenAPI contract with `INKPDF_UPDATE_OPENAPI=1 cargo test -p inkpdf --test
  contract_openapi`, and the docs with `cargo xtask docs generate`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelisable (different files, no dependency on an unfinished task)
- **[Story]**: US1 to US6
- Paths are relative to the repository root.

---

## Phase 1: Setup

- [X] T001 [P] Create the plugin skeleton as in [contracts/plugin-skills.md](./contracts/plugin-skills.md):
  - `.claude-plugin/marketplace.json`;
  - `plugins/inkpdf/.claude-plugin/plugin.json`, with `userConfig.server_url` defaulting to `http://localhost:3000`, and `version` equal to the crate version in `crates/inkpdf/Cargo.toml`;
  - `plugins/inkpdf/.mcp.json`, with an `http` server `inkpdf` at `${user_config.server_url}/mcp`;
  - empty `plugins/inkpdf/skills/generate-document/` and `plugins/inkpdf/skills/write-template/` folders.

  Run `claude plugin validate plugins/inkpdf` if Claude Code is available.
- [X] T002 [P] Add `plugins/inkpdf/.claude-plugin/plugin.json` (JSON path `$.version`) to release-please's `extra-files` in `release-please-config.json`. Release-please resolves those paths relative to the package path `crates/inkpdf` (research R7).
  - Check from release-please's documentation and source which path form reaches a file outside the package.
  - If none does, drop the change, keep the plugin version independent, and record the decision in `specs/005-mcp-agent-efficiency/research.md` (R7, "Version").

---

## Phase 2: Foundational — test templates become fixtures (blocks US6)

- [ ] T003 With `git mv`, move:
  - `examples/templates/sample` and `examples/templates/packages-demo` to `crates/inkpdf/tests/fixtures/templates/`;
  - `examples/requests/sample.json` and `examples/requests/packages-demo.json` to `crates/inkpdf/tests/fixtures/requests/`.

  Then update every path that pointed to them:
  - `crates/inkpdf/tests/common/mod.rs` (`sample_template`, `sample_request`);
  - `crates/inkpdf/benches/render.rs`;
  - the tests that build paths themselves (`grep -rn "examples/" crates/inkpdf/tests crates/inkpdf/benches`).

  `cargo test --workspace` MUST pass with every assertion unchanged. Only path helpers may change.

**Checkpoint**: tests green, `examples/templates/` holds only `progress-invoice` until US6.

---

## Phase 3: User Story 1 — The assistant knows what each variable is and who provides it (P1) 🎯 MVP

**Goal**: `x-role` is checked at load and explained to assistants.

**Independent Test**: a template with a valid `x-role` everywhere loads, one with `"x-role": "maybe"` is listed as invalid with the pointer, and the MCP instructions explain the roles.

### Tests (write first, must fail)

- [ ] T004 [P] [US1] Unit tests in `crates/inkpdf/src/template/schema.rs`:
  - valid roles (`ask`, `fill`, `default`) under `data`, `layout`, nested `properties`, `items`, `$defs` and `oneOf` are accepted;
  - `"maybe"`, `""` and `1` are refused with `invalid role "<value>" at <pointer>; expected "ask", "fill" or "default"`;
  - an input is validated the same way with or without roles.
- [ ] T005 [P] [US1] API test in `crates/inkpdf/tests/api_templates.rs`: a volume with one template whose schema has `"x-role": "maybe"` and one valid template. `GET /templates` lists the first as `invalid`, with a reason naming the pointer, and the second as `valid`.
- [ ] T006 [P] [US1] MCP test in `crates/inkpdf/tests/mcp.rs`:
  - `initialize` instructions contain `x-role`, `ask`, `fill`, `default`, the fallback rule and the sentence about not reading local files;
  - the `get_template` tool description mentions `x-role` and `example`.

### Implementation

- [ ] T007 [US1] In `TemplateSchema::load` (`crates/inkpdf/src/template/schema.rs`), walk the schema and check every `x-role` value. Cover `properties`, `items`, `prefixItems`, `additionalProperties`, `$defs`, `allOf`, `anyOf`, `oneOf`, `if`, `then`, `else`, `not`. Report the first invalid role with its JSON pointer (research R1).
- [ ] T008 [US1] Replace the server instructions in `crates/inkpdf/src/mcp/mod.rs` with the text of research R4, including its last sentence: "Use a template's default version unless the user asks for a specific one; `get_template` and `render_template` take an optional `version`." Update the `get_template` description in `crates/inkpdf/src/mcp/tools.rs` as in [contracts/template-format.md](./contracts/template-format.md).
- [ ] T009 [P] [US1] Document `x-role`:
  - in `docs/src/reference/template-format.md`: values, meaning, where allowed, fallback, load-time error;
  - in `docs/src/guides/mcp.md`: the new instructions and how assistants use roles.

**Checkpoint**: T004–T006 pass.

---

## Phase 4: User Story 2 — A generic skill to generate documents (P1)

**Goal**: the `generate-document` skill, installable with the plugin.

**Independent Test**: `cargo xtask docs check` validates the skill and plugin files; with the plugin installed, an assistant follows the skill on the `demo` template (manual, T046).

- [ ] T010 [US2] Write `plugins/inkpdf/skills/generate-document/SKILL.md` as in [contracts/plugin-skills.md](./contracts/plugin-skills.md).
  - Frontmatter: `name`, `description`, `license` and `metadata` only.
  - Body: the six steps, the role rules with their fallback, "ask rather than invent", and the handling of validation errors versus other errors.
  - No template id, field name or business rule. Under 150 lines.
- [ ] T011 [P] [US2] Extend `cargo xtask docs check` in `xtask/src/docs.rs`, with unit tests:
  - every `plugins/inkpdf/skills/*/SKILL.md` has a frontmatter using only `name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`, and has `name` and `description`;
  - `.claude-plugin/marketplace.json` and `plugins/inkpdf/.claude-plugin/plugin.json` parse;
  - `plugin.json`'s `version` equals the crate version, unless T002 decided otherwise.
- [ ] T012 [P] [US2] Write `docs/src/guides/assistant-skills.md` and add it to `docs/src/SUMMARY.md` under Guides. The page covers:
  - what the two skills do;
  - installing the plugin in Claude Code (`/plugin marketplace add ck-developer/inkpdf`, `/plugin install inkpdf@inkpdf`, `server_url`);
  - uploading the skills to claude.ai or Claude Desktop: how to build the zip of a skill folder
    (one command), and that the MCP connection is then made separately;
  - **one MCP connection only**: either the plugin's or `claude mcp add`, never both (research R7).

  In `docs/src/guides/mcp.md`, point the "Connect Claude Code" section to the plugin as the recommended way.
- [ ] T013 [US2] Report faults in the tool arguments as tool errors (FR-014), test first.
  - **Tests**, in `crates/inkpdf/tests/mcp.rs`: an unknown key, a missing `templateId` or `data`, and a `data` that is not an object each return `isError: true`, with a text naming the argument and ending with "Fix the arguments and call the tool again.". An unknown tool name stays a JSON-RPC `-32602` error.
  - **Code**, in `call_tool` in `crates/inkpdf/src/mcp/mod.rs`: turn rmcp's argument refusal into that tool error, and keep the `mcp.tool` log with outcome `invalid-arguments` (`crates/inkpdf/tests/mcp_logs.rs` stays green).
  - **Docs**: update the errors section of `docs/src/guides/mcp.md` and the Errors bullet of `docs/src/contributing/architecture.md`. Add a note under "Protocol faults" in `specs/004-mcp-server/contracts/mcp-tools.md` saying that feature 005 changes it.

---

## Phase 5: User Story 3 — A generic skill to write templates (P2)

**Goal**: the `write-template` skill and its reference files.

**Independent Test**: `cargo xtask docs check` passes with `packages.md` up to date; with the plugin, an assistant writes a valid template (manual, T046).

- [ ] T014 [US3] Write the `write-template` skill files in `plugins/inkpdf/skills/write-template/`, following [contracts/plugin-skills.md](./contracts/plugin-skills.md):
  - `SKILL.md`: portable frontmatter; the steps, starting by finding the templates folder (ask the
    author, or read the volume mounted in their `compose.yaml`; no MCP tool reveals it) and ending
    with the check through the MCP tools; `demo` as the worked example;
  - `template-format.md`: folder, `template.json`, schema rules (closed objects, `description` and `x-role` everywhere, defaults for layout, `format: date`), `example.json`, limits;
  - `typst-basics.md`: reading `sys.inputs`, tables, page setup, fonts, bundled packages only.
- [ ] T015 [US3] In `xtask/src/docs.rs`, make `cargo xtask docs generate` write `plugins/inkpdf/skills/write-template/packages.md` from `packages/lock.toml`, reusing the package reference logic: each selectable package with its version, import line and one-line purpose. `cargo xtask docs check` MUST fail when the file is stale. Add a unit test.
- [ ] T016 [P] [US3] Rewrite `docs/src/guides/writing-a-template.md` and `docs/src/guides/best-practices.md` around descriptions, roles, the example and `format`, using `demo` as the worked example. The template content is final after T038, so review these two pages again then.

---

## Phase 6: User Story 4 — An example comes with each template (P2)

**Goal**: `example.json` is checked at load and returned in the template detail (HTTP, MCP, live OpenAPI).

**Independent Test**: the detail of a template with a valid example contains it, and it renders unchanged; an invalid example is dropped and logged, and the template stays valid.

### Tests (write first, must fail)

- [ ] T017 [P] [US4] Loader tests in `crates/inkpdf/src/registry/loader.rs`:
  - a valid `example.json` is stored as written, without defaults applied;
  - an example that breaks the schema, one with a bad `metadata`, and malformed JSON all give `example: None`, with the template still `Valid`;
  - no `example.json` gives `None`.
- [ ] T018 [P] [US4] API and MCP tests:
  - `crates/inkpdf/tests/api_templates.rs`: `GET /templates/{id}` has `example` when valid and omits it otherwise; the list never carries it;
  - `crates/inkpdf/tests/mcp.rs`: `get_template` `structuredContent.example` is present, and `render_template` with it succeeds;
  - `crates/inkpdf/tests/api_live_openapi.rs`: the template's render operation has the example as its request body `example`, and the copied schemas keep their `x-role` annotations (FR-003);
  - `crates/inkpdf/tests/hot_reload.rs` (hot reload on): editing `example.json` is picked up.

### Implementation

- [ ] T019 [US4] Add `example: Option<serde_json::Value>` to `TemplateEntry` in `crates/inkpdf/src/registry/loader.rs`. In the same file, read `example.json` from the loaded files and check a copy:
  1. `render::metadata::extract`;
  2. `TemplateSchema::prepare`.

  Keep the original on success. On failure, log `event = "template.example_invalid"` with `templateId` and the reason (the violation paths or the JSON error), without any example value.
- [ ] T020 [US4] Add the optional `example` field (`skip_serializing_if = "Option::is_none"`, documented with utoipa) to `TemplateDetail` in `crates/inkpdf/src/api/templates.rs`. Then regenerate the documentation:
  - `openapi/openapi.json` with `INKPDF_UPDATE_OPENAPI=1 cargo test -p inkpdf --test contract_openapi`;
  - `docs/src/reference/api.md` with `cargo xtask docs generate`.
- [ ] T021 [US4] In `crates/inkpdf/src/api/live.rs`, add the template's example as the `example` of each per-template render request body when present.
- [ ] T022 [P] [US4] Document `example.json`:
  - in `docs/src/reference/template-format.md`: content, load-time check, log event;
  - in `docs/src/guides/exploring-the-api.md`: the `example` field of the template detail.

**Checkpoint**: T017–T018 pass.

---

## Phase 7: User Story 5 — Impossible dates are refused at validation (P2)

**Goal**: `format` is asserted, and impossible dates get a 422 with their path.

**Independent Test**: `2026-02-30` in a `format: date` property gives `validation-failed` on that path, through HTTP and MCP; `2024-02-29` passes.

### Tests (write first, must fail)

- [ ] T023 [P] [US5] Unit tests in `crates/inkpdf/src/template/schema.rs`, with a `format: date` property at the root of `data`, in a nested object, in array `items` and via `$defs`:
  - `2026-02-30`, `2026-13-45`, `2025-02-29`, `2026-04-31` and `0000-00-00` are refused with `"<value>" is not a valid calendar date (YYYY-MM-DD)` on the right path;
  - `2024-02-29` is accepted;
  - `format: date-time` refuses `"2026-02-30T10:00:00Z"` and accepts `"2024-02-29T10:00:00Z"`;
  - `format: email` refuses `"not-an-email"`;
  - an unknown format name is ignored;
  - several date faults plus a type fault come back together.
- [ ] T024 [P] [US5] HTTP and MCP tests in `crates/inkpdf/tests/api_render.rs` and `crates/inkpdf/tests/mcp.rs`:
  - a render with an impossible date gives 422 `validation-failed` with the path;
  - through MCP, a tool error with the hint "Fix the input and call render_template again.", never `render-failed`.

### Implementation

- [ ] T025 [US5] In `crates/inkpdf/src/template/schema.rs`, build the validator with `should_validate_formats(true)`. Rewrite the message of `format: date` violations as in [contracts/template-format.md](./contracts/template-format.md). Other messages are unchanged.
- [ ] T026 [P] [US5] Document asserted formats in `docs/src/reference/template-format.md` and `docs/src/concepts/schemas.md`:
  - which standard formats are checked;
  - the date message;
  - the behaviour change for templates that used `format` as a hint.

  The commit message body states the change, so that it reaches the release notes.

**Checkpoint**: T023–T024 pass; the audit's five impossible dates are refused at every depth (SC-004).

---

## Phase 8: User Story 7 — Choose which version of a template to use (P2)

**Goal**: one sub-folder per version; reads and renders take an optional `version`; the default is the highest valid version; every render states the version used ([contracts/versions-and-loading.md](./contracts/versions-and-loading.md)).

**Independent Test**: with `facture` versions 1.0.0, 1.1.0 and 2.0.0, renders without a version, with `1`, `1.0.0`, `2` and `3` use the expected version or return `template-version-not-found` with the list (SC-007).

### Tests (write first, must fail)

- [ ] T027 [P] [US7] Registry unit tests in `crates/inkpdf/src/registry/loader.rs` and `crates/inkpdf/src/registry/mod.rs`:
  - **Layouts**: the flat layout gives one version (the manifest's); the versioned layout loads each version folder; the mixed layout makes the template invalid with the reason of the contract.
  - **Flat manifest versions**: `1.2.0` is selectable by `1` and `1.2.0`; `1.0` and `v1` are shown as is and used without a selector, and any explicit selector gives `template-version-not-found`.
  - **Bad folders and manifests**: a non-version sub-folder is skipped and logged; a manifest version that differs from its folder makes that version invalid; an invalid version does not hide the others.
  - **Selection**: none → highest valid; `1` → highest valid `1.x.y`; an exact version → that version, even invalid; a malformed selector (`latest`, `1.2`, `../1.0.0`, `1.0.0/..`) → error.
  - **Ordering**: numeric (`1.10.0` > `1.9.0`).
- [ ] T028 [P] [US7] HTTP tests in a new `crates/inkpdf/tests/api_versions.rs`. Build a `TestVolume` with the `sample` fixture copied as `facture/1.0.0`, `facture/1.1.0` and `facture/2.0.0`, setting each `template.json` version and changing a visible title so the PDFs differ.
  - `GET /templates` shows `versions`, newest first, with their status, and the default `version`.
  - `GET /templates/facture?version=1.0.0` and `/schema?version=1` describe the right version.
  - Renders:
    - without a version → `2.0.0`;
    - `1` → `1.1.0`;
    - `1.0.0` → `1.0.0`;
    - `2` → `2.0.0`;
    - each response has `Inkpdf-Template-Version`.
  - `version=3` gives 404 `template-version-not-found` with `availableVersions`.
  - `version=latest`, `version=../1.0.0` and `version=1.0.0/..` give 400 `invalid-parameter`.
  - When `2.0.0` is made invalid, the default is `1.1.0`.
- [ ] T029 [P] [US7] MCP and live OpenAPI tests:
  - in `crates/inkpdf/tests/mcp.rs`: `get_template` and `render_template` with `version`; the render text names the version; the `template-version-not-found` tool error carries the contract's hint;
  - in `crates/inkpdf/tests/api_live_openapi.rs`: the per-template operation describes the default version and documents the `version` parameter;
  - in `crates/inkpdf/tests/mcp_logs.rs`: `templateVersion` is logged.

### Implementation

- [ ] T030 [US7] In `crates/inkpdf/src/registry/`, implement research R9:
  - `Version`;
  - layout detection, and per-version loading and fingerprinting in `loader.rs`, with each version folder as its Typst root;
  - `TemplateVersions` and selection (`resolve(id, selector)`) in `mod.rs`;
  - `TemplateEntry.version` (kept as the version shown to callers: the folder name, or the manifest version in the flat layout) and the new `TemplateEntry.semver: Option<Version>` (the parsed version, used for ordering and selection);
  - rescans that still work per version when hot reload is on (`watcher.rs`).
- [ ] T031 [US7] Add `TemplateVersionNotFound { available }` (404, `template-version-not-found`, with `availableVersions` in the problem object) to `crates/inkpdf/src/error.rs`, and the `version` case of `invalid-parameter`. Regenerate `docs/src/reference/errors.md` with `cargo xtask docs generate`.
- [ ] T032 [US7] HTTP:
  - `crates/inkpdf/src/api/templates.rs`: `TemplateSummary.versions`; the default `version`; the `version` query parameter on the detail and the schema;
  - `crates/inkpdf/src/api/render.rs`: the `version` query parameter, parsed like `download` and `filename`, and the `Inkpdf-Template-Version` response header;
  - `crates/inkpdf/src/render/mod.rs`: `templateVersion` in the `render` log.

  Then regenerate `openapi/openapi.json` and `docs/src/reference/api.md`.
- [ ] T033 [US7] MCP and live document:
  - `crates/inkpdf/src/mcp/tools.rs`: the optional `version` argument on `get_template` and `render_template`, and `templateVersion` in the `mcp.tool` log;
  - `crates/inkpdf/src/mcp/result.rs`: the render text with the version, and the not-found hint;
  - `crates/inkpdf/src/api/live.rs`: each template described by its default version, with the `version` parameter documented.
- [ ] T034 [P] [US7] Document versions:
  - `docs/src/reference/template-format.md` and `docs/src/concepts/templates.md`: the layouts and their rules;
  - `docs/src/guides/exploring-the-api.md`: `?version=` and the header;
  - `docs/src/guides/mcp.md`: the `version` argument;
  - `plugins/inkpdf/skills/generate-document/SKILL.md`: use the default version unless the user asks for one;
  - `plugins/inkpdf/skills/write-template/SKILL.md` and `template-format.md`: create `<id>/1.0.0/`, and copy it to a new version folder for a new version.

**Checkpoint**: T027–T029 pass.

---

## Phase 9: User Story 8 — Templates are loaded at startup; hot reload is for development (P2)

**Goal**: no watcher or rescan by default; `INKPDF_HOT_RELOAD=true` turns them on, and `compose.yaml` turns it on for development.

**Independent Test**: with default settings, a template added after startup is not listed until a restart; with hot reload on, it is (SC-008).

- [ ] T035 [P] [US8] Tests:
  - in `crates/inkpdf/src/config.rs`: unit tests for `INKPDF_HOT_RELOAD` (default `false`; `true`, `false`, `1`, `0`; a refused value);
  - in `crates/inkpdf/tests/startup.rs`: tests of `inkpdf::spawn_background_tasks(&state)`, after the initial scan as in the existing startup tests:
    - with the default config, it returns no watcher, and a template added after startup is still not listed once the rescan interval has passed;
    - with `hot_reload: true`, it returns a watcher, and the added template is listed.

  `crates/inkpdf/tests/hot_reload.rs` spawns its watcher directly and needs no change.
- [ ] T036 [US8] Add `hot_reload: bool` to `Config` in `crates/inkpdf/src/config.rs`. In `crates/inkpdf/src/lib.rs`, add `spawn_background_tasks(&AppState) -> Option<WatcherHandle>`, which spawns `registry::watcher` only when `hot_reload` is on (research R10). In `crates/inkpdf/src/main.rs`, call it after the initial scan instead of spawning the watcher, and add `hotReload` to the `server.listening` log.
- [ ] T037 [P] [US8] Development setup and documentation:
  - set `INKPDF_HOT_RELOAD: "${INKPDF_HOT_RELOAD:-true}"` in `compose.yaml`;
  - document the variable in `docs/src/reference/configuration.md` (checked by `cargo xtask docs check`), and note that `INKPDF_RESCAN_INTERVAL_SECS` only applies with hot reload;
  - in `docs/src/operations/deployment.md`: publish a template or version by restarting, with a rolling restart to avoid downtime; `INKPDF_MAX_TEMPLATE_BYTES` applies per version folder, and memory grows with the number of loaded versions; the Compose example shows the default;
  - rename `docs/src/concepts/hot-reload.md` to "Loading templates" (title and `SUMMARY.md`): startup loading, and hot reload for development;
  - in the `write-template` skill: a new template appears at once with hot reload, otherwise after a restart.

**Checkpoint**: T035 passes.

---

## Phase 10: User Story 6 — One demonstration template instead of three (P2)

**Goal**: `demo` is the only example template; nothing refers to the removed ones.

**Independent Test**: the example volume lists only `demo`, valid, with a description and a role on every property; its example and every layout setting render; `cargo xtask docs check` finds no reference to removed templates.

- [ ] T038 [US6] Create `examples/templates/demo/1.0.0/` (`template.json` with version `1.0.0`, `schema.json`, `main.typ`, `example.json`) exactly as in [contracts/demo-template.md](./contracts/demo-template.md):
  - every property has a `description` and an `x-role`;
  - every object has `additionalProperties: false`;
  - `date` uses `format: date` and `recipient.email` uses `format: email`;
  - the total is formatted with `@preview/zero`;
  - the page footer reads "page x / n";
  - no QR code and no legal or payment wording.

  Use the `typst-dev` skill for `main.typ`.
- [ ] T039 [US6] Rewrite `crates/inkpdf/tests/examples.rs` for the example volume. Check that:
  - it contains exactly `demo`, `valid`;
  - every property of its `data` and `layout` (walked at any depth) has a `description` and an `x-role` (SC-002);
  - its detail carries the example, which renders unchanged (SC-003);
  - each layout variant renders: both fonts, both densities, `showTotal: false`, another `accentColor`;
  - impossible dates in `date` are refused with `/data/date` (SC-004).
- [ ] T040 [US6] Delete:
  - `examples/templates/progress-invoice`;
  - the whole `examples/requests/` folder (after T003 it only holds `progress-invoice/`);
  - `docs/src/guides/progress-invoice.md`, with its `docs/src/SUMMARY.md` entry;
  - `examples/bruno/5 Progress invoice/`.

  Switch every test that still uses `progress-invoice` (`grep -rn progress-invoice crates/`) to `demo` or to the `sample` fixture, keeping what it checks. The `unique_prefix` examples in `crates/inkpdf/src/api/live.rs` are plain strings and stay.
- [ ] T041 [P] [US6] Update the Bruno collection: `examples/bruno/2 Templates/02 Template detail.bru`, `03 Template schema.bru` and the `examples/bruno/4 Render/` requests use `demo` and its example. Delete `examples/bruno/4 Render/04 Package demo.bru`.
- [ ] T042 [P] [US6] Replace every remaining reference to the removed templates with `demo`, or with a short inline snippet where a package example is needed, in:
  - `docs/src/concepts/packages.md`, `docs/src/guides/packages.md`, `docs/src/guides/exploring-the-api.md`;
  - `docs/src/guides/mcp.md` (the worked conversation uses `demo`);
  - `docs/src/contributing/development.md` (fixtures location);
  - `README.md`, and the `compose.yaml` header comment.
- [ ] T043 [US6] Extend `cargo xtask docs check` in `xtask/src/docs.rs`: fail when `progress-invoice`, `packages-demo` or `examples/templates/sample` appears in `docs/src/`, `examples/`, `plugins/`, `README.md` or `compose.yaml` (SC-006). Add a unit test.

**Checkpoint**: T039 passes; `cargo xtask docs check` passes.

---

## Phase 11: Polish

- [ ] T044 [P] Describe the role check, `example.json`, format assertion, version folders and startup loading in the loader section of `docs/src/contributing/architecture.md`. Update the user's project memory note `repo-layout-003.md` (outside the repository): `demo/1.0.0` only, fixtures location, `plugins/inkpdf`, `x-role`, version folders, `INKPDF_HOT_RELOAD`.
- [ ] T045 Run [quickstart.md](./quickstart.md) §1–§5:
  - `cargo fmt --all --check`, clippy with `-D warnings`, `cargo test --workspace`;
  - `cargo xtask docs check`, `mdbook build docs`;
  - `docker compose up --build` with MCP on: the curl checks of §2–§4.

  Fix anything stale.
- [ ] T046 Quickstart §6, with Claude Code: `claude plugin validate plugins/inkpdf`, then install the plugin from the local marketplace. Then:
  - check SC-001 with a request for a demo document;
  - check SC-005 by asking for a meeting attendance sheet template.

  These two conversations need the user. Record what was checked under this task.
- [ ] T047 Mark `specs/005-mcp-agent-efficiency/spec.md` as implemented once merged. Write the PR description in English, covering:
  - what changes for template authors, assistants and HTTP callers;
  - the behaviour changes: asserted formats, and templates loaded at startup (hot reload for
    development only, constitution 2.0.0);
  - template versions;
  - the removed templates;
  - how to install the plugin.

---

## Dependencies & Execution Order

- **Phase 1** (T001, T002) and **Phase 2** (T003) come first. T003 blocks US6.
- **US1 (Phase 3)**: after Phase 2. It is independent of the other stories.
- **US2 (Phase 4)**:
  - T010 needs T001;
  - T011 needs T001 and T002;
  - T013 needs Phase 2 only; it changes `mcp/mod.rs`, like T008, so do them one after the other;
  - its manual check (T046) needs US1, US4, US6 and US7.
- **US3 (Phase 5)**:
  - T014 and T015 need T001;
  - T016 is written early and reviewed after T038.
- **US4 (Phase 6)**: after Phase 2; independent of US1. T020 regenerates the OpenAPI contract.
- **US5 (Phase 7)**: after Phase 2. T025 and T007 both change `template/schema.rs`, so do them one after the other or merge carefully.
- **US7 (Phase 8)**: after US4, since both change `registry/loader.rs`. T032 regenerates the OpenAPI contract again.
- **US8 (Phase 9)**: after Phase 2. T036 touches `config.rs`, `lib.rs` and `main.rs`; T037 and T034 both edit the `write-template` skill, so do them one after the other.
- **US6 (Phase 10)**:
  - T038 needs US1 (roles), US4 (example), US5 (formats) and US7 (the versioned layout `demo/1.0.0/`);
  - T039 needs T038;
  - T040 needs T003 and T038;
  - T043 needs T040, T041 and T042.
- **Polish**: last.

The phases of the P2 stories are ordered by dependency: US7 and US8 come before US6, which uses them.

### Parallel Opportunities

- Setup: T001 ∥ T002.
- After Phase 2: US1 (`template/schema.rs`, `mcp/`) ∥ US4 (`registry/`, `api/`) ∥ US8 (`config.rs`, `lib.rs`, `main.rs`) ∥ the skill writing of US2 and US3 (`plugins/`, `xtask/`).
- Tests within a story: T004 ∥ T005 ∥ T006; T017 ∥ T018; T023 ∥ T024; T027 ∥ T028 ∥ T029.
- Docs: T009, T012, T016, T022, T026, T034, T037, T041 and T042 touch different pages, so they can run in parallel, apart from the shared skill file noted under US8.

### Parallel Example: after Phase 2

```text
Task: "T004–T008 US1 roles in crates/inkpdf/src/template/schema.rs and crates/inkpdf/src/mcp/"
Task: "T017–T021 US4 example in crates/inkpdf/src/registry/ and crates/inkpdf/src/api/"
Task: "T035–T036 US8 loading in crates/inkpdf/src/config.rs, crates/inkpdf/src/lib.rs and crates/inkpdf/src/main.rs"
Task: "T010, T014 skills in plugins/inkpdf/skills/"
```

## Implementation Strategy

1. **MVP**: Phases 1 and 2, then US1. Roles and the new instructions already change how an assistant behaves with any template.
2. Then US4, US5, US7 and US8, the service changes, followed by US6, the `demo` template, which uses them.
3. Then US2 and US3, the skills, validated on `demo`.
4. Commit per phase, with conventional English messages. One PR for the feature, on `005-mcp-agent-efficiency`. It is based on `004-mcp-server`, so open it after 004 is merged, or against `004-mcp-server`.

## Notes

- T002 may end in "plugin version independent"; T011 then skips the version check.
- T046 is the only task that needs the user (live conversations with Claude Code).
