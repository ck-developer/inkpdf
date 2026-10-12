# Writing a template

This guide builds the `demo` template of the repository's example volume
(`examples/templates/demo/1.0.0/`) step by step: a neutral document with a title, a reference, a
date, a recipient, a table of lines with a total, and a few appearance settings. Along the way,
it shows how to describe every variable so that callers, and AI assistants in particular, can
fill the template without guessing. The exact rules are in
[Template format](../reference/template-format.md).

An AI assistant can also write a template for you: see [Assistant skills](assistant-skills.md).

## 1. Start the service with hot reload

Create a folder for your templates and start inkpdf on it, with hot reload on so that every
change below is picked up without a restart. With Docker:

```sh
mkdir -p my-templates
docker run --rm -p 3000:3000 \
  -v "$PWD/my-templates:/templates:ro" \
  -e INKPDF_HOT_RELOAD=true \
  ghcr.io/ck-developer/inkpdf
```

Or, from a clone of the repository, with Docker Compose, which turns hot reload on:

```sh
INKPDF_TEMPLATES=./my-templates docker compose up --build
```

Hot reload is for development. In production, the service loads its templates at startup, and
you publish a template by restarting it (see [Loading templates](../concepts/hot-reload.md)).

## 2. List the variables and who provides them

Before writing any file, list what the document shows and, for each piece of information, who
provides it. That gives each variable its **role**:

| Variable | What it is | Who provides it | Role |
|---|---|---|---|
| `title` | text | the person asking for the document | `ask` |
| `reference` | a code such as `DEMO-2026-001` | built by the assistant when the user gives none | `fill` |
| `date` | a date | today's date, unless the user says otherwise | `fill` |
| `recipient` | name, address, optional e-mail | the person asking | `ask` (`email`: `default`) |
| `lines` | a list of label, quantity, unit amount | the person asking | `ask` |
| `urgent` | yes or no | nobody, unless someone says so | `default` |
| `status` | `draft` or `final` | nobody, unless someone says so | `default` |
| `note` | free text | only when someone gives one | `default` |
| accent colour, font, density, total shown | appearance | nobody, unless someone asks | `default` |

- `ask`: the assistant asks the user, unless the user already gave the value.
- `fill`: the assistant fills it in itself, from the context or a rule written in the
  description, and asks when it cannot work it out.
- `default`: the assistant leaves the default, or leaves the variable out.

The content goes in `data`; the appearance goes in `layout`.

## 3. Create the folder

The folder name is the template id: lowercase letters, digits, `-` and `_`. Inside it, create
one folder per version, starting with `1.0.0`:

```sh
mkdir -p my-templates/demo/1.0.0
```

