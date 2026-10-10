# The request body

A render request is a `POST /templates/{templateId}/render` with a JSON object body
(`Content-Type: application/json`). The body has up to three sections:

```json
{
  "data":     { "…": "content, defined by the template's schema" },
  "layout":   { "…": "appearance, defined by the template's schema; defaults applied" },
  "metadata": { "title": "…", "author": ["…"], "subject": "…", "keywords": ["…"], "date": "2026-10-01" }
}
```

| Section | Defined by | Required | Reaches Typst |
|---------|-----------|----------|---------------|
| `data` | the template's `schema.json` | as the schema says (usually yes) | yes, as `sys.inputs.data` |
| `layout` | the template's `schema.json` | no, defaults fill the gaps | yes, as `sys.inputs.layout` |
| `metadata` | the service (fixed schema) | no | no, written into the PDF properties |

No other key is accepted at the root of the body.

## `data`: the content

`data` is the content of the document: whatever the template needs to display. inkpdf gives it
no meaning; its structure is entirely defined by `properties.data` in the template's schema. For a
greeting card it may be a single name, for a report a list of sections, for a label a set of
codes.

## `layout`: the appearance

`layout` holds the parameters that change how the document looks without changing what it says:
colors, alignment, density, which optional blocks are shown… They are also defined by the
template, in `properties.layout`, and each one should have a `default`. Before validation, the
service fills in every missing parameter with its default, so a caller can send only the
parameters it wants to change, or no `layout` at all.

This separation lets a single template produce several visual variants without being duplicated:
the same `data` with a different `layout` gives a different-looking document.

## `metadata`: the PDF properties

`metadata` is the only section defined by the service. It is the same for every template, is not
part of the template's schema, and never reaches the Typst code: it is written into the PDF
document properties after compilation.

| Field | Type | Constraints |
|-------|------|-------------|
| `title` | string | 1 to 500 characters |
| `author` | string, or array of strings | each 1 to 200 characters; 1 to 20 authors |
| `subject` | string | up to 2000 characters |
| `keywords` | array of strings | up to 50 keywords, each 1 to 100 characters |
| `date` | string | `YYYY-MM-DD`, a real calendar date |

Unknown keys and wrong types are rejected. For each field, the value comes from, in order:

1. the request's `metadata`;
2. the template's own `#set document(...)`;
3. the service defaults: the title is the template `name` (from `template.json`, or its
   identifier), the author is `INKPDF_DEFAULT_AUTHOR` (`inkpdf` by default).

Without a `date` in the request or the template, no date is written, which keeps the output
deterministic. See [Downloads and PDF metadata](../guides/downloads-and-metadata.md).

## Validation

The body is checked before any compilation:

- `data` and `layout` against the template's schema, after the `layout` defaults are applied;
- `metadata` against the service's fixed schema.

If anything is wrong, the service answers `422 validation-failed` with all the violations at
once, from the three sections together:

```json
{
  "type": "…",
  "title": "…",
  "status": 422,
  "code": "validation-failed",
  "templateId": "sample",
  "violations": [
    { "path": "/data/title", "schemaPath": "/properties/data/properties/title/minLength", "message": "…" },
    { "path": "/layout/align", "schemaPath": "/properties/layout/properties/align/enum", "message": "…" },
    { "path": "/metadata/date", "schemaPath": "/properties/date", "message": "\"2026-02-30\" is not a valid calendar date" }
  ]
}
```

A body that is not JSON, or not a JSON object, gets `400 invalid-json`; a missing or different
`Content-Type` gets `415 unsupported-media-type`; a body larger than `INKPDF_MAX_BODY_BYTES`
gets `413 payload-too-large`. All error codes are listed in [Error codes](../reference/errors.md).

## How Typst sees the input

The validated `data` and `layout` are converted to Typst values and exposed in `sys.inputs`:

```typst
#let data = sys.inputs.data
#let opts = sys.inputs.layout  // not `layout`: that would shadow Typst's layout() function
```

| JSON | Typst |
|------|-------|
| object | dictionary |
| array | array |
| string | `str` |
| integer | `int` (beyond the 64-bit range: `float`) |
| decimal number | `float` |
| boolean | `bool` |
| `null` | `none` |

If the schema does not declare `layout`, `sys.inputs.layout` is an empty dictionary.

Strings are never interpreted as Typst code: a title such as `#import "/etc/passwd"` is displayed
as is. For exact decimal values (amounts, quantities), prefer integers (for example cents) or
strings, since decimal numbers become floats; see [Best practices](../guides/best-practices.md).

## Query parameters

The render URL accepts two optional parameters that only change the response headers:

- `download=true` (or `1`) returns the PDF with `Content-Disposition: attachment` instead of
  `inline`;
- `filename=…` names the file (path characters are removed and `.pdf` is appended; default:
  `<templateId>.pdf`).

```bash
curl -s -OJ -H 'Content-Type: application/json' \
  -d @examples/requests/sample.json \
  'localhost:3000/templates/sample/render?download=true&filename=report-2026-10'
```

## Finding the shape of a template's body

Each template's contract is published by the running service:

- `GET /templates/{templateId}/schema` returns its `schema.json` as written;
- `GET /openapi.json` contains one render operation per valid template, with typed `data` and
  `layout` components and the shared `metadata` component;
- `GET /docs` renders that document and lets you send requests.

See [Exploring the API](../guides/exploring-the-api.md).
