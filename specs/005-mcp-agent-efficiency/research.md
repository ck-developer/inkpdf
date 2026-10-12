# Research — Templates that guide AI assistants (005)

## R1 — Where and how a variable declares its role

**Decision.** A role is a JSON Schema annotation keyword, `x-role`, written next to
`description` on any property of `data` or `layout`, at any depth (inside `properties`, `items`,
`prefixItems`, `$defs`, `allOf`/`anyOf`/`oneOf`, `if`/`then`/`else`). Allowed values: `"ask"`,
`"fill"`, `"default"`.

```json
"recipient": {
  "type": "string",
  "description": "Full name of the person the document is addressed to.",
  "x-role": "ask"
}
```

- **Validation of the input**: none. JSON Schema 2020-12 ignores unknown keywords, so `jsonschema`
  treats `x-role` as an annotation. A role never changes what input is accepted (spec, assumptions).
- **Validation of the schema**: when a template is loaded, `TemplateSchema::load` walks the schema
  and checks every `x-role`. A value outside the three, or a non-string, makes the template
  invalid with the reason
  `schema.json: invalid role "<value>" at <JSON pointer>; expected "ask", "fill" or "default"`
  (FR-002).
- **Exposure**: the schema is already returned unchanged by `get_template` and
  `GET /templates/{id}` (detail and `/schema`), so roles reach assistants and HTTP callers with no
  new field (FR-003). The live OpenAPI document copies template schemas into components; `x-`
  keys are valid OpenAPI specification extensions, so they pass through.

**Rationale.** An annotation keyword keeps the role next to the description where an assistant
reads it, needs no separate file, and is invisible to validators that do not know it.

**Alternatives.**
- *A separate `roles.json` file*: drifts from the schema; the assistant has to merge two
  documents.
- *A namespaced keyword `x-inkpdf-role`*: longer for authors and assistants, with no real collision
  risk to avoid.
- *Encoding the role in `description`*: not machine-checkable.

## R2 — Impossible dates: asserting `format`

