# Typst language for parameterized templates (0.15.1)

## Data
- `sys.inputs` dict; safe access `.at("k", default: none)`.
- JSON floats lose precision → `decimal("12.30")` or integer cents; `decimal` ops are exact.
- `json(path|bytes)`, `read(path, encoding: none)` → bytes; `*.decode` removed in 0.15.

## Structure
- `set` for properties, `show sel: it => …` to transform, `#show: tpl.with(..)` for a
  template function. Named args with defaults, `..args`, `f.with(..)`.
- Modules: `#import "parts/x.typ": f`, `as x`, `#include`; relative to the caller, `/…` to the
  root; dynamic imports need `as`.

## Context & introspection
- `context { … }` required for `counter(..).get()/display()`, `here()`, `query`, `measure`,
  `text.lang`. `state("k", init)` with `.update/.get/.final`. 0.15: `counter.display(at:)`,
  `within` selector.
- Layout converges in ≤5 iterations (`MAX_ITERS`). Non-convergence causes: state updated from
  its own `final`, size depending on page count, nested measure/layout loops. Every iteration
  re-lays out the whole document (performance cost).

## Page, tables, images, fonts
- `set page(paper:, margin:, header:, footer:, numbering:)`; 0.15 adds `page.bleed`.
- `table(columns: (auto, 1fr), table.header(repeat: true, ..), ..rows, table.footer(..))`;
  multiple headers since 0.14; `breakable` on `table.cell` / `block`.
- `image(source, fit:, alt:)`: PNG, JPEG, GIF, WebP, SVG, PDF, raw pixels.
- `text(font: ("A", "B"), fallback: true)` searches only the World's FontBook (embedded +
  template `fonts/`); fonts shipped inside packages are never loaded. Variable fonts in 0.15.

## Numbers & dates
- No thousands separator / locale formatting natively: write a helper on `str`/`decimal` or
  use `zero`/`oxifmt`. `calc.round(x, digits: 2)`.
- `datetime(..).display("[day]/[month]/[year]")`, English month names only (use a table or
  `datify`). Never `datetime.today()`.

## WASM plugins
`plugin(path|bytes)` runs on wasmi 1.0 without fuel: a long call cannot be interrupted by the
inkpdf cancel flag and keeps its render slot until it ends.

## Common errors
`.at()` on missing key; `str + int` without `str()`; missing `context` ("can only be used when
context is known"); type/string comparison removed in 0.14; `path(...)` as a shape (now
`curve`); backslashes in paths (0.15).

## 0.13 → 0.15 highlights
0.13: `image(source:)`, bytes accepted by loaders, static import names, `counter.display`
needs context. 0.14: tagged PDF default, PDF/UA-1 + all PDF/A, `pdf.attach`, PDF/WebP images,
multiple table headers. 0.15: `path` type across packages, `curve`/`tiling`, `pdf.embed` and
`*.decode` removed, backslash ban, typst-kit rework, `FileId`/`VirtualRoot` API,
`RenderOptions`, Rust ≥ 1.92.
