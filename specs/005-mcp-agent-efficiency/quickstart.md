# Quickstart — validating feature 005

## 1. Automated checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask docs check     # API reference, portable skill frontmatter, plugin files, packages.md, no reference to removed templates
mdbook build docs
```

## 2. The example volume

```sh
INKPDF_MCP_ENABLED=true docker compose up --build -d
curl -s localhost:3000/templates | jq '.templates[].id'          # → "demo" only
curl -s localhost:3000/templates/demo | jq '.example | keys'      # → ["data", "metadata"]
curl -s localhost:3000/templates/demo/schema | jq '.. | objects | select(has("x-role")) | ."x-role"' | sort | uniq -c
```

Every property of `data` and `layout` has a description and a role (SC-002).

## 3. Example and dates

```sh
curl -s localhost:3000/templates/demo | jq '.example' > /tmp/demo.json
curl -s -o /tmp/demo.pdf -w '%{http_code}\n' -H 'content-type: application/json' \
  --data @/tmp/demo.json localhost:3000/templates/demo/render          # → 200 (SC-003)
jq '.data.date = "2026-02-30"' /tmp/demo.json | curl -s -H 'content-type: application/json' \
  --data @- localhost:3000/templates/demo/render | jq '.violations'    # → /data/date, "not a valid calendar date"
```

Same through MCP: `get_template` returns the example; `render_template` with the impossible date
returns `validation-failed` with `/data/date` (SC-004).

## 4. Roles in a template

Put a template with `"x-role": "maybe"` in the volume: `GET /templates` lists it as `invalid`, with
the reason naming the pointer and the value.

## 5. The plugin

```sh
claude plugin validate plugins/inkpdf
```

In Claude Code:

```text
/plugin marketplace add ./           # from the repository root, or ck-developer/inkpdf
/plugin install inkpdf@inkpdf        # server_url: http://localhost:3000
```

Open a new session and check:
- "Make me a demo document for Jane Doe with two lines": the assistant calls `list_templates`,
  `get_template`, asks only about the `ask` values it lacks (title, address, lines), fills the date
  and reference, and renders at the first attempt; it reads no local file (SC-001).
- "Write an inkpdf template for a meeting attendance sheet": the new template appears `valid` in
  `list_templates`, every variable has a description and a role, and its example renders (SC-005).
