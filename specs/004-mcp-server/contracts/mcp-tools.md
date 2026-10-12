# Contract — MCP interface

Endpoint: `POST /mcp` on the service's address and port, only when `INKPDF_MCP_ENABLED=true`.
Transport: MCP Streamable HTTP, stateless, JSON responses. Protocol versions: those negotiated by
`rmcp` 3.5 (`2026-07-28` and the earlier versions it supports).

## Server identity

`initialize` answers with:

| Field | Value |
|---|---|
| `serverInfo.name` | `inkpdf` |
| `serverInfo.version` | the crate version |
| `capabilities` | `tools` only (no `listChanged`), no prompts, no resources |
| `instructions` | the flow of [research R9](../research.md#r9--server-instructions-for-the-assistant) |

## HTTP-level behaviour

| Situation | Answer |
|---|---|
| MCP off (default) | `404`, exactly as any unknown path |
| Request with an `Origin` header | `403`, no JSON-RPC processing |
| Body above `INKPDF_MAX_BODY_BYTES + 64 KiB` | refused by the transport (`413`) |
| `GET /mcp` (SSE stream) | `405`: no server-initiated messages in stateless mode |

## Tools

All three tools carry `annotations: { readOnlyHint: true, idempotentHint: true,
openWorldHint: false }`.

### `list_templates`

- **Description (for the assistant)**: "List the document templates this inkpdf server can
  render, with their id, name, description and status. Start here to find the template that
  matches the user's request."
- **Input**: `{}` (no arguments).
- **Result**: `structuredContent` = the body of `GET /templates`
  (`{ "templates": [TemplateSummary…] }`), and the same JSON as a text block. An empty list adds
  the text "No template is loaded."

### `get_template`

- **Description**: "Get one template's details and the JSON Schema of the data it expects
  (`data` and `layout`): required fields, types, allowed values, descriptions and defaults;
  `metadata` is described in render_template's input schema. Ask the user for every required
  field you do not know before rendering."
- **Input**:

  ```json
  { "type": "object", "required": ["templateId"], "additionalProperties": false,
    "properties": { "templateId": { "type": "string", "description": "Template id, from list_templates." } } }
  ```

- **Result**: `structuredContent` = the body of `GET /templates/{templateId}` (summary,
  `loadedAt`, `schema`, `links`), and the same JSON as a text block.
- **Errors**: `template-not-found`.

### `render_template`

- **Description**: "Render a template to PDF with the given data and return the PDF file. The
  input must match the template's schema (see get_template). Nothing is stored."
- **Input**:

  ```json
  { "type": "object", "required": ["templateId", "data"], "additionalProperties": false,
    "properties": {
      "templateId": { "type": "string" },
      "data":     { "type": "object", "description": "Business data, as described by the template's schema." },
      "layout":   { "type": "object", "description": "Layout parameters; omitted properties take their default." },
      "metadata": { "$ref": "DocumentMetadata schema of the HTTP API (title, author, subject, keywords, date)" },
      "filename": { "type": "string", "description": "File name without extension; defaults to the template id." } } }
  ```

  The tool builds `{ data, layout?, metadata? }` and runs the same pipeline as
  `POST /templates/{templateId}/render`; `filename` follows the HTTP `filename` query parameter.
- **Result** (success):

  ```json
  { "content": [
      { "type": "text", "text": "Rendered employer-certificate.pdf from template employer-certificate (48.2 KiB)." },
      { "type": "resource", "resource": {
          "uri": "inkpdf://templates/employer-certificate/render/employer-certificate.pdf",
          "mimeType": "application/pdf",
          "blob": "<base64>" } } ],
    "isError": false }
  ```

- **Errors**: every error of the HTTP render route except `unsupported-media-type` and
  `invalid-json` (MCP arguments are already parsed JSON). `invalid-parameter` comes from an
  invalid `filename` only.

## Tool errors

A pipeline error is a successful JSON-RPC response with `isError: true`. Its text is the
problem's title and detail, one line per violation (`- <path>: <message>`, `(root)` for an
empty path) or Typst diagnostic (`- <file>:<line>:<column>: <message>`), then the closing hint:

```json
{ "content": [ { "type": "text", "text": "Validation failed: 2 violation(s)\n- /data/employee: \"name\" is a required property\n- /layout/accent: \"pink\" is not one of \"blue\" or \"green\"\nFix the input and call render_template again." } ],
  "structuredContent": { "type": "https://github.com/ck-developer/inkpdf/errors/validation-failed",
                         "code": "validation-failed",
                         "title": "Validation failed", "status": 422, "detail": "2 violation(s)",
                         "templateId": "employer-certificate", "violations": [ … ] },
  "isError": true }
```

| Error slug | Closing hint in the text |
|---|---|
| `validation-failed`, `invalid-parameter`, `payload-too-large` | Fix the input and call render_template again. |
| `template-not-found` | Call list_templates to see the available templates. |
| `template-invalid` | This template cannot be used until the operator fixes it. |
| `overloaded`, `render-timeout` | Try again in a moment. |
| `render-failed` | Tell the operator; the template failed to render. |

`structuredContent` is the exact problem object the HTTP API returns for the same error.

Protocol faults are JSON-RPC "invalid params" errors (`-32602`, with HTTP `200`), not tool
results: unknown tool name, missing `templateId` or `data`, `data` that is not an object, and
unknown argument keys (the input schemas forbid them). The message names the faulty argument.
`rmcp` reports an unknown tool this way; it returns argument faults as tool errors, which the
server's `tools/call` turns into this error. The input *inside* `data`, `layout` and `metadata`
is checked by the template's schema and reported as a tool error, like the HTTP API; so is a
`layout` or `metadata` that is `null` or not an object.

## Logs

- `event = "render"` gains `interface = "http" | "mcp"`.
- `event = "mcp.tool"`, `tool`, `templateId?`, `durationMs`, `outcome` (`ok`, the error slug, or
  `invalid-arguments` for a call refused for its arguments). An unknown tool name is not
  logged as `mcp.tool`.
