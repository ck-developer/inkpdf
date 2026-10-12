# inkpdf template format

## Folder

```text
<templates folder>/
└── <id>/                    # template id: ^[a-z0-9][a-z0-9_-]{0,63}$
    └── 1.0.0/               # one folder per version, named MAJOR.MINOR.PATCH
        ├── template.json    # optional: name, description, version
        ├── schema.json      # required: JSON Schema of the input
        ├── main.typ         # required: Typst entry point
        ├── example.json     # optional, recommended: a valid input
        ├── parts/           # optional: other .typ files
        ├── assets/          # optional: images and data files
        └── fonts/           # optional: .ttf, .otf or .ttc files, loaded automatically
```

- Each version folder is a complete template, loaded and checked on its own. Typst reads only
  the files inside it: no `../`, no absolute path, no network. Only `main.typ`, `schema.json`,
  `template.json` and `example.json` have fixed names.
- Only folders named with three numbers (`1.0.0`, `2.10.3`) are versions. Another sub-folder
  (`draft/`) is ignored, and the server logs it.
- Files at the root of `<id>/` next to version folders make the whole template invalid:
  `put the template files either at the root or in version folders, not both`.
- The flat layout, with the files directly in `<id>/`, still works: it is a single version,
  the one in its `template.json`. Use version folders for new templates.
- Files and folders whose name starts with `.` are ignored.

## `template.json`

```json
{
  "name": "Attendance sheet",
  "description": "List of the people present at a meeting, with a signature column.",
  "version": "1.0.0"
}
```

| Field | Rule |
|---|---|
| `name` | 1 to 120 characters; default: the template id |
| `description` | at most 2000 characters; assistants choose templates by it |
| `version` | at most 64 characters; in a version folder, it may be left out, and if given it must equal the folder name, otherwise that version is invalid |

Any other key makes the template invalid.

## `schema.json`

JSON Schema draft 2020-12. The input of a render has three parts: `data` (the content),
`layout` (the appearance) and `metadata` (the PDF's properties, defined by the server, never
declared in the schema).

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["data"],
  "additionalProperties": false,
  "properties": {
    "data": { "type": "object", "additionalProperties": false, "required": ["…"], "properties": { … } },
    "layout": { "type": "object", "additionalProperties": false, "properties": { … } }
  }
}
```

Rules the server enforces (the template is invalid otherwise):

- the root is `"type": "object"` and declares `properties.data`;
- only `data` and `layout` are declared under the root `properties`;
- every `$ref` is internal (`#/$defs/address`);
- every `x-role` is `"ask"`, `"fill"` or `"default"`.

Rules for a template that assistants fill well:

1. Every property under `data` and `layout`, at every depth (nested objects, array `items`,
   `$defs`), has a `description` and an `x-role`.
2. Every object is closed with `"additionalProperties": false`, so that a misspelt key is
   refused instead of ignored.
3. Every object lists its `required` properties.
4. Every `layout` property has a `default`. The server fills in the missing `layout` values
   before validation, recursively, so the template can read every `layout` key. Defaults are
   never applied to `data`: there, a `default` is documentation only, and the template must
   handle a missing optional value.
5. Dates are strings with `"format": "date"` (`YYYY-MM-DD`): impossible dates such as
   `2026-02-30` are refused. Other standard formats are checked too: `date-time`, `time`,
   `email`, `uri`, `uuid`, …; an unknown format name is ignored.
6. Amounts are strings with a decimal pattern, such as `^[0-9]{1,9}(\\.[0-9]{1,2})?$`, not JSON
   numbers (which become floats in Typst). Quantities are integers with a `minimum` and a
   `maximum`.
7. Choices are an `enum`. Texts have a `maxLength`, lists a `minItems` and a `maxItems`.
8. A description says what the value is, its unit and format, and, for a `fill` variable, the
   rule to compute it: "today's date unless the user says otherwise", "build it as
   DEMO-YYYY-NNN if the user gives none".

## Roles: `x-role`

