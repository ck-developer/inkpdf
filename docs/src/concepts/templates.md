# Templates

A template is the unit of inkpdf: a folder in the templates volume that holds everything needed
to produce one kind of document. The service itself contains no document logic; it discovers
templates, validates requests against them and compiles them.

## Layout on disk

```text
<volume>/                       # INKPDF_TEMPLATES_DIR (/templates in the image), read-only
└── demo/                       # template identifier
    └── 1.0.0/                  # one folder per version
        ├── main.typ            # required: Typst entry point
        ├── schema.json         # required: JSON Schema (draft 2020-12) of the request body
        ├── template.json       # optional: name, description, version
        ├── example.json        # optional: an example request body
        ├── parts/              # optional: other Typst files (any names)
        ├── fonts/              # optional: .ttf / .otf / .ttc files, loaded automatically
        └── assets/             # optional: images and data files read by the Typst code
```

Each version folder is a complete template. Only `main.typ` and `schema.json` are required. The
other names are conventions, except `template.json` and `example.json`: any file inside the
folder can be included, imported or read from Typst with a relative path
(`#include "parts/footer.typ"`, `#image("assets/logo.png")`, `#read("data/terms.txt")`). The
one exception is `fonts/`: only font files in that folder (or its sub-folders) are loaded as
fonts.

A template can also keep its files directly in its folder, without version folders (`demo/main.typ`,
`demo/schema.json`…). This flat layout is a single version; see [Versions](#versions).

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
  "name": "Demo document",
  "description": "Neutral demonstration of every template possibility: variable roles, kinds of variables, layout settings, metadata, a bundled package and an example.",
  "version": "1.0.0"
}
```

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `name` | string (1 to 120 characters) | no | the folder identifier |
| `description` | string (up to 2000 characters) | no | absent |
| `version` | string (up to 64 characters) | no | in a version folder, the folder name; otherwise absent |

Any other key makes the template invalid, to catch typos. The `name` is used as the default PDF
title and as the summary of the template's operation in the live API documentation. In a
version folder, `version` may be left out; if it is given, it must equal the folder name.

## `schema.json`

The JSON Schema of the request body. It declares `data` (the content of the document) and,
usually, `layout` (its appearance parameters, with defaults). Each variable carries a
`description` and an `x-role` that tells AI assistants who provides its value (see
[Template format](../reference/template-format.md#roles-x-role)). Its rules are described in
[Schemas and defaults](schemas.md); how a caller fills it is described in
[The request body](request-body.md).

## `example.json`

An optional, complete request body that passes the schema. It is checked when the template is
loaded and, when valid, returned as `example` in the template's details and used as the example
request body in the live API documentation. An invalid example is dropped and logged; the
template stays valid. See [Template format](../reference/template-format.md#examplejson).

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
- The total size of each version folder is limited by `INKPDF_MAX_TEMPLATE_BYTES` (50 MB by
  default). Beyond that, the version is reported as invalid without its files being read.

## Versions

A template can keep several versions side by side, one sub-folder per version, named
`MAJOR.MINOR.PATCH` (numbers only):

```text
<volume>/
├── facture/              # versioned layout
│   ├── 1.0.0/            # main.typ, schema.json, template.json?, example.json?, assets…
│   ├── 1.1.0/
│   └── 2.0.0/
└── attestation/          # flat layout: one version
    ├── main.typ
    └── schema.json
```

- Each version folder is a complete, self-contained template: it is loaded and checked on its
  own, and Typst reads only the files of that folder. An invalid version never hides the valid
  ones.
- A template laid out flat, as `attestation` above, is a single version: the `version` of its
  `template.json`, if any.
- Files at the root of a template folder **and** version sub-folders make the template invalid.
  A sub-folder whose name is not a version (`draft/`) is skipped and logged. A `template.json`
  whose `version` differs from its folder name makes that version invalid.

Callers choose the version with `version`, a query parameter of the HTTP routes and an argument
of the MCP tools:

| `version` | Version used |
|---|---|
| absent | the highest valid version (the template's default version) |
| `1` | the highest valid `1.x.y` |
| `1.0.0` | exactly `1.0.0`; if it is invalid, `409 template-invalid` |
| `latest`, `v2`, `1.2`, empty | `400 invalid-parameter` |
| no matching version | `404 template-version-not-found`, with the available versions |

A flat template whose manifest version is not `MAJOR.MINOR.PATCH` (`1.0`, `v1`) is used when no
version is asked for; asking for any version gives `404 template-version-not-found`.

`GET /templates` lists each template with its default `version` and all its `versions`, with
their status. Every render response says which version was used, in the
`Inkpdf-Template-Version` header (HTTP) or in the result text (MCP). See
[Exploring the API](../guides/exploring-the-api.md#versions).

To start a new version, copy the latest version folder (`1.1.0/` to `2.0.0/`), set the copy's
`version`, and change it: callers that ask for `1` keep getting version 1.

## Loaded in memory

Templates are loaded when the service starts: all the files of each version are read into
memory at once. A render never reads the disk: it always sees a complete, consistent version of
the template. The set of templates then stays the same until the service restarts, unless hot
reload is turned on for development. See [Loading templates](hot-reload.md).

## Valid and invalid templates

A template is **invalid** when, for example:

- `main.typ` or `schema.json` is missing, or `main.typ` is not UTF-8;
- `schema.json` is not valid JSON, is not a valid JSON Schema, or breaks one of the
  [schema rules](schemas.md), such as an `x-role` other than `ask`, `fill` or `default`;
- `template.json` has an unknown key or a value out of bounds;
- a package import is incorrect;
- a font file is unreadable;
- the folder exceeds the size limit;
- its folder mixes template files and version folders, or the `version` of `template.json`
  differs from its version folder.

An invalid template (or version) never prevents the service from starting and never hides the
other templates (or versions). It stays listed in `GET /templates` with `"status": "invalid"` and a `reason`, and a
render request for it gets `409 template-invalid`. It is absent from the live OpenAPI document
until it is fixed.

The full format, field by field, is in [Template format](../reference/template-format.md).
