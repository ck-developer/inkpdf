# Walkthrough: a progress invoice

The repository's `examples/templates/progress-invoice` template is a demanding, multi-page
document: a French construction progress invoice (*facture de situation*), which bills the
progress of works since the previous claim. This page walks through it as an **illustration of
the [best practices](best-practices.md)**, not as a product: its legal wording and calculation
rules are indicative only, and it is not an accounting template. The printed text is French,
so French appears below only where the invoice prints it.

## What it produces

- a page header repeated on every page, with a logo and the company's details, as a coloured
  band or a simple rule;
- a footer with the company's legal line and "page X / Y";
- items grouped by lot (trade), each with contract amount, cumulative progress, previously
  invoiced and current-period amounts, a subtotal per lot, and a table header repeated when
  the table runs onto the next page;
- contract amendments (additional works);
- a summary with price revision, shared site costs, advance repayment, VAT per rate (or VAT
  reverse charge), retention, and the amount due;
- the amount due in French words, payment terms, and a SEPA payment QR code.

## Structure

```text
progress-invoice/
├── template.json        # name, description (with the "indicative" warning), version
├── schema.json          # data + layout, with $defs
├── main.typ             # page setup, header, footer, tables, summary, terms
├── parts/
│   ├── compute.typ      # every calculation: data in, a dictionary of results out
│   └── format.typ       # French number, money, percent and date helpers (decimal only)
└── assets/
    └── logo.svg
```

`main.typ` starts by importing the parts and computing everything once:

```typst
#import "@preview/frogst": fr-nb
#import "@preview/sepay": epc-qr-code
#import "parts/format.typ": *
#import "parts/compute.typ": compute

#let data = sys.inputs.data
#let opts = sys.inputs.layout // not `layout`: that would shadow Typst's `layout()` function
#let r = compute(data)
```

The rest of `main.typ` only *displays* values from `r` (`r.net`, `r.vat`, `r.to-pay`…): no
arithmetic is mixed with layout code. Packages are imported by name only (`frogst` for amounts
in words, `sepay` for the payment QR code, and `datify` inside `format.typ` for French dates).

## Schema: `$defs` for the shared shapes

The schema closes every object with `"additionalProperties": false` and factors the repeated
shapes into `$defs`:

| `$defs` entry | Shape | Used by |
|---|---|---|
| `amount` | string matching `^-?[0-9]+(\.[0-9]{1,4})?$` | quantities, unit prices, amendment amounts, share capital, revision coefficient |
| `percent` | number from 0 to 100 | progress, retention rate, shared costs, advance repayment |
| `vatRate` | one of `0`, `2.1`, `5.5`, `10`, `20` | contract default rate, per-line rate |
| `date` | string `YYYY-MM-DD` | contract date, invoice date, period, due date |
| `address` | `line1`, `postalCode`, `city` required; `line2`, `country` optional | company, client, worksite |
| `line` | `code`, `label`, `unit`, `quantity`, `unitPrice`, `progress`; optional `previousProgress`, `vatRate` | every item of every lot |

`data` then reads almost like a domain model: `company`, `client`, `worksite`, `contract`,
`invoice` and `lots` are required; `amendments`, `conditions` and `notes` are optional. Each
field carries a `description` where its meaning is not obvious, which shows up in `/docs` and
in generated clients. In the live OpenAPI document, the `$defs` become the components
`ProgressInvoice_amount`, `ProgressInvoice_line` and so on (see
[Exploring the API](exploring-the-api.md)).

## `layout`: every option has a default

Appearance lives in `layout`, and every property has a `default`, so a caller can send `data`
alone:

| Property | Default | Effect |
|---|---|---|
| `primaryColor` | `#1d3557` | titles, header band, table headers |
| `accentColor` | `#e76f51` | totals and the amount due |
| `font` | `Libertinus Serif` | or `New Computer Modern` (both embedded) |
| `density` | `normal` | `compact`: smaller text and tighter tables |
| `headerStyle` | `band` | `line`: a simple rule under the header |
| `logo.show` / `logo.position` / `logo.width` | `true` / `left` / `30` (mm) | logo display |
| `columns.unitPrice` / `columns.progress` / `columns.previous` | all `true` | optional table columns |
| `zebraRows`, `showLotSubtotals`, `showAmountInWords`, `showPaymentQr`, `showInsurance` | all `true` | optional rows and blocks |

