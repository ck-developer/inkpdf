# Quickstart — validating the MCP server (004)

## Prerequisites

- The workspace builds: `cargo build -p inkpdf`.
- The example templates in `examples/templates/`.
- Optional: Claude Code, or `npx @modelcontextprotocol/inspector`.

## 1. Off by default (US3, SC-006)

```sh
INKPDF_TEMPLATES_DIR=examples/templates cargo run -p inkpdf
curl -si -X POST localhost:3000/mcp -H 'content-type: application/json' -d '{}'
```

Expected: `404`, and the startup log says `mcp: false`. `cargo test --workspace` passes with no
change to existing tests.

## 2. Turn it on (US3)

```sh
INKPDF_MCP_ENABLED=true INKPDF_TEMPLATES_DIR=examples/templates cargo run -p inkpdf
```

Expected: the startup log says `mcp: true` and gives the `/mcp` path.

## 3. Raw protocol check

```sh
curl -s localhost:3000/mcp \
  -H 'content-type: application/json' -H 'accept: application/json, text/event-stream' \
  -H 'mcp-protocol-version: 2026-07-28' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
```

Expected: exactly `list_templates`, `get_template`, `render_template`, as in
[contracts/mcp-tools.md](./contracts/mcp-tools.md).

Same request with `-H 'origin: https://evil.example'`: `403`.

## 4. With an assistant (US1, US2, SC-001, SC-005)

```sh
claude mcp add --transport http inkpdf http://localhost:3000/mcp
```

In Claude Code, ask: "Make me a progress invoice." Expected:
1. the assistant calls `list_templates`, then `get_template` on `progress-invoice`;
2. it asks for the required fields it does not know;
3. it calls `render_template` and returns a PDF matching the answers.

Then give a wrong value on purpose. Expected: the tool error names the field, and the assistant
asks again instead of giving up.

## 5. Same PDF through both interfaces (SC-002, SC-003, SC-004)

`cargo test -p inkpdf --test mcp` renders every example template through MCP and HTTP and
compares the bytes; it checks that invalid inputs list the same violation paths. `perf.rs`
compares timings.

## 6. Documentation

`cargo xtask docs check` passes (the new variable is documented) and `mdbook build docs` includes
the MCP page.
