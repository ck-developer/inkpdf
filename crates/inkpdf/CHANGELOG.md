# Changelog

## 0.1.0 (2026-10-10)


### Features

* **api:** live, template-aware OpenAPI document and /docs page ([1e7fabf](https://github.com/ck-developer/inkpdf/commit/1e7fabf0c512de89e8b7cd848093e47fa4bec20c))
* **xtask:** cargo xtask packages add|update|remove|verify|list and docs generate|check ([2f2159f](https://github.com/ck-developer/inkpdf/commit/2f2159fb9025ac0df813202c65ad4965e41f9980))


### Bug Fixes

* **release:** keep the changelog inside the package and update Cargo.lock ([ae68d57](https://github.com/ck-developer/inkpdf/commit/ae68d5733432b1f5243814ad098d7681179ee162))
* **release:** keep the changelog inside the package and update Cargo.lock ([e939f07](https://github.com/ck-developer/inkpdf/commit/e939f07a175fae8d64d4c12cafadd6cafd78f079))


### Refactoring

* move the service into a Cargo workspace (crates/inkpdf) ([6ca4263](https://github.com/ck-developer/inkpdf/commit/6ca426334cd7ae1ee5ddad711c4cb90f9e8c4173))

## Changelog

All notable changes to inkpdf are documented here. From 0.1.0 on, this file is maintained by
[release-please](https://github.com/googleapis/release-please) from
[conventional commits](https://www.conventionalcommits.org/).

## History before the first release

Work done before 0.1.0, summarised here for reference.

### Bundled Typst packages, downloads, metadata, `layout` (feature 002)

- 20 Typst Universe packages embedded in the binary (QR codes and barcodes, charts, number,
  date and amount formatting, tables, boxes, mail merge, SEPA and Swiss QR bills), imported
  **by name only**; versions pinned with sha256 digests in `packages/lock.toml`.
- Package imports resolved once at template load; incorrect imports make the template
  invalid with `file:line: message`; errors inside a package point to its file and line.
- `GET /packages` lists the packages available to templates.
- `?download=true&filename=…` returns the PDF as an attachment with a cleaned file name.
- Optional `metadata` (title, author, subject, keywords, date) written into the PDF;
  default author configurable with `INKPDF_DEFAULT_AUTHOR`.
- **Breaking**: the `design` request dimension is renamed `layout`.
- Examples: a multi-page construction progress invoice and a Bruno collection.

### PDF generation service (feature 001)

- REST API to list templates, read their schema and render PDFs from `{ data, layout }`
  validated with JSON Schema.
- Templates hot-reloaded from a mounted volume; invalid templates reported, not fatal.
- Embedded Typst engine in a sandbox: no network, no file outside the template folder,
  bounded body size, render time and concurrency; deterministic PDFs.
- RFC 9457 errors, OpenAPI document and interactive docs, Docker image.
