# Schemas and defaults

Every template has a `schema.json`: a [JSON Schema](https://json-schema.org) (draft 2020-12) that
describes the request body. It is the template's contract with its callers. inkpdf uses it to
apply `layout` defaults, to validate requests, and to describe the template in the live API
documentation.

## A complete example

A schema for a document with a title and a table of labels and values:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["data"],
  "additionalProperties": false,
  "properties": {
    "data": {
      "type": "object",
      "required": ["title", "items"],
      "properties": {
        "title": { "type": "string", "minLength": 1 },
        "items": {
          "type": "array",
          "minItems": 1,
          "items": {
            "type": "object",
            "required": ["label", "value"],
            "properties": {
              "label": { "type": "string" },
              "value": { "type": "number", "minimum": 0 }
            }
          }
        }
      }
    },
    "layout": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "primaryColor": { "type": "string", "pattern": "^#[0-9a-fA-F]{6}$", "default": "#1f4e79" },
        "align":        { "enum": ["left", "center", "right"], "default": "left" },
        "showFooter":   { "type": "boolean", "default": true }
      }
    }
  }
}
```

Everything inside `data` and `layout` is up to the template author: the service has no
predefined fields.

## Rules for the root

1. The root must be `"type": "object"` and declare `properties.data`.
2. Only `data` and `layout` may be declared under `properties`. `metadata` is handled by the
   service and must not be declared.
3. If `additionalProperties` is absent at the root, the service treats it as `false`: unknown
   keys in a request body are rejected. The schema served by
   `GET /templates/{templateId}/schema` is still your file, byte for byte.
4. Only internal references are resolved: `$ref` values starting with `#` (for example
   `#/$defs/address`). An external reference (a URL or another file) makes the template invalid.
5. The schema must itself be a valid JSON Schema.
6. Every `x-role` annotation must be `"ask"`, `"fill"` or `"default"`.

A schema that breaks one of these rules makes the template invalid, with the reason shown in
`GET /templates`.

## Descriptions and roles

Next to its constraints, each property can say what it is and who provides it:

```json
"date": {
  "type": "string",
  "format": "date",
  "description": "Document date, YYYY-MM-DD; today's date unless the user says otherwise.",
  "x-role": "fill"
}
```

`description` documents the value for every caller. `x-role` tells an AI assistant whether to
ask the user for the value (`ask`), to fill it in itself (`fill`), or to leave its default
(`default`). Neither changes what a request may contain. See
[Template format](../reference/template-format.md#roles-x-role).

## `layout` defaults

Properties of `layout` should have a `default`, so that a request without any `layout` produces
a valid document. Before validation, the service:

1. sets `layout` to `{}` if the request omits it (only when the schema declares `layout`);
2. inserts the `default` of every missing property;
3. does the same recursively in nested objects of `layout`: a missing sub-object whose properties
   have defaults is created, and a partial sub-object is completed without overwriting the values
   that were sent.

For example, with this `layout` schema:

```json
{
  "type": "object",
  "properties": {
    "accent": { "type": "string", "default": "#1f4e79" },
    "header": {
      "type": "object",
      "properties": {
        "visible": { "type": "boolean", "default": true },
        "size":    { "type": "integer", "default": 12 }
      }
    }
  }
}
```

a request with `"layout": { "header": { "visible": false } }` reaches Typst as:

```json
{ "accent": "#1f4e79", "header": { "visible": false, "size": 12 } }
```

Defaults are applied to `layout` only, **never to `data`**: a `default` inside `data` is
documentation, not a value the template can rely on. Handle optional content in Typst, for
example with `data.at("note", default: none)`.

## Validation and errors

Validation runs after the defaults are applied and before Typst is invoked. All violations are
collected, not just the first one, and returned in a `422 validation-failed` response:

| Field | Meaning |
|-------|---------|
| `path` | JSON Pointer to the faulty value in the request body, such as `/data/items/0/value` (empty for the root) |
| `schemaPath` | JSON Pointer to the rule that failed in the schema, such as `/properties/data/properties/items/minItems` |
| `message` | a human-readable explanation |

Violations of `metadata` are returned in the same list, with paths starting with `/metadata`.
See [Error codes](../reference/errors.md).

## Formats are checked

`format` is **asserted**: a value whose property declares a standard format must match it, at
any depth. The checked formats are `date`, `date-time`, `time`, `duration`, `email`,
`hostname`, `ipv4`, `ipv6`, `uri`, `uri-reference`, `iri`, `iri-reference`, `uri-template`,
`uuid`, `regex`, `json-pointer` and `relative-json-pointer`; other format names are ignored.

Declare dates with `"format": "date"`: the value must then be a real calendar date written
`YYYY-MM-DD`. `2026-02-30` is refused before Typst runs, with the path of the field:

```json
{ "path": "/data/date", "schemaPath": "/properties/data/properties/date/format", "message": "\"2026-02-30\" is not a valid calendar date (YYYY-MM-DD)" }
```

A `pattern` such as `^[0-9]{4}-[0-9]{2}-[0-9]{2}$` only checks the shape: the impossible date
would reach Typst and fail there, without the field's name.

Earlier versions of inkpdf treated `format` as a hint. A template that declared a format loosely,
for example `"format": "email"` on a free-text field, now gets `422 validation-failed` for the
values that do not match. See
[Template format → Formats](../reference/template-format.md#formats).

## The schema in the API

The schema is exposed in three ways:

- `GET /templates/{templateId}` includes it in the template details;
- `GET /templates/{templateId}/schema` returns the file as written
  (`application/schema+json`);
- `GET /openapi.json` turns it into typed components for the template's own render operation
  (`POST /templates/{templateId}/render`): a `<Prefix>RenderRequest` with `data`, `layout` and
  the shared `metadata`, a `<Prefix>Data` and a `<Prefix>Layout` copied from your schema, and one
  component per `$defs` entry. `$schema` and `$id` are dropped from these copies and internal
  references are rewritten so that they resolve inside the document.

`GET /docs` renders that document, so a good schema (descriptions, examples, enums, bounds) is
also good documentation for callers. Guidance on designing schemas, such as exact amounts, dates
and naming layout parameters, is in [Best practices](../guides/best-practices.md).
