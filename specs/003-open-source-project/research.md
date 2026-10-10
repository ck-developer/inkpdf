# Research — Open-source project foundation (003)

## R1 — Workspace layout

**Decision.** The root `Cargo.toml` is a workspace with `members = ["crates/inkpdf", "xtask"]`
and `resolver = "3"`.

- Shared settings move to `[workspace.package]` (edition, rust-version, license, repository),
  `[workspace.lints]` and `[profile.*]`.
- The service crate moves with `git mv`, so history is preserved: `src/`, `tests/`, `benches/`
  and `build.rs` go to `crates/inkpdf/`.
- `build.rs` reads `../../packages`, resolved from `CARGO_MANIFEST_DIR`.
- Test helpers resolve the repository root from `CARGO_MANIFEST_DIR/../..` (`examples/`,
  `openapi/`, `docs/`, `packages/`).
- The Dockerfile copies the workspace manifests, `crates/inkpdf` and `packages/`, then builds
  `-p inkpdf`. It does not copy `xtask`, which the workspace allows when the crate is absent:
  a stub `xtask/Cargo.toml` is created in the dependency layer.

**Rationale.**
- End-to-end tests and future components live in one change.
- The move is mechanical.

**Alternatives.**
- *Splitting the service into `inkpdf-core` and `inkpdf-server` now.* No second consumer yet
  (YAGNI, constitution VII).
- *Separate repositories.* Rejected by the user, because changes could not be tested end to
  end.

## R2 — Live, template-aware OpenAPI document

**Decision.** A new module `src/api/live.rs` builds the document. The static part comes from
`utoipa` as today, as `serde_json::Value`, built once.

For each **valid** template of the current registry snapshot, sorted by id, it adds:

- **Path.** `/templates/{id}/render` (concrete path), `post` operation:
  - `operationId: render_<id>` (`-` replaced by `_`);
  - `tags: ["templates (live)"]`;
  - `summary` = template name, `description` = template description and version;
  - the same query parameters and responses as the generic operation (copied by reference);
  - request body `$ref: #/components/schemas/<Prefix>RenderRequest`.
- **Components.**
  - `<Prefix>Data` and `<Prefix>Layout`: copies of `properties.data` and `properties.layout` of
    the template schema.
  - `<Prefix>RenderRequest`: `{ type: object, additionalProperties: false, required: <the
    template root's required>, properties: { data, layout, metadata: $ref DocumentMetadata } }`.
- **Prefix.** `<Prefix>` is PascalCase of the id: `progress-invoice` gives `ProgressInvoice`.
  A collision after normalisation (`a-b` and `a_b`) gets a numeric suffix.

Internal references are rewritten as follows:

1. Every `$defs` entry of the template becomes the component `<Prefix>_<name>`.
2. `#/$defs/<name>` becomes `#/components/schemas/<Prefix>_<name>`.
3. `#/properties/data/...` and other pointers into the template root are rewritten to point
   into the corresponding component.
4. `$schema` and `$id` are dropped from the copies.
5. Non-internal `$ref` cannot occur, because the loader rejects them in V1.

**Cache.**
- The handler takes `registry.snapshot()`, an `Arc<TemplateMap>`, and compares its pointer with
  the cached one, kept in `Mutex<Option<(Weak<TemplateMap>, Arc<Value>)>>`.
- It rebuilds only on a change and serves the cached `Arc<Value>`.
- Hot reload already replaces the map on every change, so identity is enough.

**Contract test.** `openapi/openapi.json` is the document of an **empty** registry. It is
byte-identical to today's file, apart from the deliberate removal of the `utoipa-scalar`
artefact, of which there is none in the document. A new test, `tests/api_live_openapi.rs`,
covers:
- per-template operations, schema copies and `$defs` rewriting;
- hot reload: add, change and remove a template;
- invalid templates omitted;
- the result validates as OpenAPI 3.1 structurally (paths, refs resolvable).

**`/docs` page.**
- `src/api/docs.rs` serves a small HTML page with the Scalar script pinned to an exact version
  on jsDelivr, with `data-url="/openapi.json"`. Each visit therefore shows the live document.
- `utoipa-scalar` is removed. The current one even loads an unpinned `@scalar/api-reference`.

**Rationale.**
- This is the user's request: the document is computed by the server on each call and follows
  the volume.
- A concrete path coexisting with the templated one is valid OpenAPI: concrete paths match
  first.
- Copying the schemas keeps the published document self-contained, so generators can consume
  it.

**Alternatives.**
- *`oneOf` over all templates in the generic body.* Generators produce poor types and the
  operation-template link is lost.
- *One document per template (`/templates/{id}/openapi.json`).* Useful later, but the user wants
  the main route and `/docs` to show everything.

## R3 — `xtask packages`

**Decision.** A new crate, `xtask`, run as `cargo xtask packages <cmd>` through a
`.cargo/config.toml` alias. Contract: [contracts/xtask-cli.md](./contracts/xtask-cli.md).

- **Version resolution.**
  - Read `https://packages.typst.org/preview/index.json`.
  - Candidates are the versions of `<name>`, keeping those whose `compiler` field is absent or
    `<=` the engine version.
  - The engine version is read from the workspace `Cargo.toml`, from the `typst` dependency
    requirement `=0.15.1`.
  - Pick the highest by SemVer.
- **Dependencies.**
  - Download the archive, then parse every `.typ` outside `tests/`, `docs/`, `examples/`,
    `gallery/` and `template/` with `typst-syntax`.
  - Collect the literal `@preview/x:y` imports.
  - Recursively add the missing `(x, y)` with role `dependency`.
