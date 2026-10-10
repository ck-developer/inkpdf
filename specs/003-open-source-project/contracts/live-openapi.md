# Contract — Live, template-aware OpenAPI document

`GET /openapi.json` returns the OpenAPI 3.1 document built at request time from the templates
currently loaded. `GET /docs` renders it interactively and always fetches `/openapi.json`.

## Generic part (locked)

- These are the routes and components of the service: templates, render, packages, health, ready
  and openapi.
- `openapi/openapi.json` is the document produced with **no template loaded**. The contract test
  compares it with the file, ignoring `info.version`.

## Per-template part (live)

Each **valid** template with id `<id>` and prefix `<P>` (PascalCase of `<id>`, made unique)
adds an operation and its components:

```yaml
paths:
  /templates/<id>/render:
    post:
      operationId: render_<id with - replaced by _>
      tags: ["templates (live)"]
      summary: <template.json name, or id>
      description: <template.json description> (version <version>)
      parameters: [download, filename]      # same as the generic render operation
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/<P>RenderRequest" }
      responses: <same as the generic render operation>
components:
  schemas:
    <P>RenderRequest:
      type: object
      additionalProperties: false
      required: <required list of the template schema root>
      properties:
        data:     { $ref: "#/components/schemas/<P>Data" }
        layout:   { $ref: "#/components/schemas/<P>Layout" }     # only if the template declares layout
        metadata: { $ref: "#/components/schemas/DocumentMetadata" }
    <P>Data:   <copy of properties.data, internal $ref rewritten>
    <P>Layout: <copy of properties.layout, internal $ref rewritten>
    <P>_<def>: <each $defs entry, internal $ref rewritten>
```

## Rules

- Operations are sorted by template id. The output is deterministic for a given set of
  templates.
- Invalid templates add nothing. They remain visible in `GET /templates`.
- `$schema` and `$id` are removed from the copies.
- Every `$ref` of the document resolves.
- A template change is visible in the next response once the template has been reloaded
  (hot-reload delay).
- The generic `POST /templates/{templateId}/render` remains: concrete paths take precedence
  for the listed templates.
