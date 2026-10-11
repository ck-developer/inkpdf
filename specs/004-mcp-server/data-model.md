# Data model — MCP server (004)

No stored data. The feature adds one setting and in-memory types only.

## Configuration

| Field | Variable | Type | Default | Validation |
|---|---|---|---|---|
| `Config::mcp_enabled` | `INKPDF_MCP_ENABLED` | bool | `false` | `true`, `false`, `1`, `0`; anything else stops startup with a configuration error |

## MCP tool arguments

| Type | Fields | Rules |
|---|---|---|
| `GetTemplateArgs` | `templateId: string` | unknown keys refused |
| `RenderTemplateArgs` | `templateId: string`, `data: object`, `layout?: object`, `metadata?: DocumentMetadata`, `filename?: string` | unknown keys refused; `{ data, layout, metadata }` is then validated by the template's schema and the fixed metadata schema, exactly as the HTTP body |

## Shared pipeline (refactor, no new data)

`render::run(state, templateId, body, interface) -> Result<Vec<u8>, ApiError>`

- **Input**: the same JSON object as the HTTP request body, and the calling `Interface`
  (`Http` | `Mcp`, used only for logs).
- **Output**: the PDF bytes, or one `ApiError` (unchanged enum).
- **Steps**: lookup → status check → metadata extraction → schema preparation → violations
  merged → render under the semaphore and timeouts.

## Tool result mapping

| Pipeline outcome | Tool result |
|---|---|
| PDF bytes | text summary + embedded resource (`application/pdf`, base64) |
| `ApiError` | `isError: true`, text (title, detail, violations, hint) + `structuredContent` = HTTP problem object |

## Relationships

- Templates: the same registry snapshot as the HTTP API; hot reload applies to both.
- Limits: the same semaphore, queue timeout, render timeout and body limit.
