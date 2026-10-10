# inkpdf

A service that generates PDFs from [Typst](https://typst.app) templates, with no browser.

inkpdf is **generic and content-agnostic**: it knows no document type. A template is a folder
(`main.typ` + `schema.json` + resources) dropped into a volume; the caller sends a JSON
`{ "data": …, "layout": … }`, which is validated against the template's schema, then injected
into Typst as data (`sys.inputs`) and compiled to PDF by the Typst engine embedded in the
binary.

- A single Rust binary, no subprocess and no browser.
- Templates discovered **live** (added, changed or removed without a restart).
- Sandbox: no network, no reads outside the template folder.
- Bundled Typst packages (QR codes, barcodes, charts, numbers, dates…): imported by name only
  (`#import "@preview/zero"`), never downloaded ([docs/packages.md](docs/packages.md)).
- Self-describing REST API: OpenAPI 3.1 served at `/openapi.json`, documentation at `/docs`.
- Deterministic PDFs: same template and body ⇒ same bytes.

## Getting started

With Docker:

```sh
docker run --rm -p 3000:3000 \
  -v "$PWD/examples/templates:/templates:ro" \
  ghcr.io/ck-developer/inkpdf

curl -s -H 'Content-Type: application/json' \
  -d @examples/requests/sample.json \
  -o sample.pdf localhost:3000/templates/sample/render
```

With Docker Compose (image built from source, ideal for testing):

```sh
docker compose up --build                       # http://localhost:3000/docs
INKPDF_TEMPLATES=./my-templates docker compose up --build   # another templates folder
INKPDF_PORT=8080 docker compose up --build      # another port
```

Templates in the mounted folder are reloaded live: copy a template folder into it and it
shows up in `GET /templates` within a few seconds.

Locally (Rust 1.98+):

```sh
INKPDF_TEMPLATES_DIR=examples/templates cargo run --release
```

## Configuration

| Variable | Default | Purpose |
|----------|---------|---------|
| `INKPDF_TEMPLATES_DIR` | `/templates` | templates volume |
| `INKPDF_LISTEN` | `0.0.0.0:3000` | listen address |
| `INKPDF_MAX_BODY_BYTES` | `5242880` | maximum request body size |
| `INKPDF_RENDER_TIMEOUT_SECS` | `30` | maximum duration of a render (`504` beyond) |
| `INKPDF_MAX_CONCURRENT_RENDERS` | number of CPUs | concurrent renders |
| `INKPDF_QUEUE_TIMEOUT_SECS` | `10` | maximum wait for a render slot (`503` beyond) |
| `INKPDF_RESCAN_INTERVAL_SECS` | `2` | fallback rescan of the volume |
| `INKPDF_MAX_TEMPLATE_BYTES` | `52428800` | maximum size of a template (loaded in memory) |
| `INKPDF_LOG_FORMAT` | `json` | `json` or `pretty` |
| `INKPDF_DEFAULT_AUTHOR` | `inkpdf` | PDF author when neither the request (`metadata.author`) nor the template provides one |
| `RUST_LOG` | `info` | log level |

A malformed value prevents startup, with an explicit message.

## API

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/templates` | list of templates (`valid` / `invalid` with `reason`) |
| `GET` | `/templates/{templateId}` | template details, including the schema |
| `GET` | `/templates/{templateId}/schema` | raw `schema.json` (`application/schema+json`) |
| `POST` | `/templates/{templateId}/render` | render: body `{ data, layout, metadata? }` → `application/pdf`; `?download=true&filename=…` for a download |
| `GET` | `/packages` | Typst packages available to templates |
| `GET` | `/health` | liveness |
| `GET` | `/ready` | readiness (initial scan complete) |
| `GET` | `/openapi.json` | OpenAPI 3.1 description |
| `GET` | `/docs` | interactive documentation |

Errors follow RFC 9457 (`application/problem+json`) with a `code` field:
`template-not-found` (404), `invalid-json` (400), `unsupported-media-type` (415),
`invalid-parameter` (400), `validation-failed` (422, with `violations[]`), `payload-too-large` (413),
`template-invalid` (409), `render-failed` (500, with `diagnostics[]`), `render-timeout` (504),
`overloaded` (503).

The service handles neither authentication nor users: it is meant for a private network.

## Writing a template

See [docs/templates.md](docs/templates.md) and the neutral example
[`examples/templates/sample`](examples/templates/sample); using the bundled packages:
[`examples/templates/packages-demo`](examples/templates/packages-demo).

## Examples

| Template | What it shows |
|----------|---------------|
| [`sample`](examples/templates/sample) | The template format: `data`, `layout` with defaults, an included part. |
| [`packages-demo`](examples/templates/packages-demo) | Bundled packages: QR code, formatted amount, chart. |
| [`progress-invoice`](examples/templates/progress-invoice) | A demanding, multi-page French construction progress invoice (*facture de situation*): repeated page header with logo and company details, footer with legal line and page X / Y, items grouped by lot with repeated table headers, amendments, price revision, shared site costs, advance repayment, retention or bank guarantee, VAT reverse charge, several VAT rates, amount in words, SEPA payment QR code, and a rich `layout` (colours, font, density, logo position, optional columns and blocks). |

Request bodies live in [`examples/requests`](examples/requests); the progress invoice has four of
them, each exercising a different set of conditions and layout options. The legal wording and
calculation rules of the invoice are indicative: it is a demo, not an accounting template.

### Bruno collection

[`examples/bruno`](examples/bruno) is a [Bruno](https://www.usebruno.com) collection covering every
route: open the folder in Bruno, start the service with `docker compose up --build`, pick the
`local` environment and run the requests (inline and downloaded renders, metadata, validation
error, every invoice variant). A test keeps the collection in sync with the API and with
`examples/requests`.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --release --test perf -- --ignored   # latency guard (p95 < 200 ms)
cargo bench --bench render
INKPDF_UPDATE_OPENAPI=1 cargo test --test contract_openapi   # regenerates openapi/openapi.json
scripts/add-package.sh <name> <version>   # adds or changes a bundled Typst package
```

The OpenAPI document is generated from the code; `tests/contract_openapi.rs` checks that it is
identical to the versioned file `openapi/openapi.json`.

### Known limitation

Typst offers no cancellation: a compilation that exceeds its timeout is interrupted at the
template's next access to a file or a font. A purely computational loop keeps occupying a
render slot until it ends (reported by the `render.overrun` log); the caller still receives
`504` within the timeout, and the other renders remain bounded by
`INKPDF_MAX_CONCURRENT_RENDERS`. The same applies to the WASM plugins of some bundled packages
(flagged in [docs/packages.md](docs/packages.md)): a plugin call cannot be interrupted before
it finishes.

In a debug build (`cargo test`), abandoning a cancelled compilation shows up as a
`comemo: found differing return values` message on the render thread: `comemo`'s purity
assertion only exists in debug; the slot is released normally. In release, the compilation
simply ends with an error.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
inkpdf, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

The Typst packages bundled in the binary keep their own licenses (MIT, Apache-2.0, LGPL-3.0,
EUPL-1.2…); each one ships with its license text. See [docs/packages.md](docs/packages.md#licenses).
