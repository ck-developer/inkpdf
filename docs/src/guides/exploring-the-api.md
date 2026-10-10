# Exploring the API

A running inkpdf instance describes itself: its OpenAPI document includes one typed operation
per template currently loaded, and an interactive page renders it. This guide shows how to
browse it, generate a typed client from it, and use the Bruno collection shipped with the
repository.

## The live OpenAPI document

`GET /openapi.json` returns an OpenAPI 3.1 document built from the templates loaded right now.
It has two parts:

- the **generic routes** of the service (templates, render, packages, health, readiness,
  OpenAPI), documented in [HTTP API](../reference/api.md). The generic
  `POST /templates/{templateId}/render` accepts any template, with an untyped `data` and
  `layout`;
- one **typed render operation per valid template**, tagged `templates (live)`, whose request
  body is that template's own schema.

```sh
curl -s localhost:3000/openapi.json | jq '.paths | keys'
```

```json
[
  "/health",
  "/openapi.json",
  "/packages",
  "/ready",
  "/templates",
  "/templates/packages-demo/render",
  "/templates/progress-invoice/render",
  "/templates/sample/render",
  "/templates/{templateId}",
  "/templates/{templateId}/render",
  "/templates/{templateId}/schema"
]
```

For a template with id `<id>`, the document adds:

| Element | Value |
|---|---|
| Path | `POST /templates/<id>/render` |
| `operationId` | `render_<id>`, with `-` replaced by `_` (`render_progress_invoice`) |
| `summary` | the template's `name` |
| `description` | its `description`, followed by `(version <version>)` |
| Parameters | `download` and `filename`, as on the generic operation |
| Request body | `<P>RenderRequest`, where `<P>` is the id in PascalCase (`ProgressInvoice`) |
| Components | `<P>RenderRequest`, `<P>Data`, `<P>Layout` (if the template declares `layout`), `<P>_<name>` for each `$defs` entry, and the shared `DocumentMetadata` |

`<P>RenderRequest` is closed (`additionalProperties: false`), and its `required` list is the
one of your schema root. Copied schemas lose their `$schema` and `$id`, and internal `$ref`s
are rewritten to point at the new components, so every reference resolves. If two ids map to
the same prefix (`a-b` and `a_b`), the second one gets a numeric suffix (`AB2`).

The document follows hot reload: add, change or fix a template, and the next request to
`/openapi.json` reflects it, once the template has been reloaded. Invalid templates add
nothing to it (they stay visible in `GET /templates` with their `reason`). Operations are
sorted by template id, and the output is deterministic for a given set of templates.

## The interactive documentation: `/docs`

Open `http://localhost:3000/docs` in a browser. The page is a [Scalar](https://scalar.com) API
reference that always loads `/openapi.json`, so it shows your templates as they are now: their
fields, descriptions, enums and defaults, and a form to send a render request and see the PDF.

The page loads the Scalar script from the jsDelivr CDN **in the reader's browser**. The
service itself makes no network request, but the browser opening `/docs` needs access to
`cdn.jsdelivr.net`. In an isolated network, use the raw `/openapi.json` with a local tool
instead.

## Generating a typed client

Because each template has its own typed operation, a standard OpenAPI generator pointed at a
running instance produces a client with one method and one set of types per template. Start
inkpdf with the templates you want in the client, then run your generator against
`/openapi.json`. For example, with [OpenAPI Generator](https://openapi-generator.tech):

```sh
npx @openapitools/openapi-generator-cli generate \
  -i http://localhost:3000/openapi.json \
  -g typescript-fetch \
  -o ./inkpdf-client
```

or, for TypeScript types only, with [openapi-typescript](https://openapi-ts.dev):

```sh
npx openapi-typescript http://localhost:3000/openapi.json -o inkpdf.d.ts
```

The result exposes, for instance, `renderProgressInvoice(...)` taking a
`ProgressInvoiceRenderRequest`. Things to keep in mind:

- the client reflects the templates loaded **at generation time**: regenerate it when a
  template's schema changes, ideally in CI against the same templates you deploy;
- the response of a render is binary (`application/pdf`); configure your generator or wrapper
  to read it as a blob or byte array;
- errors are RFC 9457 `application/problem+json` bodies, described in
  [Error codes](../reference/errors.md).

## The Bruno collection

The repository ships a [Bruno](https://www.usebruno.com) collection in `examples/bruno` that
covers every route against the example templates:

| Folder | Requests |
|---|---|
| `1 Operations` | health, readiness, OpenAPI document |
| `2 Templates` | list, detail, raw schema |
| `3 Packages` | list of bundled packages |
| `4 Render` | sample inline and as a download, sample with `metadata`, package demo, and a deliberate `422` (a body using the old `design` key instead of `layout`) |
| `5 Progress invoice` | the four example invoices, plus one as a download |

To use it:

1. start the service on the example templates, from a clone of the repository:

   ```sh
   docker compose up --build
   ```

2. open the `examples/bruno` folder in Bruno;
3. select the `local` environment (`baseUrl` is `http://localhost:3000`; change it to target
   another instance);
4. run a request, a folder or the whole collection. Each request asserts its expected status.

The render bodies are copies of the files in `examples/requests`. A request whose `docs` block
says `Source: examples/requests/<file>.json` must contain exactly that body, and a test in the
repository fails if they drift apart or if a request targets a route that does not exist. You
can apply the same approach to your own templates; see
[Best practices](best-practices.md).
