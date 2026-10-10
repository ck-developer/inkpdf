# Typst packages (0.15.1)

## typst.toml (`typst::syntax::package::PackageManifest`)
`[package]`: `name` (req), `version` (req, exactly MAJ.MIN.PATCH), `entrypoint` (req, relative
to manifest), `authors`, `license` (SPDX), `description`, `homepage`, `repository`, `keywords`,
`categories` (≤3), `disciplines`, `compiler` (partial bound allowed, e.g. "0.15" — a *minimum*),
`exclude` (applied at publication only). Unknown fields are tolerated (`unknown_fields`).
`[template]`: `path`, `entrypoint`, `thumbnail` — used by `typst init` only, ignored on import.
`[tool.<name>]`: tables reserved for third-party tools; a non-table under `[tool]` fails parsing.
`validate(&spec)` checks name == spec, version == spec, `compiler <= current`.
inkpdf needs the `toml` crate (0.8, already in the lockfile via typst-syntax) to parse it.

## Namespaces & resolution
- `@preview` is the only downloadable namespace (Typst Universe). `@local` is a convention;
  any identifier namespace works.
- CLI order: data dir → cache dir → download (preview only) → `PackageError::NotFound`.
  inkpdf never downloads: `typst-kit` is built without package features (its `FsPackages`
  are disk-bound anyway). Resolution is a `match` on `VirtualRoot::Package(spec)` in the World.
- Versions are exact; `"@preview/x:0.3"` fails while parsing, before reaching the World.
- No dependency declaration / lockfile; two versions of one package coexist (two roots).
  Example: fletcher 0.5.8 pulls cetz 0.3.4 while a template may use cetz 0.5.2.

## What the World receives
`FileId` = interned `RootedPath { root: VirtualRoot::{Project, Package(PackageSpec)}, vpath }`.
`VirtualPath` is absolute, normalized; `..` past root → `PathError::Escapes`; backslash → error.
Import sequence: `World::file(Package(spec), "/typst.toml")` → parse + `validate` → resolve
entrypoint in the same root → `World::source(Package(spec), "/<entrypoint>")`.
`#include "@ns/x:v"` goes through the same path. Missing package error:
`FileError::Package(PackageError::NotFound(spec))` → "package not found (searched for …)".
`World::relative_path`-style helpers must map package roots to
`packages/<ns>/<name>/<ver><vpath>` or diagnostics lose file/line.

## Isolation and the `path` type (new in 0.15)
- In a package, `/x` is the package root; it cannot build a path to the project or another
  package; its only way out is `#import "@…"`.
- `path("logo.png")` resolves at the call site: a `path` built in `main.typ` stays in the
  `Project` root even when consumed by a package. The World cannot tell who reads it.
  Recommended contract: the template passes images/bytes/`path` values to packages explicitly.
- Values (`bytes`, `content`, dicts, functions) cross roots freely.

## Static import detection
Parse each `.typ` with `typst::syntax::parse`, walk `children()`, cast to
`ast::ModuleImport` / `ast::ModuleInclude`, take `.source()`; if `Expr::Str` starting with `@`,
parse as `PackageSpec`. Literal imports only (computed imports invisible; dead code gives false
positives) → complement, never replace, the render-time check.

## Useful Universe packages (versions as of 2026-10; 0.15 compatibility unverified)
| Need | Package | Notes |
|---|---|---|
| Drawing/charts | cetz 0.5.2, cetz-plot 0.1.4 | cetz 0.5 has a WASM core (344 KB); LGPL-3.0+ |
| Plots | lilaq 0.6.0 | many deps (elembic, komet×2, suiji, tiptoe, zero) |
| Diagrams | fletcher 0.5.8 | pins cetz 0.3.4 (old; may hit 0.15 breaks) |
| Barcodes/QR | tiaoma 0.3.0 (Zint, WASM 879 KB), rustycure, qrypst, zebra | |
| Numbers | zero 0.7.1, oxifmt 1.0.0 | no dedicated currency package exists |
| Dates i18n | datify 1.3.0 (+2.7 MB CLDR), icu-datetime (4 MB WASM) | |
| i18n | linguify 0.5.0 (Fluent, WASM) | |
Obsolete: tablex (native `table` since 0.11). `zebraw` is code blocks, not barcodes.
