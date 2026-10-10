# Implementation Plan: Open-source project foundation

**Branch**: `003-open-source-project` | **Date**: 2026-10-10 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/003-open-source-project/spec.md`

## Summary

This feature turns the single-crate proof of concept into an open-source project.

- **Cargo workspace.** The service moves unchanged to `crates/inkpdf`. A new `xtask` crate
  holds the maintainer commands.
- **Live OpenAPI document.** The service builds its OpenAPI document at request time from the
  loaded templates: one typed render operation per valid template. `/docs` loads that live
  document instead of a copy embedded at startup.
- **Documentation site.** An English mdBook site lives in `docs/` and is published to GitHub
  Pages. Its API, error-code and bundled-package references are generated from
  `openapi/openapi.json` and `packages/lock.toml`.
- **Open-source files.** Contribution guide, code of conduct, security policy, changelog,
  issue and PR templates, `.editorconfig`, Dependabot and README badges.
- **CI/CD.**
  - Each push to `main` publishes the `ghcr.io/ck-developer/inkpdf:dev` image once checks
    pass.
  - release-please opens release PRs. Merging one tags `vX.Y.Z` and publishes the `X.Y.Z`,
    `X.Y` and `latest` images.
- **Package command.** `scripts/add-package.sh` is replaced by
  `cargo xtask packages add|update|verify|list`. It resolves the latest compatible version
  from the Universe index and adds Typst dependencies automatically. It writes the lock file,
  the archives and the generated package reference in a single atomic step.

Details: [research.md](./research.md).

## Technical Context

**Language/Version**: Rust 1.98, edition 2024, unchanged. Workspace resolver 3.

**Primary Dependencies**:
- Service: unchanged, with no new runtime dependency. The live OpenAPI document is built with
  `serde_json`. `utoipa-scalar` is removed in favour of a pinned Scalar page that loads
  `/openapi.json`.
- `xtask` only, never shipped in the image: `clap` (CLI), `ureq` (blocking HTTP with
  rustls, for the Universe index and archives), `flate2`, `tar`, `sha2`, `toml`, `serde`,
  `serde_json`, `typst-syntax =0.15.1` (literal-import discovery) and `anyhow`.
- Docs: mdBook (pinned version, installed in CI), plus a link check.
- CI: `googleapis/release-please-action@v4`, `docker/*` actions, `actions/deploy-pages`.

**Storage**: none. The live document is cached in memory, keyed by the identity of the
registry snapshot.

**Testing**:
- `cargo test --workspace`.
- New tests: `tests/api_live_openapi.rs` (service) and unit tests in `xtask`.
- `cargo xtask packages verify` and `cargo xtask docs check` run in CI.
- `mdbook build` runs with the link check.

**Target Platform**: unchanged (Linux amd64/arm64 image) and GitHub Pages.

**Project Type**: multi-component repository (Cargo workspace) with one service crate.

**Performance Goals**:
- Rendering is unaffected (SC-006).
- The live OpenAPI document is rebuilt only when the template set changes. Otherwise it is
  served from cache.

**Constraints**:
- The locked contract (`openapi/openapi.json`) is the document produced with **no template
  loaded**, and must stay byte-identical across the workspace move.
- The image and the API behave exactly as before.

**Scale/Scope**:
- About 25 documentation pages.
- 4 CI workflows: `ci`, `release`, `docs`, plus Dependabot.
- One new crate, `xtask`.

No open unknowns: decisions R1–R9 are in [research.md](./research.md).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| # | Principle | Status | How the design complies |
|---|-----------|--------|-------------------------|
| I | Embedded Typst, no browser | ✅ | Unchanged. The Scalar page runs in the *reader's* browser, not in the service. |
| II | Self-contained template | ✅ | Unchanged. `xtask` is a maintainer tool and its network access happens on the maintainer's machine only. |
| III | Two-dimension input, validated | ✅ | Unchanged. The live document exposes each template's `data` and `layout` schema and the fixed `metadata` schema. |
| IV | Security by construction | ✅ | The service gains no network access. The live document only republishes schemas that are already exposed (`GET /templates/{id}/schema`). Template names and descriptions are serialised as JSON strings, never as HTML. |
| V | Hot templates, frozen binary | ✅ | The live document follows hot reload, through the snapshot-identity cache. |
| VI | Self-describing REST API | ✅ (strengthened) | The OpenAPI document becomes template-aware, and the locked contract still covers the generic part (FR-020). |
| VII | Simplicity and scope | ✅ | No new runtime dependency, one dependency removed. The workspace keeps a single service crate. Release automation reduces manual work. |
| Workflow | fmt, clippy, tests; OpenAPI and docs in the same change | ✅ | CI also runs `xtask packages verify`, `xtask docs check` and the book build. Generated references fail CI when stale. |

**Re-check after Phase 1**: ✅. The contracts ([live-openapi.md](./contracts/live-openapi.md),
[xtask-cli.md](./contracts/xtask-cli.md), [delivery.md](./contracts/delivery.md) and
[docs-site.md](./contracts/docs-site.md)) add no network access to the service and no state.

## Project Structure

### Documentation (this feature)

```text
specs/003-open-source-project/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/  live-openapi.md · xtask-cli.md · delivery.md · docs-site.md
├── checklists/requirements.md
└── tasks.md            (/speckit-tasks)
```

### Repository (after this feature)

```text
Cargo.toml                  # [workspace] members = ["crates/inkpdf", "xtask"], shared lints/profiles
Cargo.lock  rust-toolchain.toml  rustfmt.toml  clippy.toml  .editorconfig
.cargo/config.toml          # alias: xtask = "run -p xtask --"
crates/
└── inkpdf/                 # the service, moved as-is
    ├── Cargo.toml  build.rs
    ├── src/ (+ src/api/live.rs: live OpenAPI document; src/api/docs.rs: /docs page)
    ├── tests/  benches/
xtask/                      # maintainer commands (packages, docs)
├── Cargo.toml
└── src/ main.rs · packages.rs · docs.rs · universe.rs
packages/                   # bundled Typst packages (lock.toml + vendor/), unchanged
docs/                       # mdBook site
├── book.toml
└── src/ SUMMARY.md, introduction.md, getting-started/, concepts/, guides/, reference/,
         operations/, contributing/
examples/                   # templates, requests, bruno (unchanged)
openapi/openapi.json        # locked generic contract
specs/  .specify/
.github/
├── workflows/ ci.yml · release.yml · docs.yml
├── dependabot.yml  ISSUE_TEMPLATE/ (bug.yml, feature.yml, config.yml)  PULL_REQUEST_TEMPLATE.md
release-please-config.json  .release-please-manifest.json
README.md  CONTRIBUTING.md  CODE_OF_CONDUCT.md  SECURITY.md  CHANGELOG.md  LICENSE-*
```

Future components get documented slots without creating empty crates:
- `typst/` for in-house Typst helper packages;
- `tools/codegen` for the client generator.

**Structure Decision**: a Cargo workspace with the service as `crates/inkpdf` and a
non-published `xtask` crate. `packages/`, `examples/`, `openapi/` and `docs/` stay at the root
because they are shared by several components and by CI. Service tests resolve the workspace
root from `CARGO_MANIFEST_DIR/../..`.

## Complexity Tracking

No constitution violation. Two additions are justified:

| Addition | Why Needed | Simpler Alternative Rejected Because |
|----------|------------|--------------------------------------|
| `xtask` crate with HTTP client | Resolves versions and dependencies from the Universe index and downloads archives atomically. | Bash: no reliable TOML editing, no Typst parser for dependency discovery, not portable. A third-party tool (utpm, gotpm): no digest-pinned vendoring. |
| Hand-written `/docs` page instead of `utoipa-scalar` | `utoipa-scalar` embeds the document once at startup, which defeats the live document (FR-018). | Rebuilding the router on each template change would be heavier and racy. |
