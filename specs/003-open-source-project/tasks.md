---

description: "Task list for feature 003: open-source project foundation"
---

# Tasks: Open-source project foundation

**Input**: Design documents from `specs/003-open-source-project/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: INCLUDED where behaviour changes. The constitution requires API contract tests,
and FR-005, FR-006, FR-014 and FR-016 require automated checks. In each story, tests are
written first and MUST fail before the implementation.

**Organization**: one phase per user story, in priority order: US1b and US1 (P1), US2 (P1),
US3 (P2), US4 (P2). The workspace move is foundational.

**Cross-cutting reminders**:
- Everything is written in English: code, docs, commit messages.
- No new runtime dependency in the service.
- `openapi/openapi.json` stays the locked document *without templates*.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelisable (different files, no dependency on an unfinished task)
- **[Story]**: US1, US1b, US2, US3 or US4
- Paths are relative to the repository root.

---

## Phase 1: Setup

- [X] T001 Make sure the license from PR #3 is on this branch. Once PR #3 is merged, rebase `003-open-source-project` onto `main`; if it is not merged yet, cherry-pick its commit. Check that `LICENSE-MIT` and `LICENSE-APACHE` are present.
- [X] T002 [P] Add `.editorconfig` at the root (UTF-8, LF, final newline, 4 spaces for `*.rs`, 2 spaces for `*.{json,yml,yaml,toml,md,typ}`, `max_line_length = 100` for `*.rs`).

---

## Phase 2: Foundational — Cargo workspace (blocks every story)

**Purpose**: move the service to `crates/inkpdf` without changing its behaviour (R1).

- [X] T003 With `git mv`, move `src/`, `tests/`, `benches/` and `build.rs` to `crates/inkpdf/`. Create `crates/inkpdf/Cargo.toml` from the current `Cargo.toml` package section, dependencies, dev-dependencies, build-dependencies and bench. The shared fields become `edition.workspace = true`, etc.
- [X] T004 Rewrite the root `Cargo.toml` as a workspace:
  - `[workspace]`: `members = ["crates/inkpdf", "xtask"]`, `resolver = "3"`;
  - `[workspace.package]`: edition, rust-version, license `MIT OR Apache-2.0`, repository;
  - `[workspace.lints]`, then move `[profile.dist]` here;
  - add a minimal `xtask/Cargo.toml` and `xtask/src/main.rs` printing usage, so the workspace builds.

  Add `.cargo/config.toml` with the alias `xtask = "run --package xtask --"`.
- [X] T005 Make every path depend on the workspace root:
  - `crates/inkpdf/build.rs` reads `CARGO_MANIFEST_DIR/../../packages` ;
  - `crates/inkpdf/tests/common/mod.rs::repo_root()` returns `CARGO_MANIFEST_DIR/../..` ;
  - check the hard-coded paths in tests: `examples/`, `openapi/`, `docs/`, `packages/`, `tests/fixtures` (fixtures move with the tests) ;
  - in `crates/inkpdf/src/api/...`, check for any `include_str!`.
- [X] T006 Update the `Dockerfile`:
  - dependency layer: copy `Cargo.toml`, `Cargo.lock`, `crates/inkpdf/Cargo.toml`, `crates/inkpdf/build.rs` and `packages/`, and create stubs for `crates/inkpdf/src` and `benches`, plus an `xtask/` stub (manifest and empty main) ;
  - build `cargo build --profile dist --locked -p inkpdf` ;
  - copy `target/dist/inkpdf`.

  Update `.dockerignore` (keep `xtask` out of the context; the stub is created inline). Update `compose.yaml` if needed.
- [X] T007 Update `.github/workflows/ci.yml`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo test --release -p inkpdf --test perf -- --ignored`, with the rust-cache keys adjusted.
- [X] T008 Run the full validation: `cargo test --workspace` (141 tests), clippy, `INKPDF_UPDATE_OPENAPI` not needed (no diff on `openapi/openapi.json`), `docker compose build` and `docker compose up`, then render `sample`. Commit with the message `refactor: move the service into a Cargo workspace (crates/inkpdf)`.

**Checkpoint**: same behaviour, same contract, same image.

---

## Phase 3: User Story 1b — Live, template-aware OpenAPI document (P1) 🎯 MVP

**Goal**: `/openapi.json` and `/docs` reflect the templates loaded at call time ([contracts/live-openapi.md](./contracts/live-openapi.md)).

**Independent Test**: read the document, add, change and remove a template, read it again (quickstart §2).

### Tests for User Story 1b ⚠️

