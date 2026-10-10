# Your first template

This page builds the smallest useful template: a greeting card with one piece of content and one
appearance parameter. It assumes the service runs with a templates folder mounted, as in
[Run with Docker](docker.md).

## 1. Create the folder

A template is a folder whose name is the template identifier (lowercase letters, digits, `-` and
`_`). Create it next to your other templates:

```bash
mkdir -p my-templates/greeting
```

## 2. Declare the input: `schema.json`

The schema says what a caller may send. Here, `data` holds the content (a required `name`) and
`layout` holds an appearance parameter (`color`) with a default value:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["data"],
  "additionalProperties": false,
  "properties": {
    "data": {
      "type": "object",
      "required": ["name"],
      "additionalProperties": false,
      "properties": {
        "name": { "type": "string", "minLength": 1, "maxLength": 80 }
      }
    },
    "layout": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "color": { "type": "string", "pattern": "^#[0-9a-fA-F]{6}$", "default": "#1f4e79" }
      }
    }
  }
}
```

Save it as `my-templates/greeting/schema.json`.

## 3. Write the document: `main.typ`

The validated input is available in `sys.inputs`, with the `layout` defaults already applied:

```typst
#let data = sys.inputs.data
#let opts = sys.inputs.layout  // not `layout`: that would shadow Typst's layout() function

#set page(paper: "a5", margin: 2cm)
#set text(font: "Libertinus Serif", size: 14pt)

#align(center + horizon)[
  #text(size: 28pt, weight: "bold", fill: rgb(opts.color))[Hello, #data.name!]
]
```

Save it as `my-templates/greeting/main.typ`. Typst reads `data.name` as a plain string: whatever
the caller sends is displayed as text, never executed.

## 4. Optional: describe it in `template.json`

```json
{
  "name": "Greeting",
  "description": "A one-line greeting card.",
  "version": "1.0.0"
}
```

Without this file, the template is listed under its folder name.

## 5. Check that it is loaded

If the service is already running with `my-templates` mounted on `/templates`, there is nothing
to restart: the template is picked up within a few seconds.

```bash
curl -s localhost:3000/templates/greeting
```

The response includes `"status": "valid"` and the schema. If the status is `invalid`, the
`reason` field explains why (missing file, malformed schema, wrong package import…).

## 6. Render it

```bash
curl -s -H 'Content-Type: application/json' \
  -d '{ "data": { "name": "Ada" } }' \
  -o greeting.pdf \
  localhost:3000/templates/greeting/render
```

The color defaults to `#1f4e79`. Override it per request, without touching the template:

```bash
curl -s -H 'Content-Type: application/json' \
  -d '{ "data": { "name": "Ada" }, "layout": { "color": "#b03a2e" } }' \
  -o greeting-red.pdf \
  localhost:3000/templates/greeting/render
```

The template now also appears in <http://localhost:3000/docs> with its own render operation,
built from this schema.

## 7. See a validation error

Send a body that does not match the schema:

```bash
curl -s -H 'Content-Type: application/json' \
  -d '{ "data": { "name": "" }, "layout": { "color": "red" } }' \
  localhost:3000/templates/greeting/render
```

The service answers `422` with an `application/problem+json` body whose `code` is
`validation-failed`. Its `violations` array lists every problem, each with the `path` in the
body (`/data/name`, `/layout/color`), the `schemaPath` of the rule that failed and a `message`.
Typst is not run when validation fails.

## Next steps

- [Templates](../concepts/templates.md): everything a template folder can contain.
- [Schemas and defaults](../concepts/schemas.md): the rules a schema must follow.
- [Writing a template](../guides/writing-a-template.md) and
  [Best practices](../guides/best-practices.md) for real documents.
