# Bundled Typst packages

inkpdf bundles a fixed list of [Typst Universe](https://typst.app/universe) packages: they are
part of the binary, and nothing is ever downloaded. The list is also available through
`GET /packages`.

## Using a package in a template

A package is imported **by name only**, without a version: the service uses the version it has
installed.

```typst
#import "@preview/zero": num
#import "@preview/tiaoma"

Amount: #num("1234.56", decimal-separator: ",")
#tiaoma.qrcode("INV-2026-0042")
```

Full rules: [templates.md](./templates.md#using-a-package).

## Available packages

| Import | What it does | Version | License | WASM |
|---|---|---|---|---|
| `@preview/cetz` | Drawing library: shapes, diagrams and charts. | 0.5.2 | LGPL-3.0-or-later | yes |
| `@preview/cetz-plot` | Plots line charts, bar charts and pie charts. | 0.1.4 | LGPL-3.0-or-later |  |
| `@preview/datify` | Formats dates in any language. | 1.3.0 | MIT |  |
| `@preview/framefit` | Resizes text so that it fits in a frame. | 0.1.0 | MIT |  |
| `@preview/frogst` | Spells out numbers in French words (amounts in words). | 1.0.0 | MIT |  |
| `@preview/ibanator` | Validates and formats IBAN numbers. | 0.1.0 | EUPL-1.2 | yes |
| `@preview/lilaq` | High-quality data plots (lines, bars, scatter plots). | 0.6.0 | MIT |  |
| `@preview/linguify` | Loads translated strings according to the document language. | 0.5.0 | MIT | yes |
| `@preview/modern-mailmerge` | Mail merge: letters, certificates, labels, badges, envelopes. | 0.1.0 | MIT |  |
| `@preview/oxifmt` | Formats strings and numbers (decimals, separators, alignment). | 1.0.0 | MIT OR Apache-2.0 |  |
| `@preview/payqr-swiss` | Swiss QR-bill payment slip. | 0.5.0 | LGPL-3.0-only |  |
| `@preview/primaviz` | Draws 50+ chart types (bars, lines, pies…) with no dependencies. | 0.11.0 | MIT |  |
| `@preview/qrypst` | Draws QR codes at an exact print size. | 0.1.1 | Unlicense AND MIT | yes |
| `@preview/sepay` | Generates the SEPA credit transfer (EPC) QR code for invoices. | 0.1.1 | MIT |  |
| `@preview/showybox` | Creates colorful, customizable boxes. | 2.0.4 | MIT |  |
| `@preview/tablem` | Writes tables simply, Markdown-style. | 0.3.0 | MIT |  |
| `@preview/tabut` | Displays data as a table. | 1.0.2 | MIT |  |
| `@preview/tiaoma` | Generates barcodes and QR codes in many formats. | 0.3.0 | MIT | yes |
| `@preview/zebra` | Generates QR codes and Data Matrix codes as native drawings. | 0.1.0 | MIT | yes |
| `@preview/zero` | Formats numbers and units precisely (separators, rounding). | 0.7.1 | MIT |  |

**WASM**: the package contains a compiled plugin. It runs inside the engine, with no network
or file access, but a very long computation cannot be interrupted before it finishes: the
response is sent as `504` at the configured timeout, and the render slot stays busy until the
computation actually ends (see README, known limitation).

## Licenses

All bundled packages, including internal dependencies (which templates cannot import). The
text of each license is kept in the binary alongside the package.

| Package | Version | Role | License |
|---|---|---|---|
| cetz | 0.5.2 | offered | LGPL-3.0-or-later |
| cetz-plot | 0.1.4 | offered | LGPL-3.0-or-later |
| datify | 1.3.0 | offered | MIT |
| datify-core | 2.1.0 | dependency | MIT |
| elembic | 1.1.1 | dependency | MIT OR Apache-2.0 |
| framefit | 0.1.0 | offered | MIT |
| frogst | 1.0.0 | offered | MIT |
| ibanator | 0.1.0 | offered | EUPL-1.2 |
| komet | 0.1.0 | dependency | MIT |
| komet | 0.2.0 | dependency | MIT |
| lilaq | 0.6.0 | offered | MIT |
| linguify | 0.5.0 | offered | MIT |
| modern-mailmerge | 0.1.0 | offered | MIT |
| oxifmt | 1.0.0 | offered | MIT OR Apache-2.0 |
| payqr-swiss | 0.5.0 | offered | LGPL-3.0-only |
| primaviz | 0.11.0 | offered | MIT |
| qrypst | 0.1.1 | offered | Unlicense AND MIT |
| rustycure | 0.2.0 | dependency | EUPL-1.2 |
| sepay | 0.1.1 | offered | MIT |
| showybox | 2.0.4 | offered | MIT |
| suiji | 0.5.1 | dependency | MIT |
| tablem | 0.3.0 | offered | MIT |
| tabut | 1.0.2 | offered | MIT |
| tiaoma | 0.3.0 | offered | MIT |
| tiptoe | 0.4.0 | dependency | MIT |
| zebra | 0.1.0 | offered | MIT |
| zero | 0.6.1 | dependency | MIT |
| zero | 0.7.1 | offered | MIT |

## Maintainers: changing the packages

The list is defined by `packages/lock.toml` (name, version, sha256 digest, license, role) and
the official archives in `packages/vendor/`. Full contract:
`specs/002-typst-packages/contracts/lock-file.md`.

- **Add a package**: `scripts/add-package.sh <name> <version>`, then add its dependencies with
  `--dependency` until `cargo test --test bundled_packages` passes (closure test). Add a
  fixture `tests/fixtures/package-smoke/<name>.typ` and a row in the table above.
- **Change a version**: rerun the script with the new version; the offered entry is replaced
  and the old archive is deleted if nothing else uses it. Every template moves to the new
  version on the next deployment: mention it in the PR.
- **Remove a package**: a breaking change for the templates that import it (they become
  invalid); explicit decision only.
- **Package incompatible with the service's Typst version**: it is removed, never patched
  locally (e.g. `codetastic` 0.2.2, removed during the initial integration).
- The build verifies the digests: a tampered or unlisted archive makes `cargo build` fail,
  naming the package.
