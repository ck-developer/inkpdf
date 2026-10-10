# Contributing to inkpdf

Thanks for your interest! This guide covers the practical side; the
[documentation site](https://ck-developer.github.io/inkpdf/contributing/architecture.html)
explains the architecture in depth.

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md). Security
issues are reported privately: see [SECURITY.md](SECURITY.md).

## Ground rules

- **English everywhere** in the repository: code, comments, documentation, commit messages.
- **inkpdf is content-agnostic.** The service must not know about any document type; concrete
  documents live in `examples/` and test fixtures.
- Changes to the generic API or the template format update the OpenAPI contract and the
  documentation in the same pull request.
- Larger features follow the Spec Kit flow in `specs/` (specification → plan → tasks).

## Setup

- Rust: the toolchain is pinned in `rust-toolchain.toml` (rustup installs it automatically).
- Docker, for the image and `docker compose up --build`.
- Optional: [mdBook](https://rust-lang.github.io/mdBook/) to preview the docs, and
  [Bruno](https://www.usebruno.com) to explore the API with `examples/bruno`.

## Repository layout

| Path | Content |
|---|---|
| `crates/inkpdf` | the service (library + binary), its tests and benchmark |
| `xtask` | maintainer commands: `cargo xtask docs …`, `cargo xtask packages …` |
| `packages` | bundled Typst packages: `lock.toml` and the vendored archives |
| `docs` | documentation site (mdBook) |
| `examples` | example templates, requests and the Bruno collection |
| `openapi/openapi.json` | locked OpenAPI contract of the generic routes |
| `specs`, `.specify` | Spec Kit specifications, plans and tasks |

## Everyday commands

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --release -p inkpdf --test perf -- --ignored      # latency guard

INKPDF_UPDATE_OPENAPI=1 cargo test -p inkpdf --test contract_openapi   # regenerate the contract
cargo xtask docs generate        # regenerate the reference pages of the docs
mdbook serve docs --port 3001    # preview the docs
cargo xtask packages verify      # check the bundled packages
```

## Commits and pull requests

- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/):
  `feat(api): …`, `fix(render): …`, `docs: …`, `refactor!: …` for breaking changes. They drive
  the changelog and the version numbers.
- One topic per pull request; fill in the template's test plan.
- CI must be green: format, clippy, tests, performance guard, docs check, packages check.

## Bundled Typst packages

Use `cargo xtask packages add|update|remove|verify|list`; never edit `packages/lock.toml` by
hand. A package that does not work with the engine version is removed, not patched. Details:
[Adding or updating a package](https://ck-developer.github.io/inkpdf/contributing/packages.html).

## Releases

Maintainers merge the release pull request opened by release-please; this tags the version,
publishes the GitHub release and the `X.Y.Z`, `X.Y` and `latest` images. Every green push to
`main` already publishes the `dev` image.