- **Atomicity.**
  - Everything (archives and new lock content) is prepared in a temporary directory, then
    verified as `verify` would.
  - Only then are files moved into `packages/` and the docs regenerated. On any failure,
    nothing changes.
- **Update.** Replaces the `selected` entry. Then it removes unreferenced archives: a
  `dependency` entry that no remaining package imports is removed too.
- **`verify`.**
  - Digests, and the one-to-one match between archives and lock entries.
  - At most one `selected` per name.
  - Closure: every literal import is resolvable.
  - Generated docs up to date.
  - It runs in CI.
- **`list`.** A table of name, version, role, license and WASM.

**Rationale.** It replaces the bash script, with automatic dependencies (FR-015), atomicity, and
docs kept in sync (FR-016).

**Alternatives.**
- *Bash.* Fragile, as above.
- *utpm or gotpm.* No digest-pinned vendoring (see 002 discussion).

## R4 — Documentation site

**Decision.** mdBook in `docs/`.
- `book.toml`: title "inkpdf", English, `git-repository-url`, search enabled.
- Link check in CI: `mdbook build`, then `lychee --offline` on the generated HTML, which fails
  on broken internal links (FR-006).
- `docs/templates.md` and `docs/packages.md` move into the book.

Page structure: [contracts/docs-site.md](./contracts/docs-site.md).

**Generated pages** (`cargo xtask docs generate`, checked by `cargo xtask docs check`):
- `reference/api.md` from `openapi/openapi.json`: one section per operation (method, path,
  parameters, body schema summary, responses). It states that each template's typed operation
  is published live by the running service at `/openapi.json` and `/docs`.
- `reference/errors.md` from the `ErrorCode` enum in the OpenAPI document (code, status,
  title).
- `reference/packages.md` from `packages/lock.toml` and the archives (description, version,
  license, WASM, import line).

**Hand-written pages**: the conceptual pages (inkpdf as a foundation, template, dimensions
defined by the author's schema, best practices) and the guides.

**Rationale.**
- mdBook is the Rust ecosystem standard: one binary, Markdown, search, no JS toolchain.
- Generating the references keeps them true (FR-005).

**Alternatives.**
- *Docusaurus or Starlight.* Node toolchain, overkill.
- *Docs only in README.* Not a "real doc".

## R5 — Open-source hygiene files

**Decision.**
- `CONTRIBUTING.md`: setup, conventional commits, tests, adding a package, release flow.
- `CODE_OF_CONDUCT.md`: Contributor Covenant 2.1, full text.
- `SECURITY.md`: GitHub private vulnerability reporting, supported versions = latest release.
- `CHANGELOG.md`: managed by release-please.
- Issue forms `bug.yml` and `feature.yml`, plus `config.yml` linking docs and security.
- `PULL_REQUEST_TEMPLATE.md`: checklist of tests, docs, conventional title.
- `.editorconfig`.
- `.github/dependabot.yml`: weekly, ecosystems `cargo`, `github-actions` and `docker`.
- README: short (purpose, quick start, links, license) with badges: CI, license, latest release,
  docs, and image on GHCR.

## R6 — Image publication

**Decision.**
- **`ci.yml`.** New job `publish-dev` with `needs: [lint, test, perf, docs]`, running only on
  `push` to `main`.
  - Builds amd64 and arm64 and pushes `ghcr.io/ck-developer/inkpdf:dev` only.
  - `concurrency: publish-dev` with `cancel-in-progress: true`, so the latest commit wins.
- **`release.yml`.** On push to `main`, run `release-please-action`. When `release_created`,
  build and push `X.Y.Z`, `X.Y` and `latest` with `docker/metadata-action` semver patterns from
  the created tag.
- `docker.yml`, today tag-triggered with no tag yet, is removed.

**Rationale.** It matches the user's request: `dev` for testing from main, no per-commit tags,
and releases without manual tagging (SC-004).

## R7 — Versioning and release-please

**Decision.**
- `release-please-config.json`: package `crates/inkpdf`, `release-type: rust`,
  `bump-minor-pre-major: true` (a breaking change before 1.0 bumps the minor),
  `include-component-in-tag: false`, so tags are `vX.Y.Z`.
- `.release-please-manifest.json`: `{ "crates/inkpdf": "0.1.0" }`.
- `bootstrap-sha`: the merge commit of feature 002 (`90ee8fb`), so the first changelog covers
  003 onwards. Features 001 and 002 are summarised by hand in the initial `CHANGELOG.md`
  ("0.1.0 – unreleased history").
- The first release proposal will therefore be **0.2.0**.
- FR-013: `/health` and the OpenAPI `info.version` already use `CARGO_PKG_VERSION`, which
  release-please bumps in `crates/inkpdf/Cargo.toml`. The locked contract test normalises
  `info.version` so that a release bump does not break it.

**Alternatives.**
- *Manual tags.* More manual work, contrary to SC-004.
- *cargo-release.* Local and manual.

## R8 — Documentation publication

**Decision.** `docs.yml` runs on push to `main`, and on PRs as a check without deploying:
1. Install the pinned mdBook.
2. `cargo xtask docs check`.
3. `mdbook build docs`.
4. lychee offline check.
5. Upload and deploy with `actions/deploy-pages` (permissions `pages: write` and
   `id-token: write`).

**Rationale.** FR-003 and FR-006; under 30 minutes from merge (SC-003).

## R9 — README as entry point

**Decision.** The README is cut to a single screen, with:
- a one-paragraph pitch;
- a 3-command quick start (docker run, curl);
- links to the docs site, API docs (`/docs` of a running instance), examples and Bruno;
- the license.

The detailed content moves into the book.
