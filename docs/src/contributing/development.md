# Development and tests

## Prerequisites

- Rust 1.99. `rust-toolchain.toml` pins the toolchain, with `rustfmt` and `clippy`; `rustup`
  installs it on first use.
- Docker, to build and run the image (optional).
- [mdBook](https://rust-lang.github.io/mdBook/), to preview this documentation site
  (optional).

The project is English-only: code, comments, documentation, commit messages, issues and pull
requests are all written in English.

## Running the service

```sh
INKPDF_TEMPLATES_DIR=examples/templates INKPDF_LOG_FORMAT=pretty cargo run -p inkpdf
```

Then open `http://localhost:3000/docs`. Use `--release` to measure anything: debug builds are
much slower. With Docker, `docker compose up --build` builds the image from source and mounts
`examples/templates`.

## Checks

These are the checks CI runs on every pull request; run them before pushing:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --release -p inkpdf --test perf -- --ignored
cargo xtask docs check
cargo xtask packages verify
```

## Tests

Integration tests live in `crates/inkpdf/tests/`. They build the application in-process
(`build_app`) against a temporary templates volume, with helpers in `tests/common/mod.rs`, and
need no network.

| File | Covers |
|------|--------|
| `api_render.rs` | `POST /templates/{id}/render`: success, every error code, downloads. |
| `api_templates.rs` | Template list, details and schema. |
| `api_layout.rs` | `layout` parameters and their defaults. |
| `api_metadata.rs` | PDF metadata and the default author. |
| `api_packages.rs` | `GET /packages`. |
| `api_docs.rs`, `api_live_openapi.rs` | `/docs` and the live, template-aware `/openapi.json`. |
| `contract_openapi.rs` | The static OpenAPI document equals `openapi/openapi.json`. |
| `hot_reload.rs` | Templates added, changed and removed while the service runs. |
| `render_limits.rs` | Render timeout, concurrency and queue. |
| `sandbox.rs` | Reads outside the template folder, outgoing links, packages, injection. |
| `bundled_packages.rs` | Every bundled package imports and works; the set is closed under imports; lock file, archives and docs agree. |
| `examples.rs` | Every example request renders; the progress invoice totals are exact. |
| `startup.rs` | Startup with many templates, many concurrent renders. |
| `perf.rs` | Latency guard (ignored by default, see below). |

Template fixtures are in `tests/fixtures/templates/` (one folder per scenario, such as `slow`
or `evil-parent`), and the per-package smoke templates in `tests/fixtures/package-smoke/`.
Unit tests sit next to the code, in `#[cfg(test)]` modules.

### OpenAPI contract

The static OpenAPI document (generic routes, no template loaded) is locked in
`openapi/openapi.json`. After an intended API change, regenerate it and commit the result:

```sh
INKPDF_UPDATE_OPENAPI=1 cargo test -p inkpdf --test contract_openapi
```

`info.version` is ignored by the comparison, so release version bumps do not break it.

### Performance

`tests/perf.rs` is ignored in normal runs. It must run in release mode:

```sh
cargo test --release -p inkpdf --test perf -- --ignored
```

It fails if the p95 render time exceeds 200 ms for the simple `sample` template, or 1 s for
`packages-demo`. For finer, comparable measurements, use the Criterion benchmark:

```sh
cargo bench -p inkpdf --bench render
```

### Debug-only message

In debug builds, a test that times out a render may print
`comemo: found differing return values` on the render thread. It is expected: see
[Limits and performance](../operations/limits.md#known-limitation-cancellation).

## Documentation

The site sources are in `docs/src/`; `docs/src/SUMMARY.md` defines the navigation.

```sh
mdbook serve docs -p 3001        # live preview on http://localhost:3001
cargo xtask docs generate        # regenerates the generated reference pages
cargo xtask docs check           # fails if a generated page is stale
```

The HTTP API, error codes and bundled packages reference pages are generated (they start with a
"generated" comment): do not edit them by hand. `docs check` also fails if an `INKPDF_*`
variable read by the service is missing from [Configuration](../reference/configuration.md).
Pages are published to GitHub Pages on every push to `main`.

## Pull requests

- Use [conventional commits](releasing.md#conventional-commits) for commit messages and PR
  titles.
- Add or update tests with the change; keep the OpenAPI contract and the generated docs up to
  date.
- Document user-visible changes in this site.
- Follow the project constitution (`.specify/memory/constitution.md`); see
  [Architecture](architecture.md#design-principles).
