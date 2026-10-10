# Changelog

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
