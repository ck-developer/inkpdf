# Assistant skills

The inkpdf repository ships a [Claude Code](https://code.claude.com) plugin, `inkpdf`, that
connects an assistant to an inkpdf server and teaches it two jobs:

| Skill | For | What the assistant does |
|---|---|---|
| `generate-document` | people who ask for documents | Finds the right template with the inkpdf tools, reads its roles and example, asks only for the values the template marks `ask`, fills in the `fill` ones, keeps the defaults, renders, and goes back to the user about the fields named in a validation error. |
| `write-template` | template authors | Finds the templates folder, writes a template folder (`template.json`, `schema.json` with a description and a role on every variable, `main.typ`, `example.json`) in the versioned layout, gets the server to load it, and checks it through the inkpdf tools until its example renders. |

Both skills work with any template: nothing in them depends on a particular one. They rely on
the descriptions and roles of the template's schema (see
[Template format](../reference/template-format.md#roles-x-role)) and on the MCP server (see
[Using inkpdf from an AI assistant](mcp.md)).

The skills are plain Markdown files with a standard frontmatter, in
`plugins/inkpdf/skills/` of the repository. Besides Claude Code, any assistant that reads
[Agent Skills](https://agentskills.io) can use them, for example claude.ai and Claude Desktop.

## Before you start

The skills need an inkpdf server with the MCP server on (`INKPDF_MCP_ENABLED=true`), reachable
from your computer: see [Turn it on](mcp.md#turn-it-on). For `write-template`, the server
should also run with hot reload while you work on templates (`INKPDF_HOT_RELOAD=true`, which
the repository's `compose.yaml` sets), so that a new template shows up without a restart.

## Install the plugin in Claude Code

In Claude Code, add the repository as a plugin marketplace, then install the plugin:

```text
/plugin marketplace add ck-developer/inkpdf
/plugin install inkpdf@inkpdf
```

Claude Code asks for one setting, `server_url`: the base address of your inkpdf server,
**without** `/mcp`. The default, `http://localhost:3000`, fits a server started on your own
machine. The plugin then connects the MCP server `inkpdf` at `<server_url>/mcp` and adds the two
skills. To give the address from the command line instead:

```sh
claude plugin install inkpdf@inkpdf --config server_url=http://inkpdf.internal:3000
```

Start a new session. The `/mcp` command lists `inkpdf` with its three tools. The skills are
used on their own when a request matches them ("make me a document for…", "write an inkpdf
template for…"); you can also call them by name, as `/inkpdf:generate-document` and
`/inkpdf:write-template`.

`claude plugin configure inkpdf` shows the current `server_url`. The plugin's version follows
inkpdf's: update it with `claude plugin update inkpdf@inkpdf` when you upgrade the server, so
that the skills match the server's features and bundled packages.

### One MCP connection only

The plugin declares its MCP server under the name `inkpdf`, the same name as the manual setup
with `claude mcp add` described in [Connect Claude Code](mcp.md#connect-claude-code). Use one or
the other, **never both**. If you added the server by hand before, remove it first:

```sh
claude mcp list                 # shows where `inkpdf` is configured
claude mcp remove inkpdf        # add --scope user or --scope project if it was added there
```

To keep a hand-made connection (for example a project `.mcp.json` shared with your team), do
not install the plugin: upload the skills only, as described below for other assistants.

## Use the skills in claude.ai or Claude Desktop

claude.ai and Claude Desktop take skills as zip files, uploaded in the Skills section of their
settings (it needs code execution to be turned on). Each zip holds one skill folder.

1. Get the skill folders: from a clone of the repository, they are in
   `plugins/inkpdf/skills/`. Without cloning, download them from GitHub:

   ```sh
   curl -sL https://github.com/ck-developer/inkpdf/archive/refs/heads/main.tar.gz \
     | tar xz --strip-components=3 inkpdf-main/plugins/inkpdf/skills
   ```

2. Build the zip of a skill, from the `skills` folder, with one command:

   ```sh
   zip -r generate-document.zip generate-document
   ```

   and the same for `write-template` if you write templates.
3. Upload the zip in the Skills section of the settings.

The zip only carries the skill. Connect the MCP server separately:

- **Claude Desktop**: through the `mcp-remote` bridge, as in
  [Connect Claude Desktop](mcp.md#connect-claude-desktop);
- **claude.ai** in a browser: its connectors are reached from Anthropic's cloud, which cannot
  reach a server on a private network (see [Private network only](mcp.md#private-network-only)).
  The skills are of no use there until the server can be reached that way.

## What to expect

With `generate-document`, a request such as "a document for Jane Doe with two lines" leads the
assistant to call `list_templates` and `get_template`, ask one question with everything it
lacks, fill in what it can (today's date, a reference built by the rule in its description),
and render. It reads no local file. A validation error brings one more question, about the
faulty fields only.

With `write-template`, a request such as "write an inkpdf template for a meeting attendance
sheet" leads the assistant to ask what the document contains and who provides each value, then
to write the template folder and check it: the template is listed `valid`, every variable has a
description and a role, and its example renders. The
[Writing a template](writing-a-template.md) guide describes the same result step by step.