- [ ] T009 [P] [US1b] Create `crates/inkpdf/tests/api_live_openapi.rs` with these tests:
  - (a) with `sample`, `packages-demo` and `progress-invoice` loaded, `paths` contains `/templates/<id>/render` for each, with `operationId` `render_sample`, `render_packages_demo` and `render_progress_invoice`, and summary = template name ;
  - (b) `components.schemas.ProgressInvoiceRenderRequest.properties` has `data`, `layout` and `metadata`, and `required == ["data"]` ;
  - (c) `$defs` are hoisted: `ProgressInvoice_amount` exists, and no `#/$defs/` remains anywhere in the document ;
  - (d) every `$ref` in the document resolves to an existing component ;
  - (e) an invalid template (fixture `invalid-schema`) has no typed path ;
  - (f) hot reload: write a new template into the volume, call `registry.refresh()` twice (confirmation), and its path appears; change a `layout` default and it is reflected; remove the folder and the path is gone ;
  - (g) with an empty volume, the document equals the static document returned by `inkpdf::openapi()` ;
  - (h) two ids colliding after PascalCase (`a-b`, `a_b`) get distinct prefixes.
- [ ] T010 [P] [US1b] Update `crates/inkpdf/tests/api_docs.rs`: `/docs` returns HTML containing `data-url="/openapi.json"` and a Scalar script pinned to an exact version (regex `@scalar/api-reference@\d+\.\d+\.\d+`).
- [ ] T011 [P] [US1b] Update `crates/inkpdf/tests/contract_openapi.rs`: compare the document of an empty registry with `openapi/openapi.json`, ignoring `info.version` (normalise it before comparing and when regenerating), so that release bumps do not break the contract (R7).

### Implementation for User Story 1b

- [ ] T012 [US1b] Create `crates/inkpdf/src/api/live.rs` (R2) :
  - `fn build(base: &Value, templates: &[Arc<TemplateEntry>]) -> Value` adds the operations and components following the contract ;
  - helpers `prefix(id, taken)` and `rewrite_refs(value, prefix)`, which hoist `$defs`, rewrite `#/$defs/...` and `#/properties/...` pointers, and drop `$schema` and `$id` ;
  - unit tests for `rewrite_refs` and `prefix`.
- [ ] T013 [US1b] Add the live document cache in `crates/inkpdf/src/api/live.rs`: a `LiveDoc` stored in `AppState` (or a static), holding `Mutex<Option<(Weak<TemplateMap>, Arc<Value>)>>`, rebuilt when the `registry.snapshot()` pointer changes. In `crates/inkpdf/src/api/mod.rs`, make `openapi_json` return `Json<Value>` from the cache. Keep `inkpdf::openapi()` (the static `utoipa` document) for the contract test and for the base of the live document.
- [ ] T014 [US1b] Create `crates/inkpdf/src/api/docs.rs`, serving `GET /docs` as `text/html`: the Scalar `api-reference` script from jsDelivr pinned to an exact version (look up the current version, at least two weeks old), with `data-url="/openapi.json"`, title "inkpdf API". Remove `utoipa-scalar` from `crates/inkpdf/Cargo.toml` and `build_app`. Document the `/docs` route in the OpenAPI if it was listed before; otherwise leave it out of the contract.
- [ ] T015 [US1b] Run T009–T011 and fix until they pass. Run `cargo test --workspace` and `INKPDF_UPDATE_OPENAPI=1 cargo test -p inkpdf --test contract_openapi`; the diff must be limited to the `info.version` normalisation. Commit.

**Checkpoint**: quickstart §2 passes locally with `cargo run` and the example volume.

---

## Phase 4: User Story 1 — Documentation site (P1)

**Goal**: an English mdBook site with generated references ([contracts/docs-site.md](./contracts/docs-site.md)).

**Independent Test**: `mdbook build docs` succeeds and the link check passes; a newcomer follows Getting started.

### Tests for User Story 1 ⚠️

- [ ] T016 [P] [US1] In `xtask/src/docs.rs`, add unit tests for the generators:
  - `api.md` lists every path and method of a small OpenAPI fixture ;
  - `errors.md` lists every `ErrorCode` with its HTTP status ;
  - `packages.md` lists every `selected` package with its import line, and every package in the licenses table.

### Implementation for User Story 1

- [ ] T017 [US1] Build the `xtask` CLI skeleton:
  - `xtask/Cargo.toml` with dependencies `clap` (derive), `anyhow`, `serde`, `serde_json` and `toml` ;
  - `xtask/src/main.rs` with subcommands `docs generate|check` (this phase) and `packages …` (US4).