Every file below goes into `demo/1.0.0/`. A template may also keep its files directly in
`demo/` (one version), but version folders let you add `2.0.0` later without touching `1.0.0`
(see [step 10](#10-publish-a-new-version)).

## 4. Describe the template: `template.json`

```json
{
  "name": "Demo document",
  "description": "Neutral demonstration of every template possibility: variable roles, kinds of variables, layout settings, metadata, a bundled package and an example.",
  "version": "1.0.0"
}
```

Assistants choose a template from its `name` and `description`: say what the document is and
when to use it. In a version folder, `version` must equal the folder name (or be left out).

## 5. Describe the input: `schema.json`

The schema defines what callers send. Every property, at every depth, gets a `description`
(what the value is, its unit and format, the rule to compute it) and an `x-role`. Every object
is closed with `"additionalProperties": false`, so that a misspelt key is refused instead of
being ignored.

The root and `data`:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["data"],
  "additionalProperties": false,
  "properties": {
    "data": {
      "type": "object",
      "additionalProperties": false,
      "required": ["title", "reference", "date", "recipient", "lines"],
      "properties": {
        "title": {
          "type": "string", "minLength": 1, "maxLength": 120,
          "description": "Document title.",
          "x-role": "ask"
        },
        "reference": {
          "type": "string", "pattern": "^[A-Z0-9-]{3,30}$",
          "description": "Document reference. Build it as DEMO-YYYY-NNN if the user gives none.",
          "x-role": "fill"
        },
        "date": {
          "type": "string", "format": "date",
          "description": "Document date, YYYY-MM-DD. Today's date unless the user says otherwise.",
          "x-role": "fill"
        },
        "recipient": { "…": "see below" },
        "lines": { "…": "see below" },
        "urgent": { "type": "boolean", "default": false, "description": "Shows an \"Urgent\" badge.", "x-role": "default" },
        "status": { "enum": ["draft", "final"], "default": "draft", "description": "Shown in the header; draft adds a watermark.", "x-role": "default" },
        "note": { "type": "string", "maxLength": 1000, "description": "Optional free text at the end.", "x-role": "default" }
      }
    },
    "layout": { "…": "see below" }
  }
}
```

- **Dates** use `"format": "date"`: the service then refuses an impossible date such as
  `2026-02-30` with `422 validation-failed` and the path `/data/date`, before Typst runs. A
  `pattern` alone would only check the shape.
- **A `fill` variable** carries its rule in its description: that is what the assistant reads.
- **`default` in `data`** is documentation only: the service applies defaults to `layout`,
  never to `data`. The template must handle a missing `urgent`, `status` or `note` (step 6).

A nested group and a list each have their own role, and so do their properties:

```json
"recipient": {
  "type": "object",
  "additionalProperties": false,
  "required": ["name", "address"],
  "description": "Who the document is for.",
  "x-role": "ask",
  "properties": {
    "name": { "type": "string", "minLength": 1, "maxLength": 120, "description": "Full name.", "x-role": "ask" },
    "address": { "type": "string", "maxLength": 300, "description": "Postal address; line breaks allowed.", "x-role": "ask" },
    "email": { "type": "string", "format": "email", "description": "Optional e-mail address.", "x-role": "default" }
  }
},
"lines": {
  "type": "array", "minItems": 1, "maxItems": 50,
  "description": "Lines of the document.",
  "x-role": "ask",
  "items": {
    "type": "object",
    "additionalProperties": false,
    "required": ["label", "quantity", "amount"],
    "properties": {
      "label": { "type": "string", "minLength": 1, "maxLength": 200, "description": "What the line is about.", "x-role": "ask" },
      "quantity": { "type": "integer", "minimum": 1, "maximum": 9999, "description": "Number of units.", "x-role": "ask" },
      "amount": { "type": "string", "pattern": "^[0-9]{1,9}(\\.[0-9]{1,2})?$", "description": "Unit amount in euros, dot as decimal separator, e.g. \"12.50\".", "x-role": "ask" }
    }
  }
}
```

Amounts are strings with a decimal pattern rather than JSON numbers, so that they stay exact
(see [Best practices](best-practices.md#exact-money-decimal-strings-and-typst-decimal)).

The appearance settings, all with a `default`, which the service fills in before validation:

```json
"layout": {
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "accentColor": { "type": "string", "pattern": "^#[0-9a-fA-F]{6}$", "default": "#1f4e79", "description": "Colour of the title and the table header, as #RRGGBB.", "x-role": "default" },
    "font": { "enum": ["Libertinus Serif", "New Computer Modern"], "default": "Libertinus Serif", "description": "Font of the document.", "x-role": "default" },
    "density": { "enum": ["normal", "compact"], "default": "normal", "description": "Spacing and text size.", "x-role": "default" },
    "showTotal": { "type": "boolean", "default": true, "description": "Prints the total under the table.", "x-role": "default" }
  }
}
```

## 6. Write the document: `main.typ`

The validated input is in `sys.inputs`, with every `layout` default filled in. A shortened
version of the demo's `main.typ`:

```typst
#import "@preview/zero": num

#let data = sys.inputs.data
#let opts = sys.inputs.layout // not `layout`: that name would hide Typst's layout() function

#let accent = rgb(opts.accentColor)
#let status = data.at("status", default: "draft") // defaults never apply to `data`
#let parse-date(s) = {
  let p = s.split("-").map(int)
  datetime(year: p.at(0), month: p.at(1), day: p.at(2))
}
#let money(x) = num(str(calc.round(x, digits: 2)), digits: 2, group: (size: 3, separator: ",", threshold: 4))

#set page(
  paper: "a4",
  margin: 2cm,
  footer: context align(right)[
    page #counter(page).display() / #counter(page).final().first()
  ],
)
#set text(font: opts.font, size: if opts.density == "compact" { 9.5pt } else { 11pt })

#text(20pt, weight: "bold", fill: accent)[#data.title] \
#data.reference · #parse-date(data.date).display("[month repr:long] [day padding:none], [year]") · #status

*#data.recipient.name* \
#data.recipient.address

#table(
  columns: (1fr, auto, auto),
  align: (left, right, right),
  fill: (_, y) => if y == 0 { accent },
  table.header(repeat: true, ..([Label], [Quantity], [Unit amount]).map(text.with(fill: white, weight: "bold"))),
  ..data.lines.map(l => (l.label, str(l.quantity), money(decimal(l.amount)))).flatten(),
)

#let total = data.lines.map(l => decimal(l.amount) * l.quantity).sum()
#if opts.showTotal { align(right)[*Total: #money(total) EUR*] }

#if "note" in data { emph(data.note) }
```

- Required properties and `layout` keys can be read directly (`data.title`, `opts.font`).
  Optional `data` properties need `.at(…, default: …)` or a `"key" in data` test.
- The total is computed with Typst's exact `decimal` type and formatted with the bundled `zero`
  package, imported by name only (see [Using bundled packages](packages.md)).
- The table header repeats on every page, and the footer reads "page x / n".

Paths in `#include`, `#import` and `#image` are relative to the file that uses them and must
stay inside the version folder.

