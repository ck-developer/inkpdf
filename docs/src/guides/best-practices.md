# Best practices

These practices keep templates predictable for callers, AI assistants included, and easy to
maintain. The `demo` template of the repository's example volume follows them;
[Writing a template](writing-a-template.md) builds it step by step.

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
- **Constrain values** with `pattern`, `enum`, `minimum`/`maximum`, `minLength`/`maxLength`,
  `minItems`/`maxItems`, and standard formats (`"format": "date"`, `"format": "email"`), which
  are checked. Validation is cheap and runs before Typst, with every violation listed at once.
- **Bound texts and lists**: a `maxLength` and a `maxItems` keep a render within its time
  limit and tell callers what fits on the page.

```json
"$defs": {
  "amount": {
    "type": "string",
    "pattern": "^-?[0-9]+(\\.[0-9]{1,4})?$",
    "description": "Exact decimal amount as a string, e.g. \"1250.00\"."
  },
  "date": {
    "type": "string",
    "format": "date",
    "description": "A calendar date, YYYY-MM-DD."
  }
}
```

A shared shape does not know what it is used for: give the role, and a precise description,
where it is used, next to the `$ref`:

```json
"dueDate": { "$ref": "#/$defs/date", "description": "Payment due date, YYYY-MM-DD.", "x-role": "ask" }
```

## Describe every variable, and give it a role

The schema is also what an AI assistant reads to fill the template. Two annotations on every
property of `data` and `layout`, at every depth, let it work without guessing:

- **`description`**: what the value is, its unit and format, and, when the assistant has to
  compute it, the rule ("Build it as DEMO-YYYY-NNN if the user gives none", "Today's date unless
  the user says otherwise"). It also shows up in `/docs` and in generated clients.
- **`x-role`**: who provides the value.

| Role | Give it to | Examples |
|---|---|---|
| `ask` | what only the end user knows | names, addresses, the items of a list, amounts, free text |
| `fill` | what the assistant can work out from the context or a rule | today's date, a reference built from a pattern, a value derived from other answers |
| `default` | options and optional content, used only when someone mentions them | a note, a flag, a status; everything in `layout` |

```json
"reference": {
  "type": "string",
  "pattern": "^[A-Z0-9-]{3,30}$",
  "description": "Document reference. Build it as DEMO-YYYY-NNN if the user gives none.",
  "x-role": "fill"
}
```

- A list and the properties of its items each have their own role: the list of lines can be
  `ask` and an item's position number `fill`.
- Without a role, an assistant treats a required variable as `ask` and an optional one as
  `default`. That works, but says nothing about values it could fill in itself: it will ask for
  them.
- A role that is not `ask`, `fill` or `default` makes the template invalid, with the location
  of the mistake in `reason`.
- Adding descriptions and roles to an existing template never changes what it accepts: callers
  are not affected.

See [Template format → Roles](../reference/template-format.md#roles-x-role).

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
`opts.primaryColor`): the schema guarantees they exist. A `default` declared inside `data` is
documentation only, never applied: read such a property with `.at(…, default: …)` too.

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

## Dates: `format: "date"` in the schema, values in `data`

Declare every date as a string with `"format": "date"`. The service then checks that it is a
real calendar date, written `YYYY-MM-DD`: `2026-02-30` is refused with `422 validation-failed`
and the path of the field, instead of failing later in Typst without it. A `pattern` alone only
checks the shape. For a date the assistant can work out, give it the role `fill` and say the
rule in its description ("Today's date unless the user says otherwise").

Never use `datetime.today()`: the PDF would change from one day to the next for the same
request, and tests could not compare outputs. Pass every date in the body and parse it in
Typst:

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

## Ship an example, and test it

Give every template an `example.json`: one complete, realistic request body, with obviously
fictitious names and addresses. The service checks it when it loads the template and returns it
as `example` in the template's details, so callers and AI assistants see at once what a valid
input looks like, and `/docs` offers it as the example request. An example that does not pass
the schema is dropped and logged (`template.example_invalid`), so a broken example never
misleads; watch the logs for it.

Render the example in CI, and keep a few more request bodies under version control for the
main variants: optional blocks on and off, long content that spans several pages, every
`layout` option. Render them all with `curl` and check for `200`, or run them as a
[Bruno](https://www.usebruno.com) collection, as the repository does in `examples/bruno` (see
[Exploring the API](exploring-the-api.md#the-bruno-collection)).

## Version templates instead of editing them

Once callers use a template, publish changes as a new version folder rather than editing the
existing one: copy `1.0.0/` to `1.1.0/` (a compatible change) or `2.0.0/` (an existing request
would stop being valid or would render differently: a removed or renamed property, a new
required one, a narrower constraint), and change the copy. Callers that ask for `version=1` keep
getting version 1; the others get the new default, the highest valid version. Remove the
versions nobody uses any more: every version stays in memory. See
[Templates → Versions](../concepts/templates.md#versions).

## What not to do

- **Do not leave a variable without a `description` and an `x-role`**: assistants then guess,
  ask too much, or invent values.
- **Do not declare a `format` you do not mean**: formats are checked, so `"format": "email"`
  on a free-text field refuses everything that is not an e-mail address.
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
