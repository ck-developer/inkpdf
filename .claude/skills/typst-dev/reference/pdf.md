# typst-pdf 0.15.1

## PdfOptions
| Field | Default | Notes |
|---|---|---|
| `ident: Smart<String>` | Auto | hashed into `/ID` + XMP DocumentID; inkpdf uses `"{id}@{fingerprint}"` |
| `creator: Smart<Option<String>>` | "Typst 0.15.1" | changes on every Typst upgrade |
| `timestamp: Option<Timestamp>` | None | keep None for determinism (no date written) |
| `page_ranges` | None | export subset |
| `standards: PdfStandards` | PDF 1.7 | `PdfStandards::new(&[..])` validates combination |
| `tagged: bool` | true | tagged PDF by default since 0.14 |
| `pretty: bool` | false | keep false |

`PdfStandard`: V_1_4…V_2_0; A_1b, A_1a, A_2b, A_2u, A_2a, A_3b, A_3u, A_3a, A_4, A_4f, A_4e;
Ua_1 (no PDF/UA-2). One version + one PDF/A + one PDF/UA, combinable when ranges overlap.

## Attachments
`pdf.attach(path, [data], relationship: "source"|"data"|"alternative"|"supplement",
mime-type:, description:)` — relationship only honoured in PDF/A-3; not allowed in PDF/A-2;
`/AF` written for PDF/A-3 or PDF 2.0. `pdf.embed` was removed in 0.15.

## Metadata
`set document(title:, author:, description:, keywords:, date:)`; language from `text.lang`;
`title` required for PDF/UA. **No custom XMP** possible.

## Determinism
`/ID = (hash(version, ident), hash(serialized bytes))`; no dates when `timestamp: None` and no
`document(date:)`. Remaining sources: `datetime.today()` (inkpdf's `SandboxWorld::today`
returns the real date), Typst upgrades, fonts, impure plugins.

## Gaps
No PDF/UA-2, no attachments in PDF/A-2, no signatures, no AcroForm, no encryption, no custom
XMP. Factur-X/ZUGFeRD = PDF/A-3 + `pdf.attach("factur-x.xml", …)` **plus** the `fx:` XMP
extension, which Typst cannot write → needs a deterministic post-processing step.

## Other outputs
PNG: `typst_render::render(&Page, &RenderOptions)` (signature changed in 0.15). SVG:
`typst_svg::svg(&Page, &SvgOptions)`. HTML: experimental, `Feature::Html`, no CSS — unsuitable
for faithful previews. A preview can reuse the same `PagedDocument` as the PDF.
