# Templates

A template is the unit of inkpdf: a folder in the templates volume that holds everything needed
to produce one kind of document. The service itself contains no document logic; it discovers
templates, validates requests against them and compiles them.

## Layout on disk

```text
<volume>/                       # INKPDF_TEMPLATES_DIR (/templates in the image), read-only
└── sample/                     # template identifier
    ├── main.typ                # required: Typst entry point
    ├── schema.json             # required: JSON Schema (draft 2020-12) of the request body
    ├── template.json           # optional: name, description, version
    ├── parts/                  # optional: other Typst files (any names)
    ├── fonts/                  # optional: .ttf / .otf / .ttc files, loaded automatically
    └── assets/                 # optional: images and data files read by the Typst code
```

Only `main.typ` and `schema.json` are required. The other names are conventions: any file inside
the folder can be included, imported or read from Typst with a relative path
(`#include "parts/footer.typ"`, `#image("assets/logo.png")`, `#read("data/terms.txt")`). The
one exception is `fonts/`: only font files in that folder (or its sub-folders) are loaded as
fonts.

## The identifier

The template identifier is the name of its folder. It must match
`^[a-z0-9][a-z0-9_-]{0,63}$`: lowercase letters, digits, `-` and `_`, starting with a letter or
a digit, at most 64 characters. It appears in every URL:

```text
GET  /templates/{templateId}
GET  /templates/{templateId}/schema
POST /templates/{templateId}/render
```

A folder whose name does not match is ignored (and logged at startup). Hidden files and folders
(starting with `.`) are ignored, and so are plain files at the root of the volume.

## `template.json`

An optional manifest describing the template:

```json
{
  "name": "Sample",
  "description": "A title and a table of labels and values; demonstrates the template format.",
  "version": "1.0.0"
}
```

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `name` | string (1 to 120 characters) | no | the folder identifier |
| `description` | string (up to 2000 characters) | no | absent |
| `version` | string (up to 64 characters) | no | absent (SemVer recommended) |

Any other key makes the template invalid, to catch typos. The `name` is used as the default PDF
title and as the summary of the template's operation in the live API documentation.

## `schema.json`

The JSON Schema of the request body. It declares `data` (the content of the document) and,
usually, `layout` (its appearance parameters, with defaults). Its rules are described in
[Schemas and defaults](schemas.md); how a caller fills it is described in
[The request body](request-body.md).

## `main.typ` and other Typst files

The entry point of the document. It reads the validated input from `sys.inputs.data` and
`sys.inputs.layout` and can split the document into several files. It can also import bundled
Typst packages by name, such as `#import "@preview/zero": num` (see
[Bundled packages](packages.md)).

## Fonts

Fonts embedded in the binary are always available: Libertinus Serif, New Computer Modern and
DejaVu Sans Mono. Add your own as `.ttf`, `.otf` or `.ttc` files in the template's `fonts/` folder; they
are only visible to that template. An unreadable font file makes
the template invalid. System fonts are never used.

## Symbolic links and size

- Symbolic links are followed as long as their target stays inside the template folder. A link
  pointing outside is excluded (and logged). Kubernetes ConfigMaps, whose links point to a
  `..data` folder inside the mount, therefore work.
- The total size of a template is limited by `INKPDF_MAX_TEMPLATE_BYTES` (50 MB by default).
  Beyond that, the template is reported as invalid without its files being read.

## Loaded in memory

When a template is discovered or changes, all its files are read into memory at once, and the
registry switches to the new version atomically. A render never reads the disk: it always sees a
complete, consistent version of the template, even while the folder is being rewritten. See
[Hot reload](hot-reload.md).

## Valid and invalid templates

A template is **invalid** when, for example:

- `main.typ` or `schema.json` is missing, or `main.typ` is not UTF-8;
- `schema.json` is not valid JSON, is not a valid JSON Schema, or breaks one of the
  [schema rules](schemas.md);
- `template.json` has an unknown key or a value out of bounds;
- a package import is incorrect;
- a font file is unreadable;
- the folder exceeds the size limit.

An invalid template never prevents the service from starting and never hides the other
templates. It stays listed in `GET /templates` with `"status": "invalid"` and a `reason`, and a
render request for it gets `409 template-invalid`. It is absent from the live OpenAPI document
until it is fixed.

The full format, field by field, is in [Template format](../reference/template-format.md).
