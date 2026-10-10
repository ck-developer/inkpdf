# Bundled packages

inkpdf ships with a selection of [Typst Universe](https://typst.app/universe) packages: QR codes
and barcodes, charts, number, amount and date formatting, boxes and tables. They are part of the
binary, verified at build time, and nothing is ever downloaded. A template uses them like any
Typst package, with one difference: it imports them **by name only**.

```typst
#import "@preview/zero": num
#import "@preview/tiaoma"

Amount: #num(sys.inputs.data.amount, digits: 2, decimal-separator: ",")
#tiaoma.qrcode(sys.inputs.data.reference)
```

The list of packages, with their installed versions and licenses, is available from the running
service at `GET /packages` and in [Bundled packages (reference)](../reference/packages.md).

## Import rules

1. **By name only**: `#import "@preview/<name>"`, optionally with `: a, b` or `as x`.
   `#include "@preview/<name>"` follows the same rule.
2. **No version**: the service uses the version it has installed, shown in `GET /packages`.
   `@preview/zero:0.7.1` is rejected.
3. **Only the offered packages**: `@preview` is the only namespace, and only the packages listed
   by `GET /packages` can be imported. Packages bundled only as dependencies of others are not
   importable.
4. **Written literally**: the package must be a string literal in the `import` or `include`.
   A computed import such as `"@preview/" + name` is not supported.
5. **A package only reads its own files**. To give it an image or data from the template, load
   them in the template and pass them in: `image("assets/logo.png")`, `read("data.csv")` or
   `path("assets/logo.png")`.
6. **Package fonts are not loaded**: use the embedded fonts or those in the template's `fonts/`
   folder.
7. A `packages/` folder inside a template is not used as a package source.
8. Because the version is omitted, an inkpdf template does not compile as is with the standard
   `typst` command-line tool, which requires one.

## Resolution when the template is loaded

Imports are checked **once, when the template is loaded**, not on every render. Each correct
import is rewritten in the in-memory copy of the template with the installed version
(`@preview/zero` becomes `@preview/zero:<installed version>`), so a render only ever sees exact,
available packages.

An incorrect import makes the template invalid, with one line per error in its `reason`:

```text
main.typ:3: remove the version: write @preview/zero (inkpdf uses its installed version)
main.typ:4: package @preview/foo is not available in inkpdf (see GET /packages)
```

Rendering an invalid template returns `409 template-invalid` with the same reason.

An error raised inside a package during a render is reported in `diagnostics[]` with a file
prefixed by the package, for example `@preview/zero:0.7.1/src/num.typ`.

## Versions and upgrades

There is exactly one version of each offered package per inkpdf release. When a release changes
a package version, every template that imports it moves to the new version on the next
deployment of the service. Templates never pin a version
themselves, so they cannot drift apart.

## Isolation

Packages run inside the same sandbox as templates: no network, no files outside their own
archive, no environment variables. Some packages contain a compiled WebAssembly plugin (flagged
in the reference list): it runs inside the engine with no network or file access, but a very long
plugin call cannot be interrupted before it finishes; see
[Limits and performance](../operations/limits.md).

## Learn more

- [Using bundled packages](../guides/packages.md): QR codes, charts, amounts and dates in practice.
- [Adding or updating a package](../contributing/packages.md): for maintainers.

The repository's `examples/templates/packages-demo` template uses a QR code, a formatted amount
and a chart.
