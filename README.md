# inkpdf

[![CI](https://github.com/ck-developer/inkpdf/actions/workflows/ci.yml/badge.svg)](https://github.com/ck-developer/inkpdf/actions/workflows/ci.yml)
[![Docs](https://github.com/ck-developer/inkpdf/actions/workflows/docs.yml/badge.svg)](https://ck-developer.github.io/inkpdf/)
[![Release](https://img.shields.io/github/v/release/ck-developer/inkpdf?include_prereleases&sort=semver)](https://github.com/ck-developer/inkpdf/releases)
[![Image](https://img.shields.io/badge/image-ghcr.io%2Fck--developer%2Finkpdf-blue)](https://github.com/ck-developer/inkpdf/pkgs/container/inkpdf)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

**inkpdf turns JSON into PDFs with [Typst](https://typst.app) templates — no browser, one small
binary.** You drop template folders into a volume; callers send `{ "data": …, "layout": … }`,
validated against each template's JSON Schema, and get a PDF back in milliseconds. inkpdf is a
foundation: it knows nothing about invoices or reports — your templates define the documents.

- Typst engine embedded in a Rust binary, in a sandbox (no network, no file outside the template).
- Templates hot-reloaded from a volume; each template's contract is published live in the
  OpenAPI document (`/openapi.json`, `/docs`).
- 20 bundled Typst packages (QR codes, barcodes, charts, number and date formatting…).
- Deterministic PDFs, PDF metadata, downloads, RFC 9457 errors.

## Quick start

```bash
git clone https://github.com/ck-developer/inkpdf && cd inkpdf
docker run --rm -p 3000:3000 -v "$PWD/examples/templates:/templates:ro" ghcr.io/ck-developer/inkpdf:dev

curl -s -H 'content-type: application/json' -d @examples/requests/sample.json \
  -o sample.pdf localhost:3000/templates/sample/render
```

Then open <http://localhost:3000/docs> to explore the API, one typed operation per template.

## Learn more

- **Documentation**: <https://ck-developer.github.io/inkpdf/> — getting started, concepts,
  writing templates, best practices, reference, operations.
- **Examples**: [`examples/templates`](examples/templates) (from a minimal sample to a
  multi-page construction progress invoice) and a [Bruno](https://www.usebruno.com) collection
  in [`examples/bruno`](examples/bruno).
- **Contributing**: [CONTRIBUTING.md](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md)
  · [Security policy](SECURITY.md) · [Changelog](CHANGELOG.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
inkpdf, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

The Typst packages bundled in the binary keep their own licenses (MIT, Apache-2.0, LGPL-3.0,
EUPL-1.2…); each one ships with its license text. See
[Bundled packages](https://ck-developer.github.io/inkpdf/reference/packages.html#licenses).
