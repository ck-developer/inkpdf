# Best practices

These practices keep templates predictable for callers and easy to maintain. The
[progress invoice walkthrough](progress-invoice.md) shows most of them in a real template.

## Design the schema as the API

The schema is the contract with your callers: it is what they see in `GET /templates/{id}`,
in the live OpenAPI document and in generated clients. Make it strict and explicit.

- **Close every object** with `"additionalProperties": false`: the root, `data`, `layout` and
  every nested object. A typo in a field name then fails with `422` instead of being silently
  ignored.
- **Declare `"required": ["data"]`** at the root, and list the required fields of every object.
  Anything not required must be optional in the Typst code too (see below).
- **Factor repeated shapes into `$defs`** and reference them with `"$ref": "#/$defs/<name>"`:
  an amount, a date, an address, a table line. Each `$defs` entry becomes its own named
  component in the live OpenAPI document (`<Template>_<name>`), so generated clients get
  reusable types. Only internal references (starting with `#`) are allowed.
- **Constrain values** with `pattern`, `enum`, `minimum`/`maximum`, `minLength`, `minItems`.
  Validation is cheap and runs before Typst, with every violation listed at once.
- **Document fields** with `description`: it shows up in `/docs` and in generated clients.

```json
"$defs": {
  "amount": {
    "type": "string",
    "pattern": "^-?[0-9]+(\\.[0-9]{1,4})?$",
    "description": "Exact decimal amount as a string, e.g. \"1250.00\"."
  },
  "date": { "type": "string", "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}$" }
}
```

## Keep content in `data`, appearance in `layout`

`data` is what the document says; `layout` is how it looks (colours, font, density, logo
position, optional columns or blocks). Callers that do not care about appearance send only
`data`.

**Give every `layout` property a `default`.** The service fills in missing defaults,
recursively into sub-objects, before validation. The template can then read any declared
`layout` key without a fallback, and a caller can override a single nested value:

```json
"logo": {
  "type": "object",
  "additionalProperties": false,
  "properties": {
    "show":     { "type": "boolean", "default": true },
    "position": { "enum": ["left", "right"], "default": "left" },
    "width":    { "type": "number", "minimum": 10, "maximum": 60, "default": 30 }
  }
}
```

With this schema, `"layout": { "logo": { "position": "right" } }` yields
`{ "show": true, "position": "right", "width": 30 }`. Defaults are never applied to `data`.

## Bind the inputs once, and call the layout `opts`

```typst
#let data = sys.inputs.data
#let opts = sys.inputs.layout // not `layout`: that would shadow Typst's `layout()` function
```

`layout` is a built-in Typst function. Binding `#let layout = sys.inputs.layout` hides it for
the whole file, and any later use of `layout(size => …)`, including inside code you copy in,
breaks with a confusing error. Use `opts` (or `style`, `settings`).

## Read optional inputs safely

Field access with a dot fails when the key is missing. For anything that is not required by
the schema:

```typst
// Test for presence.
#if "notes" in data [ #data.notes ]

// Or fall back to a default.
#let conditions = data.at("conditions", default: (:))
#let rate = conditions.at("prorataRate", default: 0)
```

Required fields and `layout` keys with defaults can be read directly (`data.invoice.number`,
`opts.primaryColor`): the schema guarantees they exist.

## Exact money: decimal strings and Typst `decimal`

JSON numbers with a fraction reach Typst as `float`, and floats cannot represent most amounts
exactly (`0.1 + 0.2 != 0.3`). For money:

1. Declare amounts as **strings** with a decimal pattern in the schema (or as integer cents).
2. Convert them to Typst `decimal` at the boundary, and keep all arithmetic in `decimal`.
3. Round explicitly where the business rules say so.

```typst
/// Converts a JSON string, integer or float to `decimal`.
#let dec(x) = if type(x) == decimal { x } else { decimal(str(x)) }
#let hundred = decimal("100")

/// Rounds to the cent (half away from zero).
#let cents(x) = calc.round(dec(x), digits: 2)

#let total = cents(dec(line.quantity) * dec(line.unitPrice))
#let vat = cents(total * dec(rate) / hundred)
```

Convert to `float` only at the very last moment, when a package requires it (for example a
payment QR code that takes `amount: float(...)`). Typst has no localized number formatting:
write a small helper or use the [`zero`](packages.md) package.

## Pass dates in `data`

Never use `datetime.today()`: the PDF would change from one day to the next for the same
request, and tests could not compare outputs. Pass every date in the body, as an ISO string
validated by the schema, and parse it in Typst:

