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
HTTP, from your computer. Run, with your own address:

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
The assistant learns what a template needs from `get_template`. Adding, changing or removing a
template in the volume shows up at the next listing, without a restart, as for the HTTP API.

| Tool | Input | Result |
|------|-------|--------|
| `list_templates` | none | Every template with its id, name, description, version and status (`valid`, or `invalid` with the reason). The same JSON as `GET /templates`. With no template loaded, the list is empty and the text says "No template is loaded." |
| `get_template` | `templateId` | The template's details and its JSON Schema: `data`, `layout` and `metadata`, with the required fields, types, allowed values, descriptions and defaults. The same JSON as `GET /templates/{templateId}`. |
| `render_template` | `templateId`, `data`, and optionally `layout`, `metadata`, `filename` | A short text (`Rendered <file> from template <id> (<size>).`) and the PDF itself, as a file (`application/pdf`). |

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

The server also gives the assistant the expected flow when it connects: find a template with
`list_templates`, read its schema with `get_template`, ask the user for every required field it
does not know (never invent values), keep the `layout` defaults unless the user asks otherwise,
then call `render_template`, and ask again about the fields named in a validation error.

## A worked conversation

With the example templates loaded and Claude connected, the user writes:

> Make me a one-page summary titled "Q3 expenses": rent 1,200, software 340, travel 515.

1. The assistant calls `list_templates`. Among `packages-demo`, `progress-invoice` and
   `sample`, it picks `sample`: "A title and a table of labels and values".
2. It calls `get_template` with `{ "templateId": "sample" }`. The schema says that
   `data.title` and `data.items` are required, each item having a `label` and a `value` (a
   number, 0 or more). `layout` has `primaryColor`, `align` and `showFooter`, all with
   defaults.
3. It already knows every required field, so it does not ask. It calls `render_template`:

   ```json
   {
     "templateId": "sample",
     "data": {
       "title": "Q3 expenses",
       "items": [
         { "label": "Rent", "value": 1200 },
         { "label": "Software", "value": 340 },
         { "label": "Travel", "value": 515 }
       ]
     },
     "filename": "q3-expenses"
   }
   ```

4. The tool answers `Rendered q3-expenses.pdf from template sample (14.4 KiB).` with the PDF,
   which the assistant hands to the user.

The user then asks for the same document "in pink". The assistant sends
`"layout": { "primaryColor": "pink" }`, and the tool answers with an error:

```text
Validation failed: the input does not match the template's schema.
- /layout/primaryColor: "pink" does not match "^#[0-9a-fA-F]{6}$"
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
| `invalid-parameter` | `filename` is not valid (longer than 200 characters). | Fix the input and call `render_template` again. |
| `payload-too-large` | The input is larger than `INKPDF_MAX_BODY_BYTES`. | Fix the input and call `render_template` again. |
| `template-not-found` | No template has this id. | Call `list_templates` to see the available templates. |
| `template-invalid` | The template failed to load; the reason is given. | This template cannot be used until the operator fixes it. |
| `overloaded` | Every render slot stayed busy for `INKPDF_QUEUE_TIMEOUT_SECS`. | Try again in a moment. |
| `render-timeout` | The render took longer than `INKPDF_RENDER_TIMEOUT_SECS`. | Try again in a moment. |
| `render-failed` | The template failed to compile with this input. | Tell the operator; the template failed to render. |

The first three, and an unknown template, are for the assistant to fix, usually by asking the
user. The others are not: the assistant retries later or tells the user.

Some mistakes are refused earlier, by the protocol layer, before inkpdf sees them: an unknown
tool name, a missing `templateId` or `data`, a `data` that is not an object, or an argument the
tool does not declare. The assistant gets a protocol error and usually corrects the call by
itself. What is *inside* `data`, `layout` and `metadata` is always checked against the
template's schema and reported as above.

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

Each tool call writes one `mcp.tool` log event with the tool, the template id when there is
one, the duration and the outcome (`ok` or the error code). Each render also writes the usual
`render` event, with `interface` set to `mcp` (or `http` for the HTTP API). The input data is
never logged. See [Deployment → Logs](../operations/deployment.md#logs).
