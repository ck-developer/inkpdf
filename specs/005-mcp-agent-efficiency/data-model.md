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

## `TemplateEntry` (in memory)

| Field | Change |
|---|---|
| `example: Option<serde_json::Value>` | **new**: the example as written, present only if it passed the load-time check |

## `TemplateDetail` (HTTP and MCP)

| Field | Change |
|---|---|
| `example` (object, optional) | **new**; omitted when absent |

## Validation errors

Unchanged form (`path`, `schemaPath`, `message`). New source: `format` assertion; the `date` format
message reads `"<value>" is not a valid calendar date (YYYY-MM-DD)`.

## Repository content

| Item | Change |
|---|---|
| `examples/templates/demo/` | **new** (see [contracts/demo-template.md](./contracts/demo-template.md)) |
| `examples/templates/sample`, `examples/templates/packages-demo` | moved to `crates/inkpdf/tests/fixtures/templates/` |
| `examples/templates/progress-invoice`, `examples/requests/` (the whole folder, emptied by the moves) | deleted; the example now lives in `examples/templates/demo/example.json` |
| `examples/requests/sample.json`, `examples/requests/packages-demo.json` | moved to `crates/inkpdf/tests/fixtures/requests/` |
| `.claude-plugin/marketplace.json`, `plugins/inkpdf/` | **new** (see [contracts/plugin-skills.md](./contracts/plugin-skills.md)) |
