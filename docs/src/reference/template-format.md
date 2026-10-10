# Template format

This page is the reference for the template format. The format is a versioned contract with
the API: breaking it is a major change of the service. For a guided tour, see
[Writing a template](../guides/writing-a-template.md).

## Folder layout

```text
<templates volume>/             # INKPDF_TEMPLATES_DIR, mounted read-only
└── invoice/                    # template id = folder name
    ├── main.typ                # required: Typst entry point
    ├── schema.json             # required: JSON Schema (draft 2020-12) of the request body
    ├── template.json           # optional: manifest
    ├── fonts/                  # optional: .ttf / .otf / .ttc files, loaded automatically
    ├── assets/                 # optional: images and files read by the Typst sources
    └── parts/                  # optional: any other .typ files, imported or included
```

| Rule | Detail |
|---|---|
| Template id | The folder name. It must match `^[a-z0-9][a-z0-9_-]{0,63}$`; other folders are ignored and logged at startup. |
| Hidden entries | Files and folders whose name starts with `.` are ignored. |
| Symbolic links | Followed while their target stays inside the template folder; a link pointing outside is excluded and logged (`template.link_excluded`). Kubernetes ConfigMap links to `..data` stay inside and are followed. |
| Size | The total size is limited by `INKPDF_MAX_TEMPLATE_BYTES` (50 MB by default); beyond it the template is `invalid`. |
| Other files | `main.typ` may import or include any file of the folder (`#include "parts/footer.typ"`, `#image("assets/logo.svg")`). Only `main.typ` and `schema.json` have fixed names. |
| `packages/` | A `packages/` folder inside a template is not used. Packages come from the service (see [Bundled packages](packages.md)). |

A template is read entirely into memory when it is (re)loaded. A render never reads the disk,
so it always sees one complete, consistent version of the template. See
[Hot reload](../concepts/hot-reload.md).

## Load order and validity

When a template is loaded, the service checks it in this order and stops at the first
failure. The template is then listed with `status: "invalid"` and a `reason`, and rendering it
returns `409 template-invalid`.

| Step | Example `reason` |
|---|---|
| Size limit | `template size 60000000 bytes exceeds the limit of 52428800 bytes` |
| `template.json` parses and is valid | `` template.json: unknown field `nmae`, expected one of … `` |
| Package imports of every `.typ` file | one line per error, `file:line: message` (see [Package imports](#package-imports)) |
| `main.typ` exists and is UTF-8 | `main.typ is missing` |
| `schema.json` is valid | `` schema.json: the root of the schema must declare `properties.data` `` |
| Every font in `fonts/` is readable | `fonts/Brand.ttf: no readable font` |

Typst compilation errors are **not** detected at load time: they are reported per render
(`500 render-failed` with `diagnostics[]`).

## `template.json`

```json
{
  "name": "Sample",
  "description": "A title and a table of labels and values; demonstrates the template format.",
  "version": "1.0.0"
}
```

| Field | Type | Required | Default |
|---|---|---|---|
| `name` | string, 1 to 120 characters | no | the template id |
| `description` | string, at most 2000 characters | no | absent |
| `version` | string, at most 64 characters | no | absent (SemVer recommended) |

Any other key makes the template invalid, to catch typos. The file itself is optional.

`name` is used as the default PDF title and as the summary of the template's operation in the
live OpenAPI document; `description` and `version` become that operation's description.

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
  "templateId": "sample",
  "violations": [
    { "path": "/layout/align", "schemaPath": "/properties/layout/properties/align/enum", "message": "…" }
  ]
}
```

`path` points into the request body, `schemaPath` into the schema.

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