- [ ] T018 [US1] Implement `xtask/src/docs.rs`. Generate:
  - `docs/src/reference/api.md` from `openapi/openapi.json`: one section per operation, with method and path, summary and description, parameters table, request body (component name and link), responses table, plus an intro explaining that each template's typed operation is published live at `/openapi.json` and `/docs` of a running instance ;
  - `docs/src/reference/errors.md`, from the `ErrorCode` schema enum and `crates/inkpdf/src/error.rs` statuses. Statuses are not in the OpenAPI enum: embed a small static table in xtask, checked by a test that compares it with the source (parse the `status()` match in `error.rs`) ;
  - `docs/src/reference/packages.md` from `packages/lock.toml` and the archives' `typst.toml` (description, license, WASM detection).

  `check` regenerates in memory and fails with a diff hint when a file differs. Each generated file starts with `<!-- generated by cargo xtask docs generate; do not edit -->`.
- [ ] T019 [US1] Create the book skeleton:
  - `docs/book.toml`: title, `language = "en"`, `src = "src"`, `[output.html]` with `git-repository-url` and `edit-url-template`, search on ;
  - `docs/src/SUMMARY.md` following the contract structure.

  Move the existing `docs/templates.md` content into `docs/src/reference/template-format.md` and `docs/src/concepts/*.md`, and the hand-written parts of `docs/packages.md` into `docs/src/contributing/packages.md`. Delete the old files and update the links (README, tests that read `docs/packages.md` → now `docs/src/reference/packages.md`).
- [ ] T020 [P] [US1] Write `docs/src/introduction.md` and `docs/src/getting-started/{docker.md,first-template.md}`: run the image, render `sample`, then create the smallest template (`main.typ` plus a `schema.json` with `data`), all copy-pasteable.
- [ ] T021 [P] [US1] Write `docs/src/concepts/{templates.md,request-body.md,schemas.md,packages.md,hot-reload.md,sandbox.md}`. Insist that inkpdf is a foundation and that the content of `data` and `layout` is defined by each template's own schema; examples are illustrations. `metadata` is service-defined.
- [ ] T022 [P] [US1] Write `docs/src/guides/{writing-a-template.md,best-practices.md,packages.md,downloads-and-metadata.md,exploring-the-api.md,progress-invoice.md}`.
  - Best practices cover:
    - schema design (`additionalProperties: false`, `$defs`, required fields) ;
    - `layout` defaults and naming (`#let opts = sys.inputs.layout`) ;
    - exact amounts with `decimal` and strings ;
    - dates passed in data ;
    - splitting into `parts/` ;
    - repeated table headers and page X / Y ;
    - determinism ;
    - testing with example requests and Bruno.
  - `exploring-the-api.md` explains the live `/docs` and `/openapi.json`, and code generation with standard OpenAPI generators.
  - The invoice walkthrough points to `examples/templates/progress-invoice` and the four requests.
- [ ] T023 [P] [US1] Write `docs/src/reference/configuration.md`, listing every `INKPDF_*` variable with its default; a test in `xtask` checks that every variable read in `crates/inkpdf/src/config.rs` appears in the page. Also write `docs/src/operations/{deployment.md,limits.md,security.md}` and `docs/src/contributing/{architecture.md,development.md,releasing.md}`.
- [ ] T024 [US1] Create `.github/workflows/docs.yml`:
  - triggers: PR (build only) and push to `main` (build and deploy) ;
  - steps: install pinned mdBook and lychee, `cargo xtask docs check`, `mdbook build docs`, `lychee --offline --no-progress docs/book` ;
  - on `main`: `actions/upload-pages-artifact` and `actions/deploy-pages`, with permissions `pages: write` and `id-token: write`.

  Add `docs/book/` to `.gitignore`.
- [ ] T025 [US1] Run `cargo xtask docs generate`, `mdbook build docs` and the link check locally (install mdBook and lychee via `cargo install --locked` or the release binaries). Fix broken links, then commit.

**Checkpoint**: the site builds and every internal link resolves.

---

## Phase 5: User Story 2 — Images and releases (P1)

**Goal**: `dev` on `main`, versioned images on release ([contracts/delivery.md](./contracts/delivery.md)).

**Independent Test**: verified on GitHub after the merge (quickstart §5). Locally, the workflows are validated with `actionlint`.

