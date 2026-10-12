# Using inkpdf from an AI assistant

inkpdf can serve the [Model Context Protocol](https://modelcontextprotocol.io) (MCP), the
standard way for an AI assistant such as Claude to use outside tools. With it, a user asks the
assistant for a document in plain words, and the assistant finds the template, asks for the
missing fields and hands back the PDF, without anyone writing an API call.

The MCP server is **off by default**. Like the HTTP API, it has **no authentication**: turn it
on only on a private network (see [Private network only](#private-network-only)).

## Turn it on

Set one environment variable, `INKPDF_MCP_ENABLED=true`. Nothing else changes: same image, same
templates, same port.

```sh
docker run --rm -p 3000:3000 \
  -v "$PWD/examples/templates:/templates:ro" \
  -e INKPDF_MCP_ENABLED=true \
  ghcr.io/ck-developer/inkpdf:latest
```

With Docker Compose, add it to the service's `environment`:

```yaml
    environment:
      INKPDF_MCP_ENABLED: "true"
```

The MCP server then answers at `/mcp`, on the same address and port as the HTTP API:

```text
http://<host>:3000/mcp
```

`<host>` is the address the assistant's computer uses to reach inkpdf: `localhost` on your own
machine, or the service's name or IP address on your network (for example
`inkpdf.internal`). The startup log says `mcp: true` and gives the `/mcp` path. When the
setting is off, `/mcp` answers `404`, like any unknown path.

See [Configuration](../reference/configuration.md) for the accepted values.

## Connect Claude Code

[Claude Code](https://code.claude.com/docs/en/mcp) connects to the MCP server directly over
HTTP, from your computer.

**Recommended: the inkpdf plugin.** One install connects the server and adds two skills, one
to generate documents and one to write templates:

```text
/plugin marketplace add ck-developer/inkpdf
/plugin install inkpdf@inkpdf
```

Claude Code asks for `server_url`, the base address of your inkpdf server without `/mcp`
(`http://localhost:3000` by default). See [Assistant skills](assistant-skills.md).

**By hand.** Without the plugin, add the server yourself. Use one way or the other, never
both: the plugin's server is also called `inkpdf`. Run, with your own address:

```sh
claude mcp add --transport http inkpdf http://localhost:3000/mcp
```

The server is added for you, in the current project. Two variants:

- `--scope user` makes it available in all your projects:

  ```sh
  claude mcp add --transport http --scope user inkpdf http://inkpdf.internal:3000/mcp
  ```

- `--scope project` writes it to a `.mcp.json` file at the root of the project, to share with
  your team. You can also write that file by hand. Keep `"type": "http"`: an entry without a
  `type` is read as a local program, not a URL.

  ```json
  {
    "mcpServers": {
      "inkpdf": {
        "type": "http",
        "url": "http://inkpdf.internal:3000/mcp"
      }
    }
  }
  ```

Check the connection:

```sh
claude mcp list
```

`inkpdf` should show as connected. In a Claude Code session, the `/mcp` command lists the
server and its three tools. To remove it: `claude mcp remove inkpdf`.

## Connect Claude Desktop

The desktop app needs a small bridge, because of how it reaches MCP servers:

- Its **Connectors** screen (custom connectors, added by URL) does not connect from your
  computer: the connection comes from
  [Anthropic's cloud](https://support.claude.com/en/articles/11175166-get-started-with-custom-connectors-using-remote-mcp),
  which cannot reach a server on a private network. Do not use it for inkpdf.
- Its configuration file, `claude_desktop_config.json`, describes local programs that the app
  starts on your computer (a `command` and its `args`). It does not take a URL.

So the app starts [`mcp-remote`](https://www.npmjs.com/package/mcp-remote) on your computer, a
small program that relays the app's requests to inkpdf over HTTP. It runs with
[Node.js](https://nodejs.org) 18 or later, which must be installed on the computer: the app uses
the system's Node.js for these programs.

1. In Claude Desktop, open **Settings**, then **Developer**, and click **Edit Config**. This
   opens `claude_desktop_config.json`:
   - macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
   - Windows: `%APPDATA%\Claude\claude_desktop_config.json`
2. Add the `inkpdf` entry, with your own address. If the file already has an `mcpServers`
   section, add the entry inside it.

   ```json
   {
     "mcpServers": {
       "inkpdf": {
         "command": "npx",
         "args": [
           "-y",
           "mcp-remote",
           "http://inkpdf.internal:3000/mcp",
           "--allow-http",
           "--transport",
           "http-only"
         ]
       }
     }
   }
   ```

3. Quit Claude Desktop completely and start it again. Closing the window is not enough: the
   file is read at startup.
4. In a conversation, click the **+** button under the message box, then **Connectors**:
   `inkpdf` is listed with its three tools.

The options:

- `--allow-http`: `mcp-remote` refuses plain `http://` addresses, except `localhost` and
  `127.0.0.1`. Keep this option for an address on your network. You can drop it if inkpdf sits
  behind a reverse proxy that serves HTTPS.
- `--transport http-only`: inkpdf only speaks the current transport (Streamable HTTP), so the
  bridge does not try the older one.
- `-y` lets `npx` download `mcp-remote` without asking. To pin its version, write
  `mcp-remote@0.14.3` instead of `mcp-remote`.

If `inkpdf` does not appear, read the logs:

- macOS: `~/Library/Logs/Claude/mcp-server-inkpdf.log` and `mcp.log`;
- Windows: the same files in `%APPDATA%\Claude\logs`.

A `command not found` or `npx` error means Node.js is missing. A connection error means the
computer cannot reach the address: try `curl http://inkpdf.internal:3000/health` from it.

## Try it by hand

To see the tools without an assistant, run the
[MCP Inspector](https://github.com/modelcontextprotocol/inspector): `npx
@modelcontextprotocol/inspector` opens a page in your browser where you enter
`http://localhost:3000/mcp` and call each tool yourself.

Or with `curl`:

```sh
curl -s localhost:3000/mcp \
  -H 'content-type: application/json' \
  -H 'accept: application/json, text/event-stream' \
  -H 'mcp-protocol-version: 2025-06-18' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

The answer lists `list_templates`, `get_template` and `render_template`. The `accept` header
must name both types, as MCP requires; otherwise the server answers `406`. A request with an
`Origin` header gets `403` (see [Security](../operations/security.md#the-mcp-server)).

## The tools

The server offers three tools. They are the same for every deployment: templates are not tools.
The assistant learns what a template needs from `get_template`. The tools see the templates the
service loaded at startup, as the HTTP API does; with hot reload on (development), a template
added, changed or removed in the volume shows up at the next listing (see
[Loading templates](../concepts/hot-reload.md)).

| Tool | Input | Result |
|------|-------|--------|
| `list_templates` | none | Every template with its id, name, description, default version, available versions and status (`valid`, or `invalid` with the reason). The same JSON as `GET /templates`. With no template loaded, the list is empty and the text says "No template is loaded." |
| `get_template` | `templateId`, and optionally `version` | The template's details and its JSON Schema: `data` and `layout`, with the required fields, types, allowed values, formats, defaults, and each variable's `description` and `x-role`. When the template ships one, a complete `example` input. The same JSON as `GET /templates/{templateId}`. `metadata` is described in the input schema of `render_template`. |
| `render_template` | `templateId`, `data`, and optionally `layout`, `metadata`, `filename`, `version` | A short text (`Rendered <file> from template <id> <version> (<size>).`, without the version when the template has none) and the PDF itself, as a file (`application/pdf`). |

`version` chooses which [version](../concepts/templates.md#versions) of the template to read or
render, with the same rules as the HTTP `version` parameter:

- absent: the template's default version, its highest valid one;
- a major version, such as `1`: the highest valid `1.x.y`;
- an exact version, such as `1.0.0`: that version.

A version that does not exist gives a `template-version-not-found` error listing the available
versions. Assistants should use the default unless the user asks for a specific version.

For `render_template`:

- `data`, `layout` and `metadata` are the three parts of the
  [request body](../concepts/request-body.md) of the HTTP API. Omitted `layout` properties take
  their default.
- `filename` is the file name without `.pdf`, cleaned like the HTTP `filename` parameter (see
  [Downloads and PDF metadata](downloads-and-metadata.md)). It defaults to the template id.
- The PDF is the one `POST /templates/{templateId}/render` returns for the same input: same
  validation, same defaults, same metadata, same engine, same bytes.
- Nothing is stored. The PDF exists only in the tool's answer.

All three tools only read: they change nothing on the server, and the assistant may call them
again safely.

## What the assistant is told

When it connects, the server gives the assistant these instructions:

> Everything you need is available through these tools; do not look for templates, schemas or
> examples in local files. Use `list_templates` to find a template matching the user's request,
> then `get_template` to read its schema and its example. Each variable has a description and
> may have an `x-role`: `ask` — ask the user unless they already gave the value; `fill` — fill
> it in yourself (for example today's date), and ask if you cannot work it out; `default` —
> leave the default unless the user mentions it, or leave the variable out if it has none.
> Without a role, treat a required variable as `ask` and an optional one with a default as
> `default`. Never invent a value you should have asked for. Start from the example's
> structure. Then call `render_template`; if it returns validation errors, ask the user about
> those fields only and call it again. Use a template's default version unless the user asks
> for a specific one; `get_template` and `render_template` take an optional `version`.

### How assistants use roles

The role of a variable, its `x-role` in the template's schema, tells the assistant who provides
the value (see [Template format](../reference/template-format.md#roles-x-role)):

| `x-role` | The assistant… |
|---|---|
| `ask` | asks the user, unless the user already gave the value in the conversation. |
| `fill` | fills it in itself from the context: today's date, a value derived from the user's answers, a rule given in the description. It asks when it cannot work the value out. |
| `default` | leaves the default value, or leaves the variable out, unless the user mentions it. |

So for a template whose `title` and `recipient` are `ask`, whose `date` is `fill` and whose
`status` and `layout` settings are `default`, a request such as "a document for Jane Doe" gets
one question (the title and Jane Doe's address, together), the date is today's, and the status
and appearance keep their defaults. A template without roles still works: the assistant treats
its required variables as `ask` and its optional ones as `default`.

The `example` returned by `get_template` shows the assistant a complete, valid input, so it gets
the structure and the formats right at the first attempt. The
[generate-document skill](assistant-skills.md) gives assistants the same steps in more detail.

## A worked conversation

With the example volume loaded and Claude connected, the user writes:

> Make a document for Jane Doe, 1 Example Street, 75001 Paris: 12 notebooks at 3.50 and a desk
> lamp at 49.90.

1. The assistant calls `list_templates` and picks `demo`, "Demo document".
2. It calls `get_template` with `{ "templateId": "demo" }`. The schema says that `title`,
   `recipient` and `lines` are `ask`, that `reference` and `date` are `fill` (build
   `DEMO-YYYY-NNN`; today's date), and that `urgent`, `status`, `note` and every `layout`
   setting are `default`. The `example` shows a complete input, with amounts as strings such
   as `"3.50"`.
3. The user gave the recipient and the lines, but no title, which is `ask`: the assistant asks
   for it, and only for it. The user answers "Office supplies".
4. It fills in the reference and the date itself, leaves the defaults out, and calls
   `render_template`:

   ```json
   {
     "templateId": "demo",
     "data": {
       "title": "Office supplies",
       "reference": "DEMO-2026-001",
       "date": "2026-10-12",
       "recipient": { "name": "Jane Doe", "address": "1 Example Street\n75001 Paris" },
       "lines": [
         { "label": "Notebooks", "quantity": 12, "amount": "3.50" },
         { "label": "Desk lamp", "quantity": 1, "amount": "49.90" }
       ]
     },
     "filename": "office-supplies"
   }
   ```

5. The tool answers `Rendered office-supplies.pdf from template demo 1.0.0 (20.1 KiB).` with the
   PDF, which the assistant hands to the user.

The user then asks for the same document "in pink". The assistant sends
`"layout": { "accentColor": "pink" }`, and the tool answers with an error:

```text
Validation failed: 1 violation(s)
- /layout/accentColor: "pink" does not match "^#[0-9a-fA-F]{6}$"
Fix the input and call render_template again.
```

The assistant learns that the colour must be a hex code, picks one for pink (or asks the user
which one), and renders again.

How the PDF is offered to the user (a file to open or save) depends on the assistant.

## Errors

When a call fails, the tool returns an error result that the assistant reads, rather than a
protocol failure. It carries:

- a text in plain words: the error's title and detail, one line per faulty field
  (`<path>: <message>`), and a closing hint that tells the assistant what to do;
- the same [problem object](../reference/errors.md) as the HTTP API (`type`, `code`, `title`,
  `status`, `detail`, `templateId`, `violations`), as structured content.

| `code` | Meaning | Closing hint |
|--------|---------|--------------|
| `validation-failed` | The input does not match the template's schema. Every faulty field is listed, not only the first. | Fix the input and call `render_template` again. |
| `invalid-parameter` | `filename` is not valid (longer than 200 characters), or `version` is neither a major version (`1`) nor an exact one (`1.0.0`). | Fix the input and call `render_template` again. |
| `payload-too-large` | The input is larger than `INKPDF_MAX_BODY_BYTES`. | Fix the input and call `render_template` again. |
| `template-not-found` | No template has this id. | Call `list_templates` to see the available templates. |
| `template-version-not-found` | The template has no version matching `version`; the available versions are listed. | Call `get_template` without a version to see the available versions. |
| `template-invalid` | The template failed to load; the reason is given. | This template cannot be used until the operator fixes it. |
| `overloaded` | Every render slot stayed busy for `INKPDF_QUEUE_TIMEOUT_SECS`. | Try again in a moment. |
| `render-timeout` | The render took longer than `INKPDF_RENDER_TIMEOUT_SECS`. | Try again in a moment. |
| `render-failed` | The template failed to compile with this input. | Tell the operator; the template failed to render. |

The first three, an unknown template and an unknown version are for the assistant to fix,
usually by asking the user. The others are not: the assistant retries later or tells the user.

Mistakes in the arguments themselves are refused before the tool runs: an argument the tool
does not declare, a missing `templateId` or `data`, or an argument of the wrong type (a `data`
that is not an object). They also come back as a tool error, so that the assistant always sees
them, but without structured content, since the HTTP API has no such error. The text names each
faulty argument and ends with "Fix the arguments and call the tool again.":

```text
Invalid arguments for render_template
- unknown argument `colour`; render_template takes data, filename, layout, metadata, templateId, version
- argument `data` must be an object, not an array
Fix the arguments and call the tool again.
```

Only an unknown tool name is a protocol error. What is *inside* `data`, `layout` and `metadata`
is always checked against the template's schema and reported as above.

## Limits

Renders started by an assistant share every limit of the HTTP API (see
[Limits and performance](../operations/limits.md)):

- **Input size**: `data`, `layout` and `metadata` together may not exceed
  `INKPDF_MAX_BODY_BYTES`; above it, the tool answers `payload-too-large`. The MCP endpoint
  accepts 64 KiB more than that for the protocol's own envelope, and refuses a larger request
  with HTTP `413`.
- **Render slots and queue**: renders from the HTTP API and from assistants take slots from the
  same `INKPDF_MAX_CONCURRENT_RENDERS`, and wait at most `INKPDF_QUEUE_TIMEOUT_SECS`.
- **Render time**: `INKPDF_RENDER_TIMEOUT_SECS`.
- **Sandbox**: no network, no file outside the template, input used only as data. See
  [Sandbox and determinism](../concepts/sandbox.md).

**Large PDFs.** The PDF is always returned whole, inside the tool's answer, encoded in base64,
which adds about a third to its size. Assistants limit what they accept: Claude Code, for
example, refuses an HTTP response above 16 MB, so a PDF above about 12 MB cannot reach it. For
very large documents, use the HTTP API.

## Private network only

The MCP server has **no authentication**, like the HTTP API: anyone who can reach `/mcp` can
list the templates and render documents. Run it on a private network, and connect assistants
from inside that network or through a VPN. Never expose it on the Internet.

Requests from a web page (they carry an `Origin` header) are refused, so a malicious site
cannot drive the server through a user's browser. The details are in
[Security](../operations/security.md#the-mcp-server).

Reaching inkpdf from the Internet with an access key, for example from a connector configured
on claude.ai, is a planned feature.

## Logs

Each tool call writes one `mcp.tool` log event with the tool, the template id and the version
used when there is one (`templateId`, `templateVersion`), the duration and the outcome: `ok`,
the error code, or `invalid-arguments` for a call refused for its arguments. Each render also
writes the usual
`render` event, with `interface` set to `mcp` (or `http` for the HTTP API). The input data is
never logged. See [Deployment → Logs](../operations/deployment.md#logs).