**Decision.** Template schemas are compiled with `jsonschema::options().should_validate_formats(true)`
(unknown formats stay ignored, the crate's default). A value of a property declared
`"format": "date"` must then be a real calendar date (`2026-02-30` refused, `2024-02-29`
accepted), at any depth (FR-007).

- The violation message for `format: date` is rewritten to the one `metadata.date` already uses:
  `"2026-02-30" is not a valid calendar date (YYYY-MM-DD)`. Other format messages keep the crate's
  text.
- Violations come from the same `iter_errors` pass as every other schema error, so they are
  listed together (US5, scenario 2).
- **Side effect, accepted**: other standard formats a template declares (`date-time`, `email`,
  `uri`, …) are asserted too. A template that used `format` as a mere hint now gets 422 for
  values that do not match. This is a correction, noted in the changelog (`feat!` is not needed
  before 1.0; the release notes say it).

**Rationale.** `format: "date"` is the standard way to say "this is a date"; asserting it is one
option, and the crate's date check is exact.

**Alternatives.**
- *Assert only `date`* with a custom keyword or a post-pass: more code, and leaves `email` and
  `date-time` unchecked for no good reason.
- *Check dates inside Typst*: the audit showed this ends in render failures without a path.

## R3 — The example input

**Decision.** A template may ship `example.json` at the root of its folder, holding exactly a
render body: `{ "data": …, "layout"?: …, "metadata"?: … }`.

- **At load** (`registry/loader.rs`), the example goes through the same checks as a render body:
  `metadata` extraction, then `TemplateSchema::prepare` (defaults, schema, formats). If it fails,
  the template stays valid, the example is dropped, and a `template.example_invalid` warning is
  logged with the template id and the violation paths (FR-006). Malformed JSON is treated the same
  way.
- **Stored** in `TemplateEntry::example: Option<Value>`, as written by the author (without the
  `layout` defaults applied), so the assistant sees what it should send.
- **Exposed** as a new optional `example` field of `TemplateDetail` (`GET /templates/{id}` and
  `get_template`). `TemplateSummary` (the list) does not carry it, to keep `list_templates` short.
- **Live OpenAPI**: each template's render operation gets the example as the request body
  `example`, so `/docs` shows a ready-to-send body.
- **Hot reload** (development only, R10): `example.json` is a file of the template folder,
  covered by the fingerprint; with hot reload on, editing it reloads the template.

**Rationale.** A full render body is what an assistant has to produce, so the example is directly
reusable; checking it at load guarantees SC-003 and keeps a broken example from misleading.

**Alternatives.**
- *Examples inside the schema (`examples` keyword)*: per-property examples do not show a whole
  coherent input; a root `examples` would be validated against itself in confusing ways.
- *Several examples*: one is enough for the goal; more can come later.
- *Make an invalid example invalidate the template*: too harsh for an optional aid
  (constitution V: an invalid template should not hide others, and here the template itself works).

## R4 — Guidance in the MCP server

**Decision.**
- **Server instructions** (R9 of 004), rewritten:

  > Everything you need is available through these tools; do not look for templates, schemas or
  > examples in local files. Use `list_templates` to find a template matching the user's request,
  > then `get_template` to read its schema and its example. Each variable has a description and
  > may have an `x-role`: `ask` — ask the user unless they already gave the value; `fill` — fill it
  > in yourself (for example today's date), and ask if you cannot work it out; `default` — leave
  > the default unless the user mentions it, or leave the variable out if it has none. Without a role, treat a required variable as `ask`
  > and an optional one with a default as `default`. Never invent a value you should have asked
  > for. Start from the example's structure. Then call `render_template`; if it returns validation
  > errors, ask the user about those fields only and call it again. Use a template's default
  > version unless the user asks for a specific one; `get_template` and `render_template` take an
  > optional `version`.

- **`get_template` description**: mentions the `example` and the `x-role` annotations.
- No change to the tool list or to the render result.

## R5 — The demonstration template `demo`

**Decision.** One neutral template, `examples/templates/demo/1.0.0/` (versioned layout, R9), called
"Demo document", that lays
out a generic document: a title, a reference, a date, a recipient block, a list of lines with
quantity and amount and a computed total, an optional note and a status. It shows:

| Possibility | Where |
|---|---|
| text (`ask`) | `title`, `recipient.name` |
| text the assistant fills (`fill`) | `reference` (description: "build it as DEMO-YYYY-NNN if the user gives none"), `date` |
| date with calendar check | `date` (`format: date`, `fill`: today's date) |
| number and integer with bounds | `lines[].quantity` (integer ≥ 1) |
| decimal amount as text | `lines[].amount` (pattern, 2 decimals), total formatted with the bundled `zero` package |
| yes/no | `urgent` (`default`, false) |
| choice from a list | `status` (enum, `default`) |
| list of items | `lines` (`ask`, 1 to 50 items) |
| nested group | `recipient` (`name`, `address`, `email` with `format: email`) |
| optional text | `note` (`default`: none) |
| layout settings | `accentColor` (pattern), `density` (enum), `showTotal` (boolean), `font` (enum) — all `default` |
| metadata | `example.json` carries `metadata.title` and `metadata.author` |
| bundled package | `@preview/zero` for number formatting |
| example | `example.json` |

Every property has a `description` and an `x-role`, and every object has
`additionalProperties: false` (the audit's defect 20). Labels in the PDF are in English and
neutral. No QR code and no legal or payment wording (the user's direction).

**Rationale.** It covers every kind of variable an author meets, in a document anyone
understands, and doubles as the skills' worked example.

## R6 — Removing the three example templates without losing test coverage

**Decision.**
- `examples/templates/sample` and `examples/templates/packages-demo` **move** to
  `crates/inkpdf/tests/fixtures/templates/` (with their requests in
  `crates/inkpdf/tests/fixtures/requests/`). Most service tests, the benchmark and the perf tests
  use `sample`; `bundled_packages.rs` and perf use `packages-demo`. Moving them keeps every
  assertion as is; `common::sample_template()` and `sample_request()` point to the new place.
- `examples/templates/progress-invoice`, `examples/requests/progress-invoice/`,
  `docs/src/guides/progress-invoice.md` and `examples/bruno/5 Progress invoice/` are **deleted**.
  Tests that used it (`examples.rs`, `mcp.rs`) switch to `demo` or to the `sample` fixture.
- `examples.rs` now checks the example volume: exactly one template, `demo`, valid; its example
  renders; each layout setting renders.
- The Bruno collection points to `demo` (`2 Templates`, `4 Render`); `4 Render/04 Package demo` is
  removed.
- `cargo xtask docs check` fails if `progress-invoice`, `packages-demo` or `examples/templates/sample`
  appears in `docs/src/`, `examples/`, `plugins/`, `README.md` or `compose.yaml` (SC-006). Tests and
  their fixtures keep the names on purpose; `specs/` keeps the history.

**Rationale.** The fixtures are test data, not examples for users; keeping them avoids rewriting
dozens of assertions, which would be risk without value.

## R7 — The two generic skills and how users install them

**Decision.** A Claude Code plugin `inkpdf` lives in the repository and doubles as a marketplace:

```text
.claude-plugin/marketplace.json        # marketplace "inkpdf", one plugin, source ./plugins/inkpdf
plugins/inkpdf/
├── .claude-plugin/plugin.json         # name, version, description, userConfig.server_url
├── .mcp.json                          # { "mcpServers": { "inkpdf": { "type": "http", "url": "${user_config.server_url}/mcp" } } }
└── skills/
    ├── generate-document/SKILL.md
    └── write-template/
        ├── SKILL.md
        ├── template-format.md         # folder, schema, roles, example, formats
        ├── typst-basics.md            # the Typst a template author needs
        └── packages.md                # bundled packages, generated by xtask from packages/lock.toml
```

- **Install in Claude Code**: `/plugin marketplace add ck-developer/inkpdf`, then
  `/plugin install inkpdf@inkpdf`; Claude Code asks for `server_url` (default
  `http://localhost:3000`) and connects the MCP server with it. One step gives the skills and the
  connection.
- **Portability**: both `SKILL.md` use only the six portable frontmatter fields (`name`,
  `description`, `license`, `compatibility`, `metadata`, `allowed-tools`) and plain Markdown, no
  Claude Code-only body feature. They can therefore also be zipped and uploaded to claude.ai or
  Claude Desktop (documented), where the MCP connection is made separately (004 guide).
- **generate-document**: the steps of US2 and R4 — tools only, roles, example, ask only what is
  needed, render, on validation errors ask about those fields only, report the file. Under 150
  lines. Template-independent.
- **write-template**: the steps of US3 — find the templates folder (ask the user, or read the
  volume mounted in their `compose.yaml`, `INKPDF_TEMPLATES`); no MCP tool reveals it. A new folder
  is picked up right away when the server runs with hot reload (development), otherwise after a
  restart, which the skill asks the user to do. Then create the template folder (versioned layout,
  starting at `1.0.0`), `template.json`,
  `schema.json` with `description` and `x-role` on every property and `additionalProperties: false`,
  layout defaults, `format: date` for dates, `example.json`, `main.typ` reading `sys.inputs`, then
  check through the MCP tools (`list_templates` shows it valid, `get_template` shows the example,
  `render_template` with the example succeeds). Uses `demo` as the worked example. Reference files
  are loaded only when needed.
- **Version**: `plugin.json` carries the crate version, bumped by release-please through
  `extra-files` (`{ "type": "json", "path": "/plugins/inkpdf/.claude-plugin/plugin.json",
  "jsonpath": "$.version" }` on the `crates/inkpdf` package). Release-please resolves
  `extra-files` relative to the package path, except a path with a leading `/`, which is relative
  to the repository root; a path containing `..` is refused ("illegal pathing characters").
  Checked in the source of release-please 17.6.1 (`release-please-action@v5` depends on
  `^17.6.1`) and of its main branch: `addPath` in `src/strategies/base.ts`, and its tests. The
  `cargo-workspace` plugin keeps these updates. The xtask version check stays.
- **One MCP connection, not two**: the plugin's `.mcp.json` declares a server named `inkpdf`, the
  same name the 004 guide uses with `claude mcp add`. The documentation says to use one or the
  other, never both.
- **Checks**: `cargo xtask docs check` also verifies that each `SKILL.md` frontmatter only uses the
  six portable fields, that `plugin.json` and `marketplace.json` parse, and that
  `skills/write-template/packages.md` is up to date. `claude plugin validate plugins/inkpdf` is run
  in the quickstart (the CI runners have no Claude Code).

**Rationale.** A plugin is the one-command install for Claude Code and carries the MCP connection
too; portable `SKILL.md` keeps the skills usable elsewhere (spec assumption: not tied to one
product).

**Alternatives.**
- *Skills only, copied by hand into `~/.claude/skills/`*: no versioning, no MCP connection.
- *A separate repository for the plugin*: the user wants one monorepo, tested end to end.

## R8 — Documentation

**Decision.**
- `reference/template-format.md`: `x-role`, `example.json`, asserted `format`.
- `guides/writing-a-template.md` and `guides/best-practices.md`: rewritten around `demo`, roles and
  the example.
- New `guides/assistant-skills.md`: what the two skills do, how to install the plugin in Claude
  Code, how to upload the skills elsewhere, `server_url`.
- `guides/mcp.md`: instructions, roles, example; the worked conversation uses `demo`.
- `guides/progress-invoice.md` deleted; `SUMMARY.md`, `concepts/packages.md`, `guides/packages.md`,
  `guides/exploring-the-api.md`, `contributing/development.md` and `README.md` updated.
- `reference/api.md` regenerated (`TemplateDetail.example`).

## R9 — Template versions

**Decision.**
- **Layout**, decided per template folder `<id>/` (constitution 2.0.0, principle II):
  - **flat**: `main.typ` and the other files at the root, as today. One version: the `version` of
    `template.json`, or none. A manifest version that parses as `MAJOR.MINOR.PATCH` is selectable
    like any version. One that does not (`1.0`, `v1`) is shown as is and used when no version is
    asked for; any explicit selector then gives `template-version-not-found`.
  - **versioned**: only sub-folders named `MAJOR.MINOR.PATCH` (numbers only), each a complete
    template folder. A `template.json` inside a version folder may omit `version`; if it gives one,
    it must equal the folder name, or that version is invalid
    (`template.json: version "1.2.0" does not match its folder "1.3.0"`).
  - **mixed** (template files at the root *and* version sub-folders): the template is invalid,
    with the reason `put the template files either at the root or in version folders, not both`.
  - A sub-folder of a versioned template whose name is not a version (`draft/`) is skipped and
    logged (`template.version_ignored`).
- **Registry**: `TemplateMap` becomes `BTreeMap<TemplateId, TemplateVersions>`, where
  `TemplateVersions` holds a `BTreeMap<Version, Arc<TemplateEntry>>` (`Version` = three `u32`,
  ordered numerically) or the single flat entry. Each version is loaded, fingerprinted and checked
  on its own; its Typst root is its own folder, so self-containment is per version.
  `TemplateEntry.version: Option<String>` stays the version shown to callers: the folder name in
  the versioned layout, the manifest version in the flat layout. `TemplateEntry` gains
  `semver: Option<Version>`, that version parsed as `MAJOR.MINOR.PATCH`, used for ordering and
  selection (`None` for a flat template without a version or whose version does not parse).
- **Size and memory**: `INKPDF_MAX_TEMPLATE_BYTES` applies per version folder, not per template.
  Every version stays loaded, so memory grows with the number of loaded versions; the deployment
  documentation says so.
- **Selection**, `version` parameter (optional):
  - absent → the highest **valid** version;
  - `MAJOR` (`1`) → the highest valid `1.x.y`;
  - `MAJOR.MINOR.PATCH` (`1.0.0`) → exactly that version, valid or not; if it is invalid, the
    usual `template-invalid` error;
  - anything else (`latest`, `v2`, `1.2`) → `invalid-parameter` on `version`;
  - nothing matches → new error `template-version-not-found` (404). Its `detail` lists the available
    versions; the problem object also carries an `availableVersions` array. A flat template whose
    manifest version does not parse never matches a selector.
  - All versions invalid and no selector → `template-invalid`, as today for a single template.
- **HTTP**: a `version` query parameter on `GET /templates/{id}`, `GET /templates/{id}/schema` and
  `POST /templates/{id}/render`, with the same parsing as `download` and `filename`. Every render
  response carries `Inkpdf-Template-Version: <version>` when the template has a version.
- **Template list**: `TemplateSummary` keeps `version`, now the default version (highest valid),
  and gains `versions`: `[{ "version": "2.0.0", "status": "invalid", "reason": "…" }, …]`, newest
  first. A flat template has one entry, or none if it has no version.
- **Template detail**: describes the selected version (default: highest valid) and states its
  `version`.
- **MCP**: `get_template` and `render_template` gain an optional `version` string argument, with
  the same rules. The render text becomes
  `Rendered facture.pdf from template facture 2.0.0 (48.2 KiB).`. The tool error for
  `template-version-not-found` ends with the hint "Call get_template without a version to see the
  available versions.".
- **Live OpenAPI**: each template's typed render operation describes its default version; the
  `version` query parameter is documented on it and on the generic operations.
- **Logs**: `render` and `mcp.tool` events gain `templateVersion`.
- **Errors reference**: `template-version-not-found` comes from `error.rs`, so
  `docs/src/reference/errors.md` regenerates.

**Rationale.** A folder per version is what the user chose; it keeps each version self-contained
and lets authors copy `1.0.0/` to `2.0.0/` to start a new version. Choosing by major number gives
"stay on version 1" without pinning a patch.

**Alternatives.**
- *`<id>@<version>/` folders next to each other*: offered to the user, not chosen.
- *Pre-release versions (`2.0.0-beta`)*: out of scope; a pre-release can live in a separate template
  id until it is ready.
- *Version in the path (`/templates/{id}/versions/{version}/render`)*: doubles every route; a query
  parameter matches the existing `download` and `filename` parameters.

## R10 — Loading at startup, hot reload for development

**Decision.**
- New setting `INKPDF_HOT_RELOAD` (`true`, `false`, `1`, `0`; default `false`).
- The decision to start the watcher moves into a library function,
  `inkpdf::spawn_background_tasks(&state)`, which `main.rs` calls after the initial `scan_all`. It
  returns the watcher handle only when `hot_reload` is on, so the choice can be tested without
  starting the binary.
- `false`: `spawn_background_tasks` starts neither the file watcher nor the periodic rescan and
  returns no handle. The registry never changes while the service runs.
- `true`: today's behaviour (`registry::watcher::spawn` with `INKPDF_RESCAN_INTERVAL_SECS`).
- The `server.listening` log line gains `hotReload: true|false`.
- `compose.yaml`, the local development setup, sets `INKPDF_HOT_RELOAD: "${INKPDF_HOT_RELOAD:-true}"`.
  The image and the deployment example keep the default, `false`.
- Documentation:
  - `configuration.md`: the variable, and `INKPDF_RESCAN_INTERVAL_SECS` now only matters with hot
    reload;
  - `deployment.md`: publishing a template or version means restarting; a rolling restart avoids
    downtime;
  - `concepts/hot-reload.md` becomes "Loading templates": startup loading, hot reload for
    development.
- Tests: `spawn_background_tasks` starts no watcher with the default config, so a template added
  after startup is not listed, and starts one with `hot_reload: true`. `hot_reload.rs` keeps
  spawning its watcher directly, so it needs no config change.

**Rationale.** The user wants no hot reload in production; with versions, a frozen set of
templates means a caller always gets the version it asked for (constitution 2.0.0, principle V).

**Alternatives.**
- *Reload on a signal (SIGHUP) or an admin route*: a restart is simpler and already handled by
  orchestrators.
- *Remove hot reload entirely*: the user wants to keep it for development, and the
  `write-template` skill relies on it to test a new template.