- [ ] T026 [US2] In `.github/workflows/ci.yml`, add a `publish-dev` job:
  - `needs: [lint, test, perf]`, `if: github.event_name == 'push' && github.ref == 'refs/heads/main'` ;
  - `permissions: packages: write` and `concurrency: { group: publish-dev, cancel-in-progress: true }` ;
  - QEMU, Buildx and a GHCR login, then build and push `linux/amd64,linux/arm64` with only the tag `ghcr.io/ck-developer/inkpdf:dev`, using the GHA cache.
- [ ] T027 [US2] Create `release-please-config.json` and `.release-please-manifest.json` (R7):
  - package `crates/inkpdf`, `release-type: rust`, `bump-minor-pre-major: true`, `include-component-in-tag: false` ;
  - `bootstrap-sha: 90ee8fb…` (full SHA of the 002 merge) ;
  - changelog sections for feat, fix, perf, docs, refactor ;
  - manifest version `0.1.0`.
- [ ] T028 [US2] Create `.github/workflows/release.yml`:
  - on push to `main`, run `googleapis/release-please-action@v4` (config and manifest files) ;
  - if `release_created`, check out the tag, then build and push `ghcr.io/ck-developer/inkpdf` with tags `X.Y.Z`, `X.Y` and `latest` (from the outputs `major`, `minor` and `patch`) for both architectures.

  Delete `.github/workflows/docker.yml`.
- [ ] T029 [P] [US2] Write the initial `CHANGELOG.md`: release-please header, plus a "0.1.0 — history before automated releases" section summarising features 001 (V1 service) and 002 (bundled packages, downloads, metadata, `layout`, examples) in English.
- [ ] T030 [US2] Validate the workflows with `actionlint` (run via `docker run --rm -v "$PWD":/repo -w /repo rhysd/actionlint:latest`). Check that `/health` and `info.version` come from `CARGO_PKG_VERSION` (FR-013). Commit.

**Checkpoint**: workflows lint-clean; behaviour verified after merge.

---

## Phase 6: User Story 3 — Open-source hygiene (P2)

**Goal**: contributor-facing files and a short README.

**Independent Test**: files present, issue and PR templates render on GitHub, README ≤ about one screen.

- [ ] T031 [P] [US3] Write `CONTRIBUTING.md`: setup (Rust toolchain, Docker), workspace layout, conventional commits, tests, docs (`cargo xtask docs generate`, `mdbook serve docs`), packages (`cargo xtask packages`), PR checklist, release flow, English-only convention.
- [ ] T032 [P] [US3] Add `CODE_OF_CONDUCT.md`: Contributor Covenant 2.1, full official text, with the contact `conduct@` replaced by "open a private report through GitHub (Security → Report a vulnerability) or contact the maintainers listed in the repository".
- [ ] T033 [P] [US3] Write `SECURITY.md`: supported versions (latest release and `dev`), private reporting through GitHub private vulnerability reporting, scope (sandbox escapes, path traversal, resource exhaustion), and a reminder of the deployment model (private network, no auth).
- [ ] T034 [P] [US3] Create `.github/ISSUE_TEMPLATE/bug.yml`, `feature.yml` and `config.yml` (blank issues disabled, links to docs and security), plus `.github/PULL_REQUEST_TEMPLATE.md` (summary, test plan, docs updated, conventional title).
- [ ] T035 [P] [US3] Create `.github/dependabot.yml`: weekly updates for `cargo` (directory `/`), `github-actions` (`/`) and `docker` (`/`), with Rust minor and patch updates grouped.
- [ ] T036 [US3] Rewrite `README.md` as an entry point:
  - badges: CI, Docs, License, Release, GHCR ;
  - a one-paragraph pitch ;
  - a 3-step quick start: `docker run` with the example volume, then `curl` render, then open `/docs` ;
  - links to the docs site, examples and Bruno ;
  - the license section from PR #3.

  The detailed sections move to the book (T019–T023).

**Checkpoint**: hygiene files present; README short.

---

## Phase 7: User Story 4 — `cargo xtask packages` (P2)

**Goal**: replace `scripts/add-package.sh` with an atomic, dependency-aware command ([contracts/xtask-cli.md](./contracts/xtask-cli.md)).

**Independent Test**: `list` and `verify` pass on the current set; `add --dry-run` shows the resolved version and dependencies; a corrupted archive makes `verify` fail.

### Tests for User Story 4 ⚠️

