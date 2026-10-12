---
name: write-template
description: "Write or improve an inkpdf template (folder with template.json, schema.json, main.typ and example.json) so that AI assistants can fill it: a description and an x-role for every variable, defaults for layout, dates with format: date, and a valid example. Use when the user wants a new document template for inkpdf or asks to improve one."
license: MIT OR Apache-2.0
metadata:
  author: inkpdf
  homepage: https://ck-developer.github.io/inkpdf/
---

# Write an inkpdf template

An inkpdf template is a folder that the inkpdf server loads from its templates folder. It holds
the document's Typst source and the JSON Schema of its input. Assistants then fill it through
the inkpdf MCP tools (`list_templates`, `get_template`, `render_template`). The goal: a template
that loads, that an assistant can fill without guessing, and whose example renders.

Reference files, to read when you need them:

- [template-format.md](template-format.md): the folder, `template.json`, the schema rules, roles,
  the example, formats, versions and limits;
- [typst-basics.md](typst-basics.md): reading the input in Typst, page setup, tables, fonts,
  dates and amounts;
- [packages.md](packages.md): the Typst packages bundled in the server, with their import line.

## 1. Understand the document

Ask the author, in one message:

- what the document is for and what it shows: sections, tables, totals, signatures;
- for each piece of information, who provides it:
  - the end user, when they ask for a document → role `ask`;
  - the assistant, from the context or from a rule → role `fill` (today's date, a reference
    built from a pattern, a value derived from other answers);
  - nobody unless someone mentions it → role `default` (options, appearance);
- what may change in the appearance (colour, font, density, optional blocks): it goes in
  `layout`, with defaults;
- whether they have a sample document (a PDF, a picture, a text) to start from.

To improve an existing template, read its files first. Add a `description` and an `x-role` to
every variable, and an `example.json`, without changing what the template accepts. Changes
that do (a closed object, a new required property, `format: "date"`, a narrower pattern) can
break existing callers: propose them to the author, and make them in a new version (step 7).

## 2. Find the templates folder

Create the template in the folder the server loads its templates from. No MCP tool reveals
it, so:

- ask the author where it is, or
- read their `compose.yaml` (or `docker run` command): it is the host side of the volume
  mounted on `/templates`. For `./my-templates:/templates:ro` it is `./my-templates`; in the
  inkpdf repository's `compose.yaml` it is `$INKPDF_TEMPLATES`, or `./examples/templates` when
  that variable is unset.

If the server runs on another machine that you cannot write to, write the folder locally and
tell the author to copy it. When in doubt, ask; never guess.

## 3. Create the folder

The template id is the folder name: lowercase letters, digits, `-` and `_`, at most 64
characters (`attendance-sheet`). Check with `list_templates` that the id is free. Create the
template in the versioned layout, starting at version `1.0.0`:

```text
<templates folder>/
└── attendance-sheet/
    └── 1.0.0/
        ├── template.json
        ├── schema.json
        ├── main.typ
        └── example.json
```

## 4. Write the files

Follow [template-format.md](template-format.md), in this order:

1. `template.json`: a `name`, a `description` that says what the document is and when to use it
   (assistants choose templates by it), and `"version": "1.0.0"`, equal to the folder name.
2. `schema.json`: `data` for the content, `layout` for the appearance. On every property, at
   every depth: a `description` and an `x-role`. Close every object with
   `"additionalProperties": false`, list the `required` properties, give every `layout`
   property a `default`, declare dates with `"format": "date"`, amounts and codes with a
   `pattern`, choices with an `enum`, and bounds on texts and lists.
3. `main.typ`: read `sys.inputs.data` and `sys.inputs.layout` (see
   [typst-basics.md](typst-basics.md)). Import only the bundled packages listed in
   [packages.md](packages.md), by name only.
4. `example.json`: a complete, realistic input (`data`, and optionally `layout` and
   `metadata`) that passes the schema, with obviously fictitious names and addresses.

## 5. Get the server to load it

- When the server runs with hot reload (`INKPDF_HOT_RELOAD=true`, which the inkpdf
  repository's `compose.yaml` sets for development), a new or changed template appears within
  a few seconds.
- Otherwise the server loads its templates at startup only: ask the author to restart it (for
  example `docker compose restart`), and wait until they confirm.

## 6. Check it through the MCP tools

1. `list_templates`: the template is listed with the status `valid`. If it is `invalid`, its
   `reason` says why (malformed JSON, a schema rule, an invalid role with its location, a wrong
   package import): fix the file and load it again.
2. `get_template`: the schema is the one you wrote, and `example` is present. If `example` is
   missing, the example failed its check: the next step shows why.
3. `render_template` with the example's `data`, `layout` and `metadata`: it succeeds. On
   `validation-failed`, fix the example or the schema; on `render-failed`, the diagnostics give
   the Typst file, line and message: fix `main.typ`.
4. Render a few variants: optional values left out, long texts, a list long enough to run over
   several pages, each `layout` setting changed.

Repeat until everything passes. Then show the author the variables with their roles, and the
rendered example.

If the inkpdf tools are not connected, say that you could not check the template, and give the
author the checks to run once the server is reachable.

## 7. New versions

Once callers use a version, do not edit its folder. To change the template, copy it:

1. copy `<id>/1.0.0/` to a new version folder, such as `<id>/1.1.0/` or `<id>/2.0.0/`;
2. set `version` in the copy's `template.json` to the new folder name;
3. make the changes in the copy, then load and check it as above.

Raise the major number when an existing input would stop being valid or would render in a way
callers do not expect (a removed or renamed property, a new required property, a narrower
constraint); the minor number for compatible additions; the patch number for fixes. Callers
get the highest valid version by default, and can ask for a major version (`1`) or an exact one
(`1.0.0`).

A template whose files sit directly in `<id>/` (flat layout) is a single version. To version
it, move its files into `<id>/<its version>/`. Never keep files at the root of `<id>/` next to
version folders: the template becomes invalid.

## Worked example: `demo`

The inkpdf example volume ships `demo`, a neutral template that uses every possibility above:
text the user gives (`title`, the recipient's name and address), values the assistant fills
(a `reference` built as `DEMO-YYYY-NNN`, a `date` with `"format": "date"`), a list of lines with
a quantity and an amount, a yes/no, a choice, an optional note, four `layout` settings with
defaults, `metadata` in its example, and the `zero` package to format the total. If the server
has it, call `get_template` with `demo` to read its schema and example. Excerpts are in
[template-format.md](template-format.md).