| Value | Meaning for the assistant |
|---|---|
| `"ask"` | Ask the end user, unless they already gave the value. |
| `"fill"` | Fill it in from the context (today's date, a value derived from other answers, a rule given in the description); ask if it cannot be worked out. |
| `"default"` | Leave the default value, or leave the variable out, unless the end user mentions it. |

- Allowed on any property under `data` or `layout`, at any depth. Without a role, a required
  variable is treated as `ask` and an optional one as `default`.
- Any other value makes the template invalid, with the location:
  `schema.json: invalid role "maybe" at /properties/data/properties/title/x-role; expected "ask", "fill" or "default"`.
- A role never changes what input is accepted.

Choosing a role:

- `ask`: what only the user knows: names, addresses, the items of a list, amounts, free text.
- `fill`: what can be derived: today's date, a reference built from a pattern, a value computed
  from other answers. Put the rule in the description.
- `default`: options and optional content the user brings up only when they want it (a note, a
  flag, a status), and everything in `layout`.
- A list and the properties of its items each have their own role: the list can be `ask` and an
  item's position number `fill`.

Excerpt of a schema written this way, in the style of the `demo` template:

```json
"date": {
  "type": "string",
  "format": "date",
  "description": "Document date, YYYY-MM-DD. Today's date unless the user says otherwise.",
  "x-role": "fill"
},
"recipient": {
  "type": "object",
  "description": "Who the document is for.",
  "x-role": "ask",
  "additionalProperties": false,
  "required": ["name", "address"],
  "properties": {
    "name": { "type": "string", "minLength": 1, "maxLength": 120, "description": "Full name.", "x-role": "ask" },
    "address": { "type": "string", "maxLength": 300, "description": "Postal address; line breaks allowed.", "x-role": "ask" },
    "email": { "type": "string", "format": "email", "description": "Optional e-mail address.", "x-role": "default" }
  }
},
"status": {
  "enum": ["draft", "final"],
  "default": "draft",
  "description": "Shown in the header; draft adds a watermark.",
  "x-role": "default"
}
```

and in `layout`:

```json
"accentColor": {
  "type": "string",
  "pattern": "^#[0-9a-fA-F]{6}$",
  "default": "#1f4e79",
  "description": "Colour of the title and the table header, as #RRGGBB.",
  "x-role": "default"
}
```

## `example.json`

A render input, at the root of the version folder:

```json
{
  "data": { "…": "…" },
  "metadata": { "title": "Demo document DEMO-2026-001", "author": "Example Ltd" }
}
```

- `data` is required; `layout` and `metadata` are optional. Leave `layout` out to show that the
  defaults are enough.
- The server checks it when it loads the template, like a render input: `metadata`, then the
  schema with the `layout` defaults and the formats.
- Valid: `get_template` (and `GET /templates/<id>`) return it unchanged as `example`.
- Invalid or malformed: it is left out, the template stays usable, and the server logs a
  `template.example_invalid` event with the reason. Render the example with `render_template`
  to see the violations.
- Use realistic, obviously fictitious values, and fill every property a typical document has.

`metadata` accepts `title`, `author`, `subject`, `keywords` and `date`; its full schema is in
`render_template`'s input schema.

## Errors

| When | What you see |
|---|---|
| The template cannot be loaded | `list_templates` shows it `invalid` with a `reason`; a render returns `template-invalid` |
| The input does not match the schema | `validation-failed`, with every faulty value: `path` (`/data/date`), `message` |
| Typst fails to compile | `render-failed`, with `diagnostics`: `message`, `file`, `line`, `column`, `hints` |

Typst errors are only found when rendering: always render the example after a change.

## Limits

- A version folder is at most 50 MB by default (`INKPDF_MAX_TEMPLATE_BYTES`), and every version
  stays in memory.
- A render input is at most 5 MiB by default (`INKPDF_MAX_BODY_BYTES`).
- A render takes at most 30 seconds by default (`INKPDF_RENDER_TIMEOUT_SECS`).
- Fonts: Libertinus Serif, New Computer Modern and DejaVu Sans Mono are always available; add
  others in `fonts/`. System fonts are never used.
