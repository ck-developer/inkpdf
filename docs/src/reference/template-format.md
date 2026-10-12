# Template format

This page is the reference for the template format. The format is a versioned contract with
the API: breaking it is a major change of the service. For a guided tour, see
[Writing a template](../guides/writing-a-template.md).

## Folder layout

```text
<templates volume>/             # INKPDF_TEMPLATES_DIR, mounted read-only
└── invoice/                    # template id = folder name
    ├── 1.0.0/                  # one folder per version, named MAJOR.MINOR.PATCH
    │   ├── main.typ            # required: Typst entry point
    │   ├── schema.json         # required: JSON Schema (draft 2020-12) of the request body
    │   ├── template.json       # optional: manifest
    │   ├── example.json        # optional: an example request body
    │   ├── fonts/              # optional: .ttf / .otf / .ttc files, loaded automatically
    │   ├── assets/             # optional: images and files read by the Typst sources
    │   └── parts/              # optional: any other .typ files, imported or included
    └── 2.0.0/
        └── …                   # a complete template again
```

This is the **versioned layout**. A template may also keep its files directly in its folder
(`invoice/main.typ`, `invoice/schema.json`…): the **flat layout**, a single version. See
[Versions](#versions).

| Rule | Detail |
|---|---|
| Template id | The folder name. It must match `^[a-z0-9][a-z0-9_-]{0,63}$`; other folders are ignored and logged at startup. |
| Template folder | A version folder (versioned layout), or the template folder itself (flat layout). Everything below applies to each template folder on its own. |
| Hidden entries | Files and folders whose name starts with `.` are ignored. |
| Symbolic links | Followed while their target stays inside the template folder; a link pointing outside is excluded and logged (`template.link_excluded`). Kubernetes ConfigMap links to `..data` stay inside and are followed. |
| Size | The total size of a template folder (each version folder) is limited by `INKPDF_MAX_TEMPLATE_BYTES` (50 MB by default); beyond it the version is `invalid`. |
| Other files | `main.typ` may import or include any file of the folder (`#include "parts/footer.typ"`, `#image("assets/logo.svg")`). Only `main.typ`, `schema.json`, `template.json` and `example.json` have fixed names. |
| `packages/` | A `packages/` folder inside a template is not used. Packages come from the service (see [Bundled packages](packages.md)). |

Each version is read entirely into memory when it is loaded, at startup (or on a change, with
hot reload in development). A render never reads the disk, so it always sees one complete,
consistent version of the template. See [Loading templates](../concepts/hot-reload.md).

## Versions

A template folder holds either its files (flat layout) or version folders (versioned layout),
never both.

| Situation | Result |
|---|---|
| Flat layout | One version: the `version` of `template.json`, or none. |
| Flat layout, manifest version `MAJOR.MINOR.PATCH` | Selectable like any version. |
| Flat layout, manifest version that does not parse (`1.0`, `v1`) | Shown as written, and used when no version is asked for; asking for any version gives `404 template-version-not-found`. |
| Versioned layout | One version per sub-folder named `MAJOR.MINOR.PATCH` (numbers only), each loaded and checked on its own, with its own folder as its Typst root. |
| Template files at the root **and** version sub-folders | Template `invalid`: `put the template files either at the root or in version folders, not both`. |
| Sub-folder that is not a version (`draft/`) | Skipped, and logged: `{"event":"template.version_ignored","templateId":"invoice","folder":"draft"}`. |
| `template.json` version different from its folder name | That version `invalid`: `template.json: version "1.2.0" does not match its folder "1.3.0"`. |

An invalid version never hides the valid ones. Versions are ordered numerically (`1.10.0` comes
after `1.9.0`). Callers choose a version with the `version` query parameter (HTTP) or argument
(MCP); without it they get the highest valid version. See
[Templates → Versions](../concepts/templates.md#versions).

To start a new version, copy the latest version folder to a new one (`1.0.0/` to `2.0.0/`), and
change the copy.

## Load order and validity

When a template is loaded, the service first checks its layout: a template that mixes files at
its root with version folders is invalid (see [Versions](#versions)). It then checks each
version in this order and stops at the first failure. The version is then listed with
`status: "invalid"` and a `reason`, and rendering it returns `409 template-invalid`.

| Step | Example `reason` |
|---|---|
| Size limit | `template size 60000000 bytes exceeds the limit of 52428800 bytes` |
| `template.json` parses and is valid | `` template.json: unknown field `nmae`, expected one of … `` |
| Package imports of every `.typ` file | one line per error, `file:line: message` (see [Package imports](#package-imports)) |
| `main.typ` exists and is UTF-8 | `main.typ is missing` |
| `schema.json` is valid, roles included | `` schema.json: the root of the schema must declare `properties.data` `` |
| Every font in `fonts/` is readable | `fonts/Brand.ttf: no readable font` |

Last, a valid version's `example.json`, if any, is checked like a request body. A failing
example is dropped and logged, but never makes the version invalid (see
[`example.json`](#examplejson)).

Typst compilation errors are **not** detected at load time: they are reported per render
(`500 render-failed` with `diagnostics[]`).

## `template.json`

```json
{
  "name": "Demo document",
  "description": "Neutral demonstration of every template possibility: variable roles, kinds of variables, layout settings, metadata, a bundled package and an example.",
  "version": "1.0.0"
}
```

| Field | Type | Required | Default |
|---|---|---|---|
| `name` | string, 1 to 120 characters | no | the template id |
| `description` | string, at most 2000 characters | no | absent |
| `version` | string, at most 64 characters | no | in a version folder, the folder name; in the flat layout, absent (`MAJOR.MINOR.PATCH` recommended) |

Any other key makes the template invalid, to catch typos. The file itself is optional. In a
version folder, `version` may be left out; if it is given, it must equal the folder name.

`name` is used as the default PDF title and as the summary of the template's operation in the
live OpenAPI document; `description` and `version` become that operation's description.
Assistants choose a template from its `name` and `description`: say what the document is and
when to use it.

## `schema.json`

A render request body has three sections:

- `data` (required): the document content, entirely defined by the template's schema;
- `layout` (optional): appearance settings, defined by the template's schema;
- `metadata` (optional): PDF properties, defined by the service, never declared in the schema
  (see [Downloads and PDF metadata](../guides/downloads-and-metadata.md)).

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["data"],
  "additionalProperties": false,
  "properties": {
    "data": {
      "type": "object",
      "required": ["title", "date", "items"],
      "additionalProperties": false,
      "properties": {
        "title": {
          "type": "string", "minLength": 1,
          "description": "Document title.", "x-role": "ask"
        },
        "date": {
          "type": "string", "format": "date",
          "description": "Document date, YYYY-MM-DD; today's date unless the user says otherwise.",
          "x-role": "fill"
        },
        "items": {
          "type": "array", "minItems": 1,
          "description": "Lines of the table.", "x-role": "ask",
          "items": {
            "type": "object",
            "required": ["label", "value"],
            "additionalProperties": false,
            "properties": {
              "label": { "type": "string", "description": "What the line is about.", "x-role": "ask" },
              "value": { "type": "number", "minimum": 0, "description": "Value of the line.", "x-role": "ask" }
            }
          }
        }
      }
    },
    "layout": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "primaryColor": {
          "type": "string", "pattern": "^#[0-9a-fA-F]{6}$", "default": "#1f4e79",
          "description": "Colour of the title, as #RRGGBB.", "x-role": "default"
        },
        "align": {
          "enum": ["left", "center", "right"], "default": "left",
          "description": "Alignment of the title.", "x-role": "default"
        },
        "showFooter": {
          "type": "boolean", "default": true,
          "description": "Prints the footer.", "x-role": "default"
        }
      }
    }
  }
}
```

### Root rules

1. The schema is JSON Schema **draft 2020-12**.
2. The root MUST be an object with `"type": "object"` and MUST declare `properties.data`.
3. Only `data` and `layout` may be declared under the root `properties`.
4. If the root has no `additionalProperties`, the service validates as if it were `false`.
   The schema served by `GET /templates/{templateId}/schema` is still your file, byte for byte.
5. The service does not add `"required": ["data"]` for you: declare it. (The live OpenAPI
   document uses your root `required`, or `["data"]` when it is absent.)
6. Only internal references are resolved: every `$ref` and `$dynamicRef` must start with `#`
   (for example `#/$defs/amount`). An external reference makes the template invalid.
7. A schema that is not valid JSON Schema makes the template invalid
   (`schema.json: invalid JSON Schema: …`).
8. Every `x-role` is `"ask"`, `"fill"` or `"default"` (see [Roles](#roles-x-role)).

### Descriptions

Give every property of `data` and `layout`, at every depth, a `description`. It shows up in
`/docs`, in generated clients and in what AI assistants read: say what the value is, its unit
and format, and, for a value an assistant fills in, the rule to compute it.

### Roles: `x-role`

`x-role` is an annotation written next to `description`. It tells an AI assistant who provides
the value:

| Value | Meaning for the assistant |
|---|---|
| `"ask"` | Ask the end user, unless they already gave the value. |
| `"fill"` | Fill it in yourself from the context (today's date, a value derived from other answers, a rule given in the description); ask if you cannot work it out. |
| `"default"` | Leave the default value (or leave the variable out) unless the end user mentions it. |

- **Where**: on any property schema under `data` or `layout`, at any depth: nested
  `properties`, array `items` and `prefixItems`, `additionalProperties`, `$defs`, `allOf`,
  `anyOf`, `oneOf`, `if`, `then`, `else`, `not` and the other keywords that hold sub-schemas.
  A list and the properties of its items each have their own role.
- **Fallback**: a variable without `x-role` is treated as `ask` when it is required, and as
  `default` when it is optional. A role is optional; give one everywhere anyway, so that
  assistants do not have to guess.
- **Checked at load**: any other value, or a value that is not a string, makes the template
  invalid. The reason names the value and its JSON Pointer in the schema:

  ```text
  schema.json: invalid role "maybe" at /properties/data/properties/title/x-role; expected "ask", "fill" or "default"
  ```

- **Never used to validate input**: a role does not change what a request may contain. The
  schema is returned with its roles by `GET /templates/{templateId}`, `/schema` and the MCP
  tool `get_template`, and the live OpenAPI document keeps them (`x-` keys are OpenAPI
  extensions).

Roles are how the [MCP server](../guides/mcp.md#how-assistants-use-roles) and the
[assistant skills](../guides/assistant-skills.md) avoid unneeded questions and invented values.

### `layout` defaults

Before validation, the service completes `layout`:

- if the schema declares `layout` and the body has none, `layout` is set to `{}`;
- every missing property of `layout` that has a `default` receives it;
- this applies recursively to sub-objects: a missing sub-object is created when any of its
  properties (at any depth) has a `default`, and a partial sub-object is completed without
  overwriting the values sent;
- defaults are **never** applied to `data`;
- if the schema does not declare `layout`, nothing is added, and Typst sees an empty
  dictionary.

Give every `layout` property a `default`: the template can then read any declared key without
checking that it exists. See [Schemas and defaults](../concepts/schemas.md).

### Validation

`metadata` is validated by the service's own schema; `data` and `layout` by the template's
schema, after defaults are applied. All violations are returned together in one
`422 validation-failed` response, and Typst is not invoked:

```json
{
  "type": "…",
  "title": "Validation failed",
  "status": 422,
  "code": "validation-failed",
  "templateId": "invoice",
  "violations": [
    { "path": "/data/date", "schemaPath": "/properties/data/properties/date/format", "message": "\"2026-02-30\" is not a valid calendar date (YYYY-MM-DD)" },
    { "path": "/layout/align", "schemaPath": "/properties/layout/properties/align/enum", "message": "…" }
  ]
}
```

`path` points into the request body, `schemaPath` into the schema.

### Formats

Template schemas are validated with `format` **asserted**: a value whose property declares a
standard format must match it, at any depth of `data` and `layout`. The checked formats are
`date`, `date-time`, `time`, `duration`, `email`, `hostname`, `ipv4`, `ipv6`, `uri`,
`uri-reference`, `iri`, `iri-reference`, `uri-template`, `uuid`, `regex`, `json-pointer` and
`relative-json-pointer`. Any other format name, `idn-email` and `idn-hostname` included, is
ignored.

`"format": "date"` takes a calendar date written `YYYY-MM-DD`. A date with the right shape that
does not exist, such as `2026-02-30`, `2026-13-45` or `2025-02-29`, is refused with this message
(shown here with its path, as the MCP server prints it):

```text
/data/date: "2026-02-30" is not a valid calendar date (YYYY-MM-DD)
```

`2024-02-29` is accepted. `date-time` is an RFC 3339 date and time with a time zone
(`2026-03-01T10:00:00Z`); its date is checked the same way. The other formats keep the
validator's message. Format violations are listed in the same `422 validation-failed` response
as every other violation.

> **Behaviour change.** Earlier versions of inkpdf treated `format` as a hint and accepted any
> string. A template that used `format` loosely, for example `"format": "email"` on a free-text
> field, now gets `422 validation-failed` for values that do not match. Remove the `format`, or
> send values that match it.

## `example.json`

A template folder may ship one example request body, `example.json`, next to `schema.json`:

```json
{
  "data": { "title": "Quarterly summary", "date": "2026-10-01", "items": [{ "label": "Rent", "value": 1200 }] },
  "metadata": { "title": "Quarterly summary", "author": "Example Ltd" }
}
```

- **Content**: a render body: `data` (required), and optionally `layout` and `metadata`.
- **Checked at load**, like a request: `metadata` against the service's schema, then `data` and
  `layout` against the template's schema, with the `layout` defaults and the formats.
- **Valid**: returned unchanged (without the `layout` defaults) as `example` in the template's
  details: `GET /templates/{templateId}` and the MCP tool `get_template`. The live OpenAPI
  document uses it as the example request body of the template's render operation. The list
  `GET /templates` does not carry it.
- **Invalid or malformed**: not returned, and logged; the template itself stays valid and
  usable. The log event gives the template id and the reason (violation paths and messages, or
  the JSON error), never the example's values:

  ```json
  {"event":"template.example_invalid","templateId":"demo","reason":"/data/date: \"2026-02-30\" is not a valid calendar date (YYYY-MM-DD)"}
  ```

A template without `example.json` has no `example` in its details. Give one anyway: it shows
callers and AI assistants a complete, valid input, and a test can render it as is.

## Inputs in Typst

The validated body (defaults applied, `metadata` removed) is exposed in `sys.inputs`:

```typst
#let data = sys.inputs.data
#let opts = sys.inputs.layout  // not `layout`: that name would shadow Typst's `layout()` function

#set text(fill: rgb(opts.primaryColor))
#let aligns = (left: left, center: center, right: right)
#align(aligns.at(opts.align))[= #data.title]
#table(columns: 2, ..data.items.map(i => (i.label, str(i.value))).flatten())
#if opts.showFooter [ #include "parts/footer.typ" ]
```

| JSON | Typst |
|---|---|
| object | `dictionary` |
| array | `array` |
| string | `str` |
| integer within the 64-bit range | `int` |
| integer outside the 64-bit range | `float` |
| number with a fraction or exponent | `float` |
| `true` / `false` | `bool` |
| `null` | `none` |

Strings are **never** interpreted as Typst code: a title `#import "/etc/passwd"` is printed as
is. Nothing else from the request reaches Typst.

## Package imports

Templates can import the packages bundled with the service. Imports are checked once, when
the template is loaded, in every `.typ` file of the folder:

1. Import by name only: `#import "@preview/<name>"`, optionally with `: a, b` or `as x`.
   `#include "@preview/<name>"` follows the same rule.
2. No version: the service substitutes the version it has installed (listed by
   `GET /packages`).
3. Only `@preview` packages listed in [Bundled packages](packages.md) are available.
4. The package name must be written literally: a computed import such as
   `"@preview/" + name` is not supported.

Each wrong import adds one line to the template's `reason`:

| Import | Message |
|---|---|
| `"@preview/zero:0.7.1"` | `main.typ:3: remove the version: write @preview/zero (inkpdf uses its installed version)` |
| `"@preview/foo"` | `main.typ:4: package @preview/foo is not available in inkpdf (see GET /packages)` |
| `"@local/foo"` | `main.typ:5: only @preview packages are available: @local/foo` |
| `"@preview"` | `main.typ:6: invalid package import "@preview"` |

Because imports have no version, an inkpdf template does not compile as is with the standard
`typst` command-line tool. More in [Using bundled packages](../guides/packages.md).

## Sandbox

| Not available | Result |
|---|---|
| Package not offered, version written, other namespace | template `invalid` (`409 template-invalid`) |
| Reading outside the template folder (`../`, outgoing link) | render fails (`500 render-failed`) |
| Network, environment variables, system fonts | do not exist for Typst |

A package can only read its own files. Compilation errors are returned in `diagnostics[]`
with the file (relative to the template folder, or `@preview/<name>:<version>/<path>` inside a
package), line, column and Typst's hints. See
[Sandbox and determinism](../concepts/sandbox.md).

## Fonts

Always available, embedded in the binary: **Libertinus Serif**, **New Computer Modern** and
**DejaVu Sans Mono**. Add your own as `.ttf`, `.otf` or `.ttc` files (extension matched
case-insensitively) anywhere under `fonts/`. A font file that contains no readable font makes
the template invalid. System fonts and fonts shipped inside packages are never loaded.

## Determinism

The same template version and the same body produce byte-identical PDFs. The PDF carries no
creation timestamp unless a date is given (`metadata.date` or the template's own
`set document(date: …)`). `datetime.today()` is allowed but makes the document depend on the
day of the render: pass dates in `data` instead.
