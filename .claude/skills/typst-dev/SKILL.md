---
name: "typst-dev"
description: "Typst 0.15.1 expert for inkpdf: writing or reviewing Typst templates (main.typ, schema-driven data/layout), Typst packages (typst.toml, @ns/name:version, embedded packages), typst-pdf output (PDF/A, PDF/UA, attachments, determinism) and the Rust embedding (World, FileId/VirtualRoot, SandboxWorld). Use whenever a task touches .typ files, template folders, Typst packages, PDF options or src/render/."
metadata:
  author: "inkpdf"
  typst-version: "0.15.1"
---

# Typst developer (inkpdf)

Target: **Typst 0.15.1** (`typst`, `typst-pdf`, `typst-layout`, `typst-kit` pinned `=0.15.1`).
Source of truth when in doubt: the crate sources in
`~/.cargo/registry/src/index.crates.io-*/typst-*-0.15.1/` — read them rather than guessing;
Typst changed a lot between 0.12 and 0.15 and older blog posts/answers are often wrong.

## Non-negotiable inkpdf constraints

1. Caller data reaches Typst only through `sys.inputs` (`data`, `layout`) — never build Typst
   source by string concatenation, never `eval` caller strings.
2. Sandbox: no network, no system fonts, no files outside the template folder (and, per
   package, outside the package root). Everything a template needs lives in its folder.
3. Deterministic PDF: same template + same input ⇒ identical bytes. Never use
   `datetime.today()` in templates (pass dates in `data`); keep `PdfOptions.timestamp = None`
   and a stable `ident`.
4. Every render is bounded (timeout + concurrency). Avoid layouts that fail to converge and be
   wary of WASM plugins (no fuel limit in wasmi: a long plugin call cannot be cancelled).
5. `layout` parameters drive appearance; their defaults live in `schema.json`, so templates
   can rely on every declared `layout` key being present.

## Writing templates — rules of thumb

- Read inputs defensively: `let data = sys.inputs.data`, `data.at("key", default: none)`;
  `.key` access fails on a missing key.
- Money: never `float`. Use integer cents or `decimal("12.30")`; Typst has **no** thousands
  separator / localized number formatting — use a helper or the `zero` package.
- Dates: `datetime(year:, month:, day:)`; `.display()` month names are English only.
- Positional state needs `context` (`counter(page)`, `query`, `measure`, `here()`).
  Page X/Y: `context [#counter(page).display() / #counter(page).final().first()]` in `page(footer:)`.
- Tables: native `table` (not `tablex`), `table.header(repeat: true)`, `table.footer`,
  `breakable` on cells/blocks. Prefer `table` over `grid` for accessibility.
- Template function pattern: `#let doc(title: none, body) = { set …; body }` then
  `#show: doc.with(title: …)`.
- Images: `image(source)` takes a path, `path` value or `bytes`; PNG/JPEG/GIF/WebP/SVG/PDF.
- 0.15 breaking: backslashes forbidden in paths, shape `path` → `curve`, `pattern` → `tiling`,
  `pdf.embed` and `*.decode` removed (use `json(bytes(..))`), `path` is now a *type*.

## Packages — rules of thumb

- **In inkpdf templates, import by name only**: `#import "@preview/zero": num`. Never write a
  version (`@preview/zero:0.7.1` makes the template invalid); the service substitutes its
  installed version once, at template load. Only `@preview` packages listed by
  `GET /packages` / `docs/packages.md` exist; imports must be literal strings.
- The bundled set is fixed in `packages/lock.toml` + `packages/vendor/*.tar.gz`, embedded by
  `crates/inkpdf/build.rs` (sha256-checked). Manage them with `cargo xtask packages
  add|update|remove|verify|list` (dependencies are added automatically, `--dry-run` previews);
  `cargo test -p inkpdf --test bundled_packages` checks import, usage, closure, docs.
  A package that fails on the engine version is removed, never patched.
- Plain Typst (outside inkpdf) needs a full exact version: `@preview/cetz:0.5.2` (partial
  versions fail at eval time). No lockfile; transitive deps are just imports.
- Layout of a package store: `<root>/<namespace>/<name>/<version>/typst.toml` + entrypoint.
- Inside a package, `/x` means the package root; `..` cannot escape it. To let a package use
  a template file, the template passes `image(..)`, `read(..)` bytes or a `path("..")` value.
- Validate manifests with `typst::syntax::package::PackageManifest` + `validate(&spec)`
  (checks name, version, `compiler` minimum). `compiler` is a minimum only: an old package
  may still break on 0.15 — only a test render proves compatibility.

## References (read on demand)

- `reference/packages.md` — manifest fields, resolution, World/FileId mechanics, isolation, useful Universe packages.
- `reference/pdf.md` — `PdfOptions`, standards (PDF/A, PDF/UA), attachments, metadata, determinism, Factur-X gap, PNG/SVG previews.
- `reference/language.md` — language patterns, convergence, fonts, numbers/dates, common errors, 0.13→0.15 changes.
- Full sourced research: `specs/002-typst-packages/research/typst.md`.
