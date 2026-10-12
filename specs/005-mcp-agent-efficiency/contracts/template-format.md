# Contract — Template format additions

Applies to every template folder. A template written for an earlier version (no role, no example,
no `format`) loads and renders exactly as before.

## `x-role` in `schema.json`

| Value | Meaning for the assistant |
|---|---|
| `"ask"` | Ask the end user, unless they already gave the value. |
| `"fill"` | Fill it in yourself from the context (today's date, a value derived from other answers, a rule given in the description); ask if you cannot work it out. |
| `"default"` | Leave the default value (or leave the variable out) unless the end user mentions it. |

- Allowed on any property schema under `data` or `layout`, at any depth, including array `items`
  and `$defs`.
- Without `x-role`: a required variable is `ask`; an optional one is `default`.
- Load-time check: a value that is not one of the three strings makes the template invalid:

  ```text
  schema.json: invalid role "maybe" at /properties/data/properties/title/x-role; expected "ask", "fill" or "default"
  ```

- Never used to validate input.

## `example.json`

- Optional file at the root of the template folder. Content: a render body,
  `{ "data": {…}, "layout": {…}?, "metadata": {…}? }`.
- Checked at load like a render body (metadata schema, template schema with `layout` defaults and
  asserted formats).
- Valid: returned unchanged as `example` in the template detail.
- Invalid or malformed: not returned; the template stays valid; log event:

  ```json
  {"event":"template.example_invalid","templateId":"demo","reason":"/data/date: \"2026-02-30\" is not a valid calendar date (YYYY-MM-DD)"}
  ```

## Asserted `format`

- Template schemas are validated with format assertion on. Standard formats (`date`, `date-time`,
  `time`, `email`, `uri`, …) are checked; unknown format names are ignored.
- `format: "date"` message:

  ```text
  /data/date: "2026-02-30" is not a valid calendar date (YYYY-MM-DD)
  ```

- Reported in the same `validation-failed` problem (422) as every other schema violation.

## Template detail (HTTP and MCP)

`GET /templates/{templateId}` and the MCP tool `get_template` return the existing `TemplateDetail`
plus:

| Field | Type | Presence |
|---|---|---|
| `example` | object (render body) | only when the template ships a valid `example.json` |

The OpenAPI document (`openapi/openapi.json`, `docs/src/reference/api.md`) is regenerated. In the
live document, each template's render operation carries the example as the request body
`example`.

## MCP server

- `initialize` → `instructions`: the text of [research R4](../research.md#r4--guidance-in-the-mcp-server).
- `get_template` description: "Get one template's details: its JSON Schema (`data` and `layout`)
  with a description and an optional `x-role` (`ask`, `fill` or `default`) for each variable, and,
  when the template ships one, a complete `example` input to start from. `metadata` is described in
  render_template's input schema."
- Tools, inputs and render results are otherwise unchanged.
