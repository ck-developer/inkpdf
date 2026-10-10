# Using bundled packages

inkpdf ships a fixed selection of [Typst Universe](https://typst.app/universe) packages inside
its binary: QR codes and barcodes, charts, number, amount and date formatting, boxes, tables.
Nothing is downloaded at render time. The full list, with versions and licences, is in
[Bundled packages](../reference/packages.md); a running instance lists them at
`GET /packages`.

## Import by name only

```typst
#import "@preview/zero": num
#import "@preview/tiaoma"

Amount: #num("1234.56", digits: 2)
#tiaoma.qrcode("INV-2026-0042")
```

Write the package name **without a version**. When the template is loaded, the service
rewrites each import to the version it has installed (`@preview/zero` becomes
`@preview/zero:0.7.1`). Consequences:

- `GET /packages` (or the reference page) tells you which version your template uses;
- when the service upgrades a package, every template moves to the new version on its next
  load, with no change to the template;
- the import must be a literal string: `#import "@preview/" + name` is not supported;
- `#include "@preview/<name>"` follows the same rules;
- only `@preview` packages from the list can be imported. Packages that appear only as
  dependencies in the licence table cannot be imported directly;
- an inkpdf template does not compile as is with the standalone `typst` tool, which requires a
  version in every package import.

## How errors look

Imports are checked once, when the template is loaded, in every `.typ` file of the folder. A
wrong import makes the template `invalid`; `GET /templates/{id}` shows one line per error in
`reason`, and a render returns `409 template-invalid`:

```text
main.typ:3: remove the version: write @preview/zero (inkpdf uses its installed version)
main.typ:4: package @preview/foo is not available in inkpdf (see GET /packages)
parts/qr.typ:1: only @preview packages are available: @local/qr
```

An error raised **inside** a package at render time is a normal `500 render-failed`. Its
diagnostic names the package file, prefixed with the package and its version:

```json
{
  "message": "…",
  "file": "@preview/zero:0.7.1/src/num.typ",
  "line": 42,
  "column": 3
}
```

Look one level up in your own code: the cause is almost always the value you passed in.

## Passing images and data to a package

A package can only read its own files: `image("assets/logo.png")` written *inside* a package
looks in the package, not in your template. Load the file in your template and pass the
result in:

```typst
// An already-built image element.
#some-package.card(logo: image("assets/logo.png", width: 2cm))

// Raw bytes, or parsed data.
#some-package.render(read("assets/logo.svg", encoding: none))
#some-package.table(csv("assets/rates.csv"))

// A path value, resolved relative to your template file.
#some-package.stamp(path("assets/logo.png"))
```

The same applies to request data: read it from `sys.inputs` in your template and pass the
values as arguments. Fonts inside packages are not loaded either: use the embedded fonts or
the template's `fonts/` folder.

## Common recipes

QR code and barcode:

```typst
#import "@preview/tiaoma"
#box(width: 3cm, tiaoma.qrcode(data.reference))
#tiaoma.ean("4006381333931")
```

Formatted amount, from a decimal string:

```typst
#import "@preview/zero": num
€#num(data.amount, digits: 2, group: (size: 3, separator: ",", threshold: 4))
```

Chart:

```typst
#import "@preview/cetz"
#import "@preview/cetz-plot": chart

#cetz.canvas({
  chart.columnchart(
    data.series.map(item => (item.label, item.value)),
    size: (10, 5),
  )
})
```

Date in another language:

```typst
#import "@preview/datify": custom-date-format
#custom-date-format(datetime(year: 2026, month: 10, day: 10), pattern: "long", lang: "fr")
```

SEPA payment QR code, French amount in words and more are used in the
[progress invoice](progress-invoice.md). The `packages-demo` example template in the repository
combines a QR code, a formatted amount and a chart.

## WASM packages

Some packages, flagged **WASM** in [Bundled packages](../reference/packages.md) (for example
`tiaoma`, `qrypst`, `zebra`, `cetz`), contain a compiled WebAssembly plugin. The plugin runs
inside the engine, without network or file access. Its only caveat: a plugin call cannot be
interrupted. If one runs for longer than the render timeout, the caller still gets
`504 render-timeout` on time, but the render slot stays busy until the call ends. Keep the
input of such packages bounded (a QR code payload, not a whole document). See
[Limits and performance](../operations/limits.md).
