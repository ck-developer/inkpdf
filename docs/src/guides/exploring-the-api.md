# Exploring the API

A running inkpdf instance describes itself: each template's details carry its schema and an
example input, its OpenAPI document includes one typed operation per template currently loaded,
and an interactive page renders it. This guide shows how to read a template, choose its version,
browse the OpenAPI document, generate a typed client from it, and use the Bruno collection
shipped with the repository.

## A template's details and example

`GET /templates/{templateId}` returns a template's details: its id, name, description,
version, status and load time, its JSON Schema (`data` and `layout`, with each variable's
`description` and `x-role`), and links to its schema and render routes. When the template ships
a valid `example.json`, the details also carry an `example`: a complete request body, as the
author wrote it.

```sh
curl -s localhost:3000/templates/demo | jq '.example | keys'
```

```json
["data", "metadata"]
```

The example can be sent as is, which makes it a good starting point for your own requests:

```sh
curl -s localhost:3000/templates/demo | jq '.example' \
  | curl -s -H 'content-type: application/json' --data @- -o demo.pdf \
      localhost:3000/templates/demo/render
```

`example` is absent when the template has no `example.json`, or when its example does not pass
the template's own schema (the service then logs `template.example_invalid`). The list
`GET /templates` never carries it. See
[Template format → `example.json`](../reference/template-format.md#examplejson).

## Versions

A template can hold several versions (see [Templates → Versions](../concepts/templates.md#versions)).
`GET /templates` shows, for each template, its default `version` (the highest valid one) and
all its `versions`, newest first, with their status:

```json
{ "id": "facture", "name": "Facture", "version": "1.1.0", "status": "valid",
  "versions": [ { "version": "2.0.0", "status": "invalid", "reason": "schema.json: …" },
                { "version": "1.1.0", "status": "valid" },
                { "version": "1.0.0", "status": "valid" } ] }
```

The detail, the schema and the render routes take an optional `version` query parameter: a
major version (`1`, the highest valid `1.x.y`) or an exact one (`1.0.0`). Without it, they use
the default version.

```sh
curl -s 'localhost:3000/templates/facture?version=1.0.0' | jq '.version'
curl -s 'localhost:3000/templates/facture/schema?version=1'
curl -si -H 'content-type: application/json' --data @facture.json \
  'localhost:3000/templates/facture/render?version=1' | grep -i inkpdf-template-version
```

```text
inkpdf-template-version: 1.1.0
```

Every render response of a template that has a version carries the version used in the
`Inkpdf-Template-Version` header, with or without the parameter. A version that is not a
version number (`latest`, `v2`, `1.2`) gives `400 invalid-parameter`; a version that does not
exist gives `404 template-version-not-found`, whose problem object lists the available versions:

```json
{ "code": "template-version-not-found", "status": 404,
  "detail": "template facture has no version matching \"3\"; available: 2.0.0, 1.1.0, 1.0.0",
  "templateId": "facture", "availableVersions": ["2.0.0", "1.1.0", "1.0.0"] }
```

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
  "/templates/demo/render",
  "/templates/{templateId}",
  "/templates/{templateId}/render",
  "/templates/{templateId}/schema"
]
```

For a template with id `<id>`, the document adds:

| Element | Value |
|---|---|
| Path | `POST /templates/<id>/render` |
| `operationId` | `render_<id>`, with `-` replaced by `_` (`render_demo`; `render_my_report` for `my-report`) |
| `summary` | the template's `name` |
| `description` | its `description`, followed by `(version <version>)`, both from the default version |
| Parameters | `download`, `filename` and `version`, as on the generic operation |
| Request body | `<P>RenderRequest`, where `<P>` is the id in PascalCase (`Demo`; `MyReport` for `my-report`), with the template's `example`, when it has one, as its example |
| Components | `<P>RenderRequest`, `<P>Data`, `<P>Layout` (if the template declares `layout`), `<P>_<name>` for each `$defs` entry, and the shared `DocumentMetadata` |

`<P>RenderRequest` is closed (`additionalProperties: false`), and its `required` list is the
one of your schema root. The components describe the template's default version. Copied
schemas lose their `$schema` and `$id` but keep their `description` and `x-role` annotations,
and internal `$ref`s are rewritten to point at the new components, so every reference
resolves. If two ids map to
the same prefix (`a-b` and `a_b`), the second one gets a numeric suffix (`AB2`).

The document describes the templates the service has loaded: those found at startup, or, with
hot reload on (development), the current ones, so that the next request to `/openapi.json`
reflects a template once it has been reloaded. Invalid templates add nothing to it (they stay
visible in `GET /templates` with their `reason`). Operations are
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
covers every route against the example volume, which holds the `demo` template:

| Folder | Requests |
|---|---|
| `1 Operations` | health, readiness, OpenAPI document |
| `2 Templates` | list, detail, raw schema, detail of a selected version (`?version=1`) |
| `3 Packages` | list of bundled packages |
| `4 Render` | the `demo` example inline and as a download, every `layout` setting changed, a selected version (`?version=1`), and three deliberate errors: a body using the old `design` key instead of `layout` (`422`), an impossible date (`422`) and an unknown version (`404`) |

To use it:

1. start the service on the example templates, from a clone of the repository:

   ```sh
   docker compose up --build
   ```

2. open the `examples/bruno` folder in Bruno;
3. select the `local` environment (`baseUrl` is `http://localhost:3000`; change it to target
   another instance);
4. run a request, a folder or the whole collection. Each request asserts its expected status.

Most render bodies are copies of the template's `example.json`. A request whose `docs` block
says `Source: <file>.json` must contain exactly that file's body, and a test in the
repository fails if they drift apart or if a request targets a route that does not exist. You
can apply the same approach to your own templates; see
[Best practices](best-practices.md).
