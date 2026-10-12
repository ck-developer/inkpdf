# Contract — The `inkpdf` plugin and its two skills

## Layout

```text
.claude-plugin/marketplace.json
plugins/inkpdf/
├── .claude-plugin/plugin.json
├── .mcp.json
└── skills/
    ├── generate-document/SKILL.md
    └── write-template/{SKILL.md, template-format.md, typst-basics.md, packages.md}
```

## `marketplace.json`

```json
{
  "name": "inkpdf",
  "owner": { "name": "inkpdf" },
  "plugins": [
    { "name": "inkpdf", "source": "./plugins/inkpdf",
      "description": "Generate PDF documents with an inkpdf server, and write inkpdf templates." }
  ]
}
```

## `plugin.json`

```json
{
  "name": "inkpdf",
  "version": "<crate version>",
  "description": "Connects an inkpdf server and adds two skills: generate a document from a template, and write a template.",
  "license": "MIT OR Apache-2.0",
  "repository": "https://github.com/ck-developer/inkpdf",
  "userConfig": {
    "server_url": {
      "type": "string",
      "title": "inkpdf server URL",
      "description": "Base URL of your inkpdf server, without /mcp. The server must run with INKPDF_MCP_ENABLED=true.",
      "default": "http://localhost:3000"
    }
  }
}
```

The version follows the crate version (release-please updates it; see the plan).

## `.mcp.json`

```json
{ "mcpServers": { "inkpdf": { "type": "http", "url": "${user_config.server_url}/mcp" } } }
```

## Skills

Both `SKILL.md` frontmatters use only `name`, `description`, `license`, `metadata`, four of the
six portable Agent Skills fields (`name`, `description`, `license`, `compatibility`, `metadata`,
`allowed-tools`; research R7).

### `generate-document`

- **description**: "Generate a PDF document with an inkpdf server: find the right template, ask the
  user only what the template marks as `ask`, fill in `fill` values, keep defaults, render, and fix
  validation errors with the user. Use when the user asks for a document, a PDF, a letter, a
  report or any file that an inkpdf template can produce."
- **Steps**:
  1. Use the inkpdf MCP tools only; do not search local files.
  2. `list_templates`; pick by name and description; if none fits or several do, ask the user.
  3. `get_template`; read every variable's description and `x-role`; start from `example`.
  4. Collect values: ask for `ask` variables not already given (group the questions in one
     message); fill `fill` variables, asking when unsure; leave `default` ones unless mentioned.
  5. `render_template`; on `validation-failed`, ask about the listed fields only and retry; on any
     other error, report it to the user as is.
  6. Tell the user the file name and offer it; never claim content you did not send.
- Template-independent: no template id, field name or business rule in the skill.

### `write-template`

- **description**: "Write or improve an inkpdf template (folder with template.json, schema.json,
  main.typ and example.json) so that AI assistants can fill it: a description and an x-role for
  every variable, defaults for layout, dates with format: date, and a valid example. Use when the
  user wants a new document template for inkpdf or asks to improve one."
- **Steps**: ask what the document contains and who provides each piece of information; find the
  templates folder (ask the user, or read the volume mounted in their `compose.yaml`; no MCP tool
  reveals it) and create the template folder there, in the versioned layout (`<id>/1.0.0/`); it is
  picked up at once with hot reload (development), otherwise ask the user to restart the server; write `template.json`, `schema.json` (closed objects,
  descriptions, roles, formats, defaults), `main.typ` (reads `sys.inputs`, uses bundled packages
  only), `example.json`; check through the MCP tools (`list_templates` shows it `valid`,
  `get_template` shows the example, `render_template` with the example succeeds); fix and repeat.
- **Reference files**, loaded when needed: `template-format.md` (folder, schema rules, roles,
  example, formats, limits), `typst-basics.md` (reading inputs, tables, page setup, fonts),
  `packages.md` (generated from `packages/lock.toml`: each bundled package, version and import
  line).
- Worked example: the `demo` template.

## Checks

- `cargo xtask docs check`: frontmatter keys within the portable set; `plugin.json` and
  `marketplace.json` parse; `plugin.json` version equals the crate version; `packages.md` up to
  date.
- Manual (quickstart): `claude plugin validate plugins/inkpdf` passes.