Because defaults are applied recursively, `"logo": { "position": "right" }` keeps `show` and
`width`. The template reads every key directly (`opts.logo.width * 1mm`,
`opts.columns.previous`), without fallbacks. Optional columns are handled by building the
column list and each row's cells from the same visibility flags, so headers and cells always
line up.

Optional **data**, on the other hand, is always read defensively:

```typst
#let conditions = data.at("conditions", default: (:))
#let default-rate = contract.at("vatRate", default: 20)
#if "iban" in company { … }
```

## Calculations: exact decimals

Amounts arrive as decimal strings and are converted to Typst `decimal` by one helper; nothing
in a total ever goes through `float`:

```typst
#let dec(x) = if type(x) == decimal { x } else { decimal(str(x)) }
#let cents(x) = calc.round(dec(x), digits: 2) // half away from zero
```

`parts/compute.typ` documents its rules at the top and applies them in order:

1. per line: contract amount = quantity × unit price; cumulated = contract × progress;
   previous = contract × previous progress; period = cumulated − previous. Every line amount
   is rounded to the cent, and totals are exact sums of those rounded amounts;
2. per VAT rate: period works, plus price revision (works × (coefficient − 1)), minus shared
   site costs, minus advance repayment, gives the net base; VAT = base × rate, or zero under
   reverse charge;
3. retention = total including VAT × rate, when the guarantee mode is `retention`;
4. amount due = total including VAT − retention.

The result is converted to `float` in exactly one place: the `amount` argument of the SEPA QR
code, which the package requires. The repository's tests render every example request; for
the first claim, they recompute the totals independently, in integer cents, and find them in
the PDF text. The other requests are checked for page count, reverse charge and the VAT
breakdown.

## Pages: repeated headers and "page X / Y"

The company header and the legal footer are set once with `set page(header: …, footer: …)`.
The footer prints the page counter, which needs `context`:

```typst
context [Facture #invoice.number — page #counter(page).display() / #counter(page).final().first()]
```

The items table uses `table.header(repeat: true, …)`, so its column headers reappear on each
page, and `table.footer(repeat: false, …)` for the grand total. The summary, the terms and the
payment block are wrapped in `block(breakable: false)` so they are never split across pages.

Dates all come from `data` (`invoice.date`, `periodStart`, `dueDate`…) and are printed with
`datify` in French, for example « 10 octobre 2026 ». There is no `datetime.today()`, so the
same request always gives the same bytes.

## Metadata

The template sets its own PDF title from the data:

```typst
#set document(title: [Facture de situation n° #invoice.situation — #invoice.number])
```

A request's `metadata.title` overrides it, and without `metadata.author` the service's default
author is used (see [Downloads and PDF metadata](downloads-and-metadata.md)).

## The four example requests

The bodies live in `examples/requests/progress-invoice/`, and each one is also a request in the
Bruno collection (folder `5 Progress invoice`, with a fifth request that downloads the first
claim with a file name). Together they exercise every condition and most `layout` options:

| Request | Content | `layout` | `metadata` |
|---|---|---|---|
| `01-first-claim.json` | first claim, 2 lots, 4 lines, 5 % retention, IBAN and payment QR code | defaults | none: template title, default author |
| `02-long-claim-all-conditions.json` | fourth claim, 7 lots and 46 lines over several pages, 2 amendments, price revision, shared site costs, advance repayment, retention | defaults | title, two authors, subject, keywords, date |
| `03-subcontractor-reverse-charge.json` | subcontracting: VAT reverse charge (« Autoliquidation »), bank guarantee instead of retention | other colours, `compact`, `line` header, logo on the right at 22 mm, no previous-amount columns, no zebra rows, no QR code | none |
| `04-final-claim-multiple-vat-rates.json` | final claim (« Situation de solde »), contract rate 10 % with lines at other rates, no guarantee | `New Computer Modern`, other colours, no logo, no amount in words | title, single author, keywords |

Render one with:

```sh
curl -s -H 'Content-Type: application/json' \
  -d @examples/requests/progress-invoice/02-long-claim-all-conditions.json \
  -o progress-invoice.pdf \
  localhost:3000/templates/progress-invoice/render
```

To adapt the template to your own needs, start from these requests: change one thing at a
time, re-render, and add a new request file for every variant you want to keep working.