## 7. Add an example: `example.json`

An example is a complete request body that passes the schema. The service checks it when it
loads the template, returns it as `example` in the template's details, and uses it as the
example request in `/docs`. AI assistants start from it.

```json
{
  "data": {
    "title": "Office supplies",
    "reference": "DEMO-2026-001",
    "date": "2026-10-12",
    "recipient": {
      "name": "Jane Doe",
      "address": "1 Example Street\n75001 Paris",
      "email": "jane.doe@example.com"
    },
    "lines": [
      { "label": "Notebooks", "quantity": 12, "amount": "3.50" },
      { "label": "Desk lamp", "quantity": 1, "amount": "49.90" },
      { "label": "Printer paper, 500 sheets", "quantity": 5, "amount": "6.20" }
    ],
    "urgent": false,
    "status": "draft"
  },
  "metadata": { "title": "Demo document DEMO-2026-001", "author": "Example Ltd" }
}
```

It leaves `layout` out, to show that the defaults are enough. An example that does not pass the
schema is not offered, and the service logs `template.example_invalid` with the reason; the
template itself stays valid.

## 8. Check that it loaded

```sh
curl -s localhost:3000/templates/demo | jq '{status, version, example: (.example | keys)}'
```

```json
{ "status": "valid", "version": "1.0.0", "example": ["data", "metadata"] }
```

If the template is invalid, the `reason` field says why, for example a misspelt role:

```json
{ "id": "demo", "status": "invalid", "reason": "schema.json: invalid role \"aks\" at /properties/data/properties/title/x-role; expected \"ask\", \"fill\" or \"default\"" }
```

## 9. Render it

Render the example as is:

```sh
curl -s localhost:3000/templates/demo | jq '.example' \
  | curl -s -H 'Content-Type: application/json' --data @- -o demo.pdf \
      localhost:3000/templates/demo/render
```

Change the appearance per request with `layout`; the properties you omit keep their default:

```json
{ "data": { "…": "…" }, "layout": { "accentColor": "#2a9d8f", "density": "compact" } }
```

An impossible date is refused before Typst runs:

```json
{
  "status": 422,
  "code": "validation-failed",
  "templateId": "demo",
  "violations": [
    { "path": "/data/date", "schemaPath": "/properties/data/properties/date/format", "message": "\"2026-02-30\" is not a valid calendar date (YYYY-MM-DD)" }
  ]
}
```

Each kind of mistake surfaces differently:

| Mistake | Response | Where to look |
|---|---|---|
| Broken `schema.json` or `template.json`, invalid role, wrong package import, missing `main.typ` | `409 template-invalid` | `reason` in `GET /templates/demo` |
| Body does not match the schema | `422 validation-failed` | `violations[]`: `path`, `schemaPath`, `message` |
| Typst error (unknown variable, missing key, bad type…) | `500 render-failed` | `diagnostics[]`: `message`, `file`, `line`, `column`, `hints` |

A Typst error looks like this:

```json
{
  "status": 500,
  "code": "render-failed",
  "templateId": "demo",
  "diagnostics": [
    { "message": "dictionary does not contain key \"note\"", "file": "main.typ", "line": 41, "column": 8 }
  ]
}
```

With hot reload, edit any file and render again: a change is applied once the folder has been
stable for about a second. Render the example after every change, with each `layout` setting,
and with a list long enough to run over several pages. To see it all in one place, open
`http://localhost:3000/docs`: your template appears with a typed request body and its example
as soon as it is valid (see [Exploring the API](exploring-the-api.md)).

## 10. Publish a new version

Once callers use `1.0.0`, do not edit it. Copy it to a new version folder, set the copy's
`version`, and change the copy:

```sh
cp -R my-templates/demo/1.0.0 my-templates/demo/1.1.0
# then set "version": "1.1.0" in my-templates/demo/1.1.0/template.json
```

Without a `version` parameter, callers now get `1.1.0`, the highest valid version; a caller can
ask for `?version=1.0.0` (exact) or `?version=1` (the highest `1.x.y`). Each render response
says which version it used in its `Inkpdf-Template-Version` header. Raise the major number
(`2.0.0`) when an existing request would stop being valid, for example when you add a required
property. See [Templates → Versions](../concepts/templates.md#versions).

## Next steps

- [Best practices](best-practices.md): schema design, roles, money, dates, multi-page tables,
  testing.
- [Using bundled packages](packages.md): QR codes, charts, number and date formatting.
- [Downloads and PDF metadata](downloads-and-metadata.md): file names, title, author.