```typst
#let parse-date(s) = {
  let p = s.split("-").map(int)
  datetime(year: p.at(0), month: p.at(1), day: p.at(2))
}

#parse-date(data.invoice.date).display("[day]/[month]/[year]")
```

`display()` prints month names in English only. For other languages, use the `datify`
package (`custom-date-format(date, pattern: "long", lang: "fr")`).

## Split the template into `parts/`

Keep `main.typ` about the document structure and move the rest into files:

```text
invoice/
├── main.typ             # page setup, header, footer, sections
└── parts/
    ├── format.typ       # number, money, percent and date helpers
    └── compute.typ      # every calculation, from `data` to a dictionary of results
```

```typst
#import "parts/format.typ": *
#import "parts/compute.typ": compute

#let r = compute(data)
```

Doing all calculations up front in one function, then only displaying `r.<value>`, makes the
logic testable and keeps layout code readable. Use `#include` for static blocks and `#import`
for functions.

## Multi-page tables and page numbers

Let tables flow across pages, and repeat their header row on every page:

```typst
#table(
  columns: (auto, 1fr, auto),
  table.header(repeat: true, [Code], [Description], [Amount]),
  ..rows,
  table.footer(repeat: false, [], [*Total*], [*#money(total)*]), // `money`: your formatting helper
)
```

Put repeated page content in `page(header:)` and `page(footer:)`. Page counters need
`context`; `counter(page).final()` gives the total number of pages:

```typst
#set page(
  footer: context [
    #h(1fr) Page #counter(page).display() / #counter(page).final().first()
  ],
)
```

Wrap blocks that must not be split (totals, signature, payment details) in
`block(breakable: false)[…]`. Avoid layouts whose size depends on the page count or on their own
final state: Typst stops after a few layout iterations, and each one costs a full re-layout.

## Fonts

Three fonts are always available: **Libertinus Serif**, **New Computer Modern** and
**DejaVu Sans Mono**. To use another one, put its `.ttf`, `.otf` or `.ttc` files in the
template's `fonts/` folder. System fonts are never available, and fonts shipped inside
packages are not loaded. Check the licence of any font you add.

Set the font explicitly (`#set text(font: "Libertinus Serif")`) rather than relying on
fallback, and if callers may choose it, offer it as a `layout` `enum` with a `default`.

## Keep renders deterministic

The same template and the same body produce byte-identical PDFs. Keep it that way:

- no `datetime.today()`;
- no randomness that is not seeded from the input;
- no data that is not in the body or in the template folder (there is no network anyway);
- give the PDF a date only through `metadata.date` or an explicit `set document(date: …)`
  computed from `data`.

Determinism lets you compare PDFs byte for byte in tests and cache them safely. See
[Sandbox and determinism](../concepts/sandbox.md).

## Keep example requests next to the template, and test them

For every template, keep a few realistic request bodies under version control, covering the
main variants (optional blocks on and off, long content that spans several pages, every
`layout` option). In the inkpdf repository they live in `examples/requests/`, and a test renders every one of
them.

Make them runnable by hand too: the repository's [Bruno](https://www.usebruno.com) collection
(`examples/bruno`) has one request per example. A request whose `docs` block contains
`Source: examples/requests/<file>.json` must have exactly that body; a test checks it, so the
collection never drifts from the examples. Do the same in your own repository, or at least
render your examples in CI with `curl` and check for `200`. See
[Exploring the API](exploring-the-api.md#the-bruno-collection).

## What not to do

- **Do not build Typst source from caller strings.** Data reaches Typst only through
  `sys.inputs`, as values; never `eval` a caller string, never generate a `.typ` file per
  request. Strings in `data` are printed, never executed.
- **Do not write package versions** (`@preview/zero:0.7.1`): the template becomes invalid.
  Import by name only.
- **Do not reach outside the template folder** (`../shared/logo.png`, absolute paths): the
  render fails. Copy shared assets into each template.
- **Do not expect network access, environment variables or system fonts**: none exist inside
  the sandbox.
- **Do not use `float` for money**, and do not compute totals from rounded display strings.
- **Do not use `datetime.today()`** or anything else that changes between two identical
  requests.
- **Do not put appearance switches in `data`**, or content in `layout`.
- **Do not run long computations** in Typst loops or WASM-based packages: a render is bounded
  by a timeout. See [Limits and performance](../operations/limits.md).
