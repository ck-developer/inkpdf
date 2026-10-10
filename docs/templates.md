# Writing an inkpdf template

This guide is for template authors. The format described here is a versioned contract with
the API: any break in it is a major break of the service.

## Layout on disk

```text
<volume>/                       # INKPDF_TEMPLATES_DIR, mounted read-only
└── sample/                     # template identifier: ^[a-z0-9][a-z0-9_-]{0,63}$
    ├── main.typ                # REQUIRED — Typst entry point
    ├── schema.json             # REQUIRED — JSON Schema (draft 2020-12) of the input
    ├── template.json           # optional — metadata
    ├── fonts/                  # optional — .ttf / .otf / .ttc, loaded automatically
    └── assets/                 # optional — images and files read by main.typ
```

- The template identifier is the name of its folder. A folder whose name does not match the
  format is ignored (and logged at startup).
- Hidden files and folders (`.` prefix) are ignored.
- Symbolic links are followed as long as their target stays inside the template folder; a
  link pointing outside it is excluded (Kubernetes ConfigMaps: the links to `..data` stay
  inside the folder and are therefore followed).
- The total size of a template is limited by `INKPDF_MAX_TEMPLATE_BYTES` (50 MB by default);
  beyond that, the template is reported as `invalid`.
- `main.typ` can include or import other files from the same folder
  (`#include "parts/footer.typ"`, `#image("assets/logo.png")`).

A template is loaded entirely into memory when it is (re)discovered; a render never reads the
disk, so it always sees a complete and consistent version of the template.

## `template.json`

```json
{
  "name": "Sample",
  "description": "Title and a table of labels/values; demonstrates the format.",
  "version": "1.0.0"
}
```

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `name` | string (1–120) | no | folder identifier |
| `description` | string (≤ 2000) | no | absent |
| `version` | string (≤ 64) | no | absent (SemVer recommended) |

Any other key makes the template invalid (to catch typos).

## `schema.json`

A render request body has two sections:

- `data` (required): the document content, entirely defined by you;
- `layout` (optional): the appearance settings (color, alignment, blocks shown…).

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

Rules:

1. The root MUST be `type: object` and declare `properties.data`.
2. Only `data` and `layout` are allowed at the root. If `additionalProperties` is absent at the
   root, the service treats it as `false` (without changing the schema served by
   `GET /templates/{id}/schema`, which returns your file byte for byte).
3. The properties of `layout` SHOULD have a `default`. Before validation, the service sets
   `layout` to `{}` if it is absent and inserts the missing defaults, recursively into the
   sub-objects of `layout`. Defaults are **not** applied to `data`.
4. Only internal `$ref` references (`#/...`, `$defs`) are resolved; an external reference
   makes the template invalid.
5. For exact decimal values, prefer integers (cents) or strings: decimal numbers are passed to
   Typst as floats.

A non-conforming body is rejected (`422 validation-failed`) with the list of all violations,
without invoking Typst.

## Reading the data in `main.typ`

The validated input (with defaults applied) is available in `sys.inputs`:

```typst
#let data = sys.inputs.data
#let opts = sys.inputs.layout  // not `layout`: that name would shadow Typst's `layout()` function

#set text(fill: rgb(opts.primaryColor))
#let aligns = (left: left, center: center, right: right)
#align(aligns.at(opts.align))[= #data.title]
#table(columns: 2, ..data.items.map(i => (i.label, str(i.value))).flatten())
#if opts.showFooter [ #include "parts/footer.typ" ]
```

If the schema does not declare `layout`, `sys.inputs.layout` is an empty dictionary.

| JSON | Typst |
|------|-------|
| object | dictionary |
| array | array |
| string | `str` |
| integer | `int` (beyond the 64-bit range: `float`) |
| decimal number | `float` |
| boolean | `bool` |
| `null` | `none` |

Strings are **never** interpreted as Typst code: a title `#import "/etc/passwd"` is displayed
as is.

## Using a package

