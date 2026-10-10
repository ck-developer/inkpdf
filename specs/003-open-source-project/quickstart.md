# Quickstart — Validate feature 003

## 1. Workspace and tests

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --release -p inkpdf --test perf -- --ignored
```

Expected: every test from 002 passes unchanged, the `openapi/openapi.json` contract included,
and so do the new live-OpenAPI and xtask tests.

## 2. Live OpenAPI document

```bash
docker compose up --build -d
curl -s localhost:3000/openapi.json | jq '.paths | keys'
```

Expected: the generic routes, plus `/templates/packages-demo/render`,
`/templates/progress-invoice/render` and `/templates/sample/render`.

Then:
- copy `examples/templates/sample` to `examples/templates/sample-copy`, wait 3 s and query
  again: the path `/templates/sample-copy/render` is present;
- change a `layout` default in its schema: the change is reflected;
- delete the folder: the path is gone;
- open `http://localhost:3000/docs`: the same operations appear, each with its typed body.

## 3. Package command

```bash
cargo xtask packages list
cargo xtask packages verify                         # exit 0
cargo xtask packages add rowmantic --dry-run        # shows the version and dependencies to add
```

Corrupt an archive: `verify` exits 1 and names the package. Restore it.

## 4. Documentation

```bash
cargo xtask docs check
mdbook build docs && lychee --offline docs/book
mdbook serve docs        # http://localhost:3000 (stop compose first) or --port 3001
```

## 5. Delivery

This can only be checked on GitHub, after the merge:
- the `CI` workflow on `main` publishes `ghcr.io/ck-developer/inkpdf:dev` ;
- the `Docs` workflow deploys the site ;
- `Release` opens "chore(main): release 0.2.0" ;
- merging that PR publishes `0.2.0`, `0.2` and `latest`, plus the GitHub release.
