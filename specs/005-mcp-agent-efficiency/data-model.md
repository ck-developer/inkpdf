# Data model — Templates that guide AI assistants (005)

No stored data. The template format gains two optional parts, and two in-memory structures gain a
field.

## Template folder

| File | Status | Change |
|---|---|---|
| `template.json` | optional | unchanged |
| `schema.json` | required | may carry `x-role` on any property; `format` is now asserted |
| `main.typ` | required | unchanged |
| `example.json` | **new, optional** | a render body; checked at load |
| other files | optional | unchanged (assets, fonts, `parts/`) |

## `x-role`

- Values: `ask`, `fill`, `default` (string).
- Location: any property schema under `data` or `layout`.
- Rule: any other value → template invalid (reason names the JSON pointer and the value).
- Effect on input validation: none.

## `TemplateEntry` (in memory, `registry/loader.rs`)

| Field | Change |
|---|---|
| `example: Option<serde_json::Value>` | **new**: the example as written, present only if it passed the load-time check |
| `version: Option<Version>` | **new**: the version folder's number (versioned layout); the flat layout keeps the manifest's version |

## Template versions (registry)

- `Version`: `MAJOR.MINOR.PATCH`, three integers, ordered numerically.
- `TemplateMap`: `BTreeMap<TemplateId, TemplateVersions>`; `TemplateVersions` is either the single
  flat entry or a `BTreeMap<Version, Arc<TemplateEntry>>`, plus the reason when the layout is mixed.
- Selection: absent → highest valid; `MAJOR` → highest valid of that major; exact → that version.
- Each version is loaded, fingerprinted and checked independently.

## Configuration

| Field | Variable | Type | Default |
|---|---|---|---|
| `Config::hot_reload` | `INKPDF_HOT_RELOAD` | bool (`true`, `false`, `1`, `0`) | `false` |

## New error

| Code | Status | Extra field |
|---|---|---|
| `template-version-not-found` | 404 | `availableVersions: [string]` |

## `TemplateDetail` (HTTP and MCP)

| Field | Change |
|---|---|
| `example` (object, optional) | **new**; omitted when absent |
| `version` | now the selected version |

## `TemplateSummary` (HTTP and MCP)

| Field | Change |
|---|---|
| `version` | now the default version (highest valid) |
| `versions` | **new**: `[{ version, status, reason? }]`, newest first |

## Validation errors

Unchanged form (`path`, `schemaPath`, `message`). New source: `format` assertion; the `date` format
message reads `"<value>" is not a valid calendar date (YYYY-MM-DD)`.

## Repository content

| Item | Change |
|---|---|
| `examples/templates/demo/1.0.0/` | **new** (see [contracts/demo-template.md](./contracts/demo-template.md)) |
| `examples/templates/sample`, `examples/templates/packages-demo` | moved to `crates/inkpdf/tests/fixtures/templates/` |
| `examples/templates/progress-invoice`, `examples/requests/` (the whole folder, emptied by the moves) | deleted; the example now lives in `examples/templates/demo/example.json` |
| `examples/requests/sample.json`, `examples/requests/packages-demo.json` | moved to `crates/inkpdf/tests/fixtures/requests/` |
| `.claude-plugin/marketplace.json`, `plugins/inkpdf/` | **new** (see [contracts/plugin-skills.md](./contracts/plugin-skills.md)) |
