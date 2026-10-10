# Downloads and PDF metadata

A render returns the PDF bytes with `Content-Type: application/pdf`. Two independent controls
let callers shape what they receive: query parameters for the file name and download
behaviour, and the `metadata` section of the body for the PDF's own properties (title, author,
subject, keywords, date).

## Inline display or download

By default the response is meant to be displayed inline:

```text
Content-Disposition: inline; filename="invoice.pdf"; filename*=UTF-8''invoice.pdf
```

Add `?download=true` to ask browsers to save the file instead, and `filename` to name it:

```sh
curl -s -OJ -H 'Content-Type: application/json' \
  -d @examples/requests/sample.json \
  'localhost:3000/templates/sample/render?download=true&filename=sample-report'
```

```text
Content-Disposition: attachment; filename="sample-report.pdf"; filename*=UTF-8''sample-report.pdf
```

(`curl -OJ` saves the file under the name given by the header.)

| Parameter | Values | Default |
|---|---|---|
| `download` | `true`, `false`, `1`, `0` | `false`: `inline` |
| `filename` | at most 200 characters, URL-encoded | the template id |

Any other value of `download`, or a `filename` longer than 200 characters, is rejected with
`400 invalid-parameter`. Unknown query parameters are ignored. `filename` also applies to
inline responses (it is the name a browser proposes when the user saves the PDF).

### How the file name is cleaned

The service never trusts `filename` as a path. It:

1. removes control characters and the characters `/ \ " : * ? < > |`;
2. trims spaces and dots at both ends;
3. removes a trailing `.pdf` extension (in any case), then trims again;
4. falls back to the template id if nothing is left;
5. appends `.pdf`.

The header carries two forms of the name: `filename="…"`, where every non-ASCII character is
replaced by `_`, and `filename*=UTF-8''…`, percent-encoded, which modern clients prefer.

| `filename` sent | Saved as |
|---|---|
| `invoice-042` | `invoice-042.pdf` |
| `invoice-042.PDF` | `invoice-042.pdf` |
| `../../etc/passwd` | `etcpasswd.pdf` |
| `Café report 2026` | `Café report 2026.pdf` (ASCII fallback: `Caf_ report 2026.pdf`) |
| `...` | `<template id>.pdf` |

## PDF metadata

The optional `metadata` section of the request body sets the PDF document properties shown by
viewers and indexed by search tools:

```json
{
  "data": { "…": "…" },
  "metadata": {
    "title": "Invoice FA-2026-0142",
    "author": ["Example Ltd", "Accounts department"],
    "subject": "Construction progress invoice",
    "keywords": ["invoice", "FA-2026-0142"],
    "date": "2026-10-10"
  }
}
```

| Field | Type | Limits |
|---|---|---|
| `title` | string | 1 to 500 characters |
| `author` | string, or array of strings | each 1 to 200 characters; at most 20 |
| `subject` | string | at most 2000 characters |
| `keywords` | array of strings | each 1 to 100 characters; at most 50 |
| `date` | string `YYYY-MM-DD` | must be a real calendar date |

All fields are optional. `metadata` is defined by the service, not by the template: do not
declare it in `schema.json`. Unknown keys and wrong types are rejected with violations whose
paths start with `/metadata`, reported in the same `422 validation-failed` response as the
violations of `data` and `layout`:

```json
{
  "status": 422,
  "code": "validation-failed",
  "violations": [
    { "path": "/metadata/date", "schemaPath": "/properties/date", "message": "\"2026-02-30\" is not a valid calendar date" }
  ]
}
```

`metadata` never reaches the Typst code: it is applied to the compiled document, after
rendering.

### Precedence

For the **title** and the **author**, the first source that provides a value wins:

1. the request's `metadata`;
2. the template's own `set document(...)`;
3. the service defaults: the template's `name` (from `template.json`, or its id) for the
   title, and `INKPDF_DEFAULT_AUTHOR` for the author.

A template can therefore set a meaningful title from its data, and callers can still override
it:

```typst
#set document(title: [Invoice #data.invoice.number])
```

`subject`, `keywords` and `date` come from the request, when given (a template may also set
them with `set document`). Without a date from either source, **no date is written** to the PDF,
which keeps the output byte-identical between renders.

### `INKPDF_DEFAULT_AUTHOR`

The author written when neither the request nor the template sets one. It defaults to
`inkpdf`; surrounding spaces are trimmed. Set it to your organisation's name:

```sh
docker run --rm -p 3000:3000 \
  -e INKPDF_DEFAULT_AUTHOR="Example Ltd" \
  -v "$PWD/templates:/templates:ro" \
  ghcr.io/ck-developer/inkpdf
```

See [Configuration](../reference/configuration.md) for the other settings.