- [ ] T037 [P] [US4] Write unit tests in `xtask/src/packages.rs` and `xtask/src/universe.rs`, all offline, with fixture index JSON and small in-memory tarballs:
  - pick the latest compatible version, skipping versions whose `compiler` is above 0.15.1 ;
  - discover literal imports in the `.typ` files, ignoring the `tests/` and `docs/` folders ;
  - compute the transitive dependency closure ;
  - in update mode, remove orphans ;
  - `verify` detects a digest mismatch, a missing archive, an extra archive, two `selected` entries for one name, and an unresolved import ;
  - atomicity: a simulated failure leaves the lock file unchanged.

### Implementation for User Story 4

- [ ] T038 [US4] Implement `xtask/src/universe.rs`:
  - index download with `ureq` (rustls) and a 30 s timeout ;
  - archive download ;
  - engine version read from the root `Cargo.toml` (`typst = "=X.Y.Z"` in `crates/inkpdf/Cargo.toml`) ;
  - SemVer comparison.
- [ ] T039 [US4] Implement `xtask/src/packages.rs` with `add`, `update`, `remove`, `verify` and `list` (`--dry-run` for `add` and `update`). Work in a temporary directory, then move the files into place atomically, then regenerate `docs/src/reference/packages.md` (via `docs.rs`). Lock entries are written sorted, with the existing header. Dependency discovery uses `typst-syntax =0.15.1`.
- [ ] T040 [US4] Delete `scripts/add-package.sh`. Update the references in `crates/inkpdf/src/packages/mod.rs` (doc comment), `specs/002-typst-packages/contracts/lock-file.md` (note "replaced by cargo xtask packages in 003"), `.claude/skills/typst-dev/SKILL.md`, `docs/src/contributing/packages.md` and `CONTRIBUTING.md`.
- [ ] T041 [US4] Add `cargo xtask packages verify` to the CI `test` job in `.github/workflows/ci.yml`. Run `cargo xtask packages list`, `verify` and `add rowmantic --dry-run` (with network) locally. Corrupt an archive and check that `verify` fails, then restore it. Commit.

**Checkpoint**: the package command replaces the script; verify runs in CI.

---

## Phase 8: Polish

- [ ] T042 [P] Update the `.claude/skills/typst-dev` references to the new layout (`crates/inkpdf/src/render/…`, `cargo xtask packages`), and the memory notes in the user's project memory (workspace layout, docs site, live OpenAPI).
- [ ] T043 Run quickstart §1–§4 end to end:
  - `cargo fmt --all --check`, clippy, `cargo test --workspace`, perf ;
  - `docker compose up --build`: live document scenario, `/docs` in a browser (check that Scalar loads and shows the per-template operations) ;
  - the Bruno collection still passes (`npx @usebruno/cli@2 run --env local` in `examples/bruno`) ;
  - `cargo xtask docs check` and the book build.

  Fix anything stale.
- [ ] T044 Mark `specs/003-open-source-project/spec.md` as implemented once merged. Write the PR description in English, and explain that publication (`dev`, docs, release PR) is verified after the merge.

---

## Dependencies & Execution Order

- **Phase 1** → **Phase 2 (workspace)**, which blocks everything.
- **US1b (Phase 3)**: after Phase 2. It is independent of the docs.
- **US1 (Phase 4)**: after Phase 2. T017 (xtask skeleton) is needed by US4 too. T018 reads `openapi/openapi.json`; run T015 first if US1b changes the locked file (`info.version` normalisation).
- **US2 (Phase 5)**: after Phase 2. T026 needs the `ci.yml` from T007.
- **US3 (Phase 6)**: independent after Phase 2. T036 (README) after T019, so the links exist.
- **US4 (Phase 7)**: after T017 and T018, since it regenerates `packages.md` through `docs.rs`.
- **Polish**: last.

### Parallel Opportunities

- After Phase 2:
  - US1b (service code) ∥ US2 (workflows) ∥ US3 (hygiene files) ;
  - US1's writing tasks T020–T023 run in parallel with each other.
- US3: T031–T035 all in parallel.
- Tests: T009, T010 and T011 in parallel; T016 ∥ T037.

## Implementation Strategy

1. **MVP**: Phase 2 + US1b. The live per-template OpenAPI is the most visible product change and is testable immediately.
2. Then US1 (docs site) and US2 (publication), which together make the project usable by others.
3. Then US3 and US4.
4. Commit per phase, with conventional English messages. One PR for the feature.

## Notes

- Publication behaviour (US2, docs deployment) can only be fully verified on GitHub after the merge. The PR states it.
- The Scalar version pin and the mdBook and lychee versions are chosen at implementation time (latest release at least two weeks old) and recorded in the files.
