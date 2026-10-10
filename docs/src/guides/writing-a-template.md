# Writing a template

This guide builds a small template step by step: a delivery note with a title, a table of
items and a few appearance settings. It then shows how to iterate on it with hot reload. The
exact rules are in [Template format](../reference/template-format.md).

## 1. Start the service on a templates folder

Create a folder for your templates and start inkpdf on it. With Docker:

```sh
mkdir -p my-templates
docker run --rm -p 3000:3000 \
  -v "$PWD/my-templates:/templates:ro" \
  ghcr.io/ck-developer/inkpdf
```

Or, from a clone of the repository, with Docker Compose:

```sh
INKPDF_TEMPLATES=./my-templates docker compose up --build
```

The folder is watched: everything below is picked up without restarting the service.

## 2. Create the folder

The folder name is the template id. Use lowercase letters, digits, `-` and `_`:

```sh
mkdir -p my-templates/delivery-note/parts
```

## 3. Describe the input in `schema.json`

The schema defines what callers send: `data` for the content, `layout` for the appearance.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["data"],
  "additionalProperties": false,
  "properties": {
    "data": {
      "type": "object",
      "required": ["number", "date", "customer", "items"],
      "additionalProperties": false,
      "properties": {
        "number": { "type": "string", "minLength": 1 },
        "date": { "type": "string", "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}$" },
        "customer": { "type": "string", "minLength": 1 },
        "items": {
          "type": "array",
          "minItems": 1,
          "items": {
            "type": "object",
            "required": ["label", "quantity"],
            "additionalProperties": false,
            "properties": {
              "label": { "type": "string" },
              "quantity": { "type": "integer", "minimum": 1 }
            }
          }
        },
        "notes": { "type": "string" }
      }
    },
    "layout": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "primaryColor": { "type": "string", "pattern": "^#[0-9a-fA-F]{6}$", "default": "#1f4e79" },
        "showSignature": { "type": "boolean", "default": true }
      }
    }
  }
}
```

Every `layout` property has a `default`, so the template can always read it.

## 4. Write `main.typ`

```typst
#let data = sys.inputs.data
#let opts = sys.inputs.layout  // not `layout`: it would shadow Typst's `layout()` function

#let primary = rgb(opts.primaryColor)
#let parse-date(s) = {
  let p = s.split("-").map(int)
  datetime(year: p.at(0), month: p.at(1), day: p.at(2))
}

#set page(paper: "a4", margin: 2cm)
#set text(font: "Libertinus Serif", size: 11pt)

#text(size: 20pt, weight: "bold", fill: primary)[Delivery note #data.number]

#data.customer \
#parse-date(data.date).display("[day]/[month]/[year]")

#table(
  columns: (1fr, auto),
  table.header(
    table.cell(fill: primary, text(fill: white, weight: "bold")[Item]),
    table.cell(fill: primary, text(fill: white, weight: "bold")[Quantity]),
  ),
  ..data.items.map(i => (i.label, str(i.quantity))).flatten(),
)

#if "notes" in data { emph(data.notes) }

#if opts.showSignature [ #include "parts/signature.typ" ]
```

And `parts/signature.typ`:

```typst
#v(1fr)
#align(right)[Received by: #box(width: 6cm, repeat[.])]
```

Paths in `#include`, `#import` and `#image` are relative to the file that uses them and must
stay inside the template folder.

## 5. Add a manifest (optional)

`template.json` gives the template a readable name, a description and a version:

```json
{
  "name": "Delivery note",
  "description": "A delivery note with a list of items.",
  "version": "0.1.0"
}
```

## 6. Check that it loaded

```sh
curl -s localhost:3000/templates/delivery-note
```

The answer includes `"status": "valid"` and your schema. If the template is invalid, the
`reason` field says why, for example:

```json
{ "id": "delivery-note", "status": "invalid", "reason": "schema.json: expected `,` or `}` at line 12 column 5" }
```

## 7. Render it

Save a request body as `delivery-note.json`:

```json
{
  "data": {
    "number": "DN-0042",
    "date": "2026-10-10",
    "customer": "Example Ltd",
    "items": [
      { "label": "Steel bracket", "quantity": 12 },
      { "label": "Mounting kit", "quantity": 3 }
    ]
  }
}
```

and post it:

```sh
curl -s -H 'Content-Type: application/json' \
  -d @delivery-note.json \
  -o delivery-note.pdf \
  localhost:3000/templates/delivery-note/render
```

Change the appearance per request with `layout`; the properties you omit keep their default:

```json
{ "data": { "…": "…" }, "layout": { "primaryColor": "#2a9d8f", "showSignature": false } }
```

## 8. Iterate with hot reload

Edit any file of the folder and render again. A change is applied once the folder has been
stable for about a second (a periodic rescan, every 2 seconds by default, catches anything the
file watcher misses). While files are still being written, the previous version keeps being
served; once loaded, the new version replaces it, even if it is invalid. See
[Hot reload](../concepts/hot-reload.md).

Each kind of mistake surfaces differently:

| Mistake | Response | Where to look |
|---|---|---|
| Broken `schema.json` or `template.json`, wrong package import, missing `main.typ` | `409 template-invalid` | `reason` in `GET /templates/delivery-note` |
| Body does not match the schema | `422 validation-failed` | `violations[]`: `path`, `schemaPath`, `message` |
| Typst error (unknown variable, missing key, bad type…) | `500 render-failed` | `diagnostics[]`: `message`, `file`, `line`, `column`, `hints` |

A Typst error looks like this:

```json
{
  "status": 500,
  "code": "render-failed",
  "templateId": "delivery-note",
  "diagnostics": [
    { "message": "dictionary does not contain key \"note\"", "file": "main.typ", "line": 22, "column": 8 }
  ]
}
```

To see all of it in one place, open `http://localhost:3000/docs`: your template appears with a
typed request body as soon as it is valid (see [Exploring the API](exploring-the-api.md)).

## Next steps

- [Best practices](best-practices.md): schema design, money, dates, multi-page tables, testing.
- [Using bundled packages](packages.md): QR codes, charts, number and date formatting.
- [Downloads and PDF metadata](downloads-and-metadata.md): file names, title, author.