inkpdf bundles a selection of Typst Universe packages (QR codes, barcodes, charts, number and
date formatting…): see the list in [packages.md](./packages.md) or through `GET /packages`.

```typst
#import "@preview/zero": num
#import "@preview/tiaoma"

Amount: #num(sys.inputs.data.amount, digits: 2, decimal-separator: ",")
#tiaoma.qrcode(sys.inputs.data.reference)
```

Rules:

1. **A package is imported by name only**: `#import "@preview/<name>"` (with `: a, b` or
   `as x` if needed); `#include "@preview/<name>"` follows the same rule.
2. **No version**: the service uses the one it has installed (shown in `GET /packages`).
   `@preview/zero:0.7.1` is rejected.
3. **Only the offered packages** can be imported; `@preview` is the only namespace.
4. **Written literally**: `"@preview/" + name` (a computed import) is not supported.
5. **Files**: a package only reads its own files. To give it an image or data from the
   template, pass them in: `image("assets/logo.png")`, `read("data.csv")` or
   `path("assets/logo.png")`.
6. **Fonts**: a package's fonts are not loaded; use the service's fonts or those in `fonts/`.
7. A `packages/` folder inside a template is not used.
8. An inkpdf template does not compile as is with the standard `typst` tool, which requires a
   version.

Imports are checked **once, when the template is deployed**. An incorrect import makes it
`invalid`, with one line per error in `reason`:

```
main.typ:3: remove the version: write @preview/zero (inkpdf uses its installed version)
main.typ:4: package @preview/foo is not available in inkpdf (see GET /packages)
```

An error inside a package is reported with a file prefixed by the package, for example
`@preview/zero:0.7.1/src/num.typ`.

Complete example: [`examples/templates/packages-demo`](../examples/templates/packages-demo).

## Request body: `data`, `layout`, `metadata`

```json
{
  "data":     { "…": "content, validated against properties.data" },
  "layout":   { "…": "appearance, validated against properties.layout; defaults applied" },
  "metadata": { "title": "…", "author": ["…"], "subject": "…", "keywords": ["…"], "date": "2026-10-01" }
}
```

`metadata` is optional and is not part of the template schema: the service validates it with a
fixed schema (unknown keys and wrong types are rejected with `/metadata/...` paths) and writes it
into the PDF properties after compilation; it never reaches Typst code. Precedence for each
field: the request, then the template's own `set document(...)`, then the service defaults
(title = template `name`, author = `INKPDF_DEFAULT_AUTHOR`, `inkpdf` by default). Without a
`date`, no date is written (deterministic output).

To get a download instead of an inline PDF, add `?download=true` (and optionally
`&filename=invoice-042`) to `POST /templates/{templateId}/render`: the response then carries
`Content-Disposition: attachment` with a cleaned file name (`.pdf` appended).

## Sandbox restrictions

| Forbidden | Behavior |
|-----------|----------|
| package not offered, version written, other namespace | template `invalid` (`409 template-invalid`) |
| reading outside the folder (`../`, absolute path, outgoing link) | failure (`500 render-failed`) |
| network access, environment variables, system fonts | unavailable |

Compilation errors are returned in `diagnostics[]` with the file (relative to the template
folder), the line, the column and Typst's hints.

## Fonts

Fonts always available (embedded in the binary): Libertinus Serif, New Computer Modern,
DejaVu Sans Mono. Add your own in `fonts/` (TTF, OTF, TTC); an unreadable font file makes the
template invalid.

## Determinism

Two renders with the same (unchanged) template and the same body produce byte-identical PDFs.
Exception: `datetime.today()` is allowed but makes the document depend on the day it is
rendered; pass the date in `data` instead.

## Publishing and updating

Copy the folder into the volume: it is picked up within a few seconds, without a restart. A
change is applied only once the folder has stayed stable for one second; during a copy, the
previous version keeps being served. An invalid template is listed with `status: invalid` and
its `reason`, without affecting the others.
