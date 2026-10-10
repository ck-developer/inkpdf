# Introduction

inkpdf is an HTTP service that turns JSON into PDF documents using
[Typst](https://typst.app) templates. The Typst engine is embedded in a single Rust binary:
there is no browser, no subprocess and no network access during a render.

## A foundation, not a document product

inkpdf knows nothing about invoices, reports, certificates or any other kind of document. It
provides the machinery around templates, and leaves the documents themselves to template
authors:

- A **template** is a folder dropped into a mounted volume: a Typst entry point (`main.typ`), a
  JSON Schema (`schema.json`) and optional resources (other Typst files, images, fonts).
- The **template author** decides, in the template's own schema, what the caller sends:
  - `data`: the content of the document;
  - `layout`: appearance parameters (colors, alignment, optional blocks…), each with a default
    value so that a call without any `layout` still produces a valid document.
- The **service** defines one more section, `metadata`, for the PDF properties (title, author,
  subject, keywords, date). It is the same for every template.

A render request is therefore always shaped like this:

```json
{
  "data":     { "…": "defined by the template's schema" },
  "layout":   { "…": "defined by the template's schema, defaults applied" },
  "metadata": { "title": "…", "author": "…" }
}
```

The body is validated against the template's schema before Typst runs. A valid body is passed to
Typst as data (`sys.inputs`), never as code, and compiled to PDF.

The templates in the repository's `examples/templates` folder (a neutral sample, a package demo,
a multi-page progress invoice) are illustrations. They are not a required structure: a template
can model any document with any schema.

## What the service provides

- **Live templates**: templates are discovered from the volume and reloaded without a restart.
- **Validation**: every request is checked against the template's JSON Schema; errors list every
  faulty path.
- **A sandbox**: a template only reads its own folder; no network, no environment variables, no
  system fonts.
- **Bundled Typst packages**: QR codes, barcodes, charts, number and date formatting, imported
  by name and never downloaded.
- **Deterministic output**: the same template and the same body produce byte-identical PDFs.
- **A self-describing API**: the running service publishes an OpenAPI 3.1 document at
  `GET /openapi.json` and interactive documentation at `GET /docs`. Both are built live from the
  loaded templates: each valid template gets its own typed render operation, with its `data` and
  `layout` schemas.

## What it is not

- Not a document generator with built-in document types: without templates, it renders nothing.
- Not an HTML-to-PDF converter: templates are written in Typst.
- Not a public-facing service: it has no authentication, no users and no permissions, and is
  meant to run on a private network.
- Not a storage or batch system: one request renders one PDF, returned in the response; nothing
  is kept.
- Not a template editor: templates are managed as files in the volume, never uploaded through
  the API.

## Where to go next

- [Run with Docker](getting-started/docker.md) to render a first PDF in a few minutes.
- [Your first template](getting-started/first-template.md) to write the smallest possible
  template.
- [Templates](concepts/templates.md) and [The request body](concepts/request-body.md) for the
  core concepts.
- [Exploring the API](guides/exploring-the-api.md) to use the live `/docs` page.
