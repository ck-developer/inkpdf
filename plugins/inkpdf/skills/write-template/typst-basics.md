# Typst for inkpdf templates

inkpdf compiles `main.typ` with Typst 0.15. The template reads its input from `sys.inputs`; it
never receives code. This page covers what a template usually needs; the full language is at
<https://typst.app/docs>.

## Reading the input

```typst
#let data = sys.inputs.data
#let opts = sys.inputs.layout // not `layout`: that name would hide Typst's layout() function
```

- `data` and `opts` are dictionaries. The server has validated them against the schema, and
  `opts` has every `layout` default filled in.
- `data.title` fails when the key is missing. For an optional property, test it or give a
  fallback:

  ```typst
  #if "note" in data [ #data.note ]
  #let email = data.recipient.at("email", default: none)
  ```

- JSON values become: object → dictionary, array → array, string → `str`, integer → `int`,
  number with a fraction → `float`, `true`/`false` → `bool`, `null` → `none`.
- Strings are always printed as text, never run: a value `#import "x"` shows up as is.
- Concatenating a number with text needs `str(n)`.

## Page setup and text

```typst
#set document(title: "Attendance sheet") // metadata.title from the input wins over this
#set page(
  paper: "a4",
  margin: 2cm,
  footer: context align(right)[
    page #counter(page).display() / #counter(page).final().first()
  ],
)
#set text(font: "Libertinus Serif", size: 11pt, lang: "en")
#set par(justify: true)

#text(size: 20pt, weight: "bold", fill: rgb(opts.accentColor))[#data.title]
```

- Page counters need `context`.
- A colour from the input: `rgb("#1f4e79")` (validate it with a pattern in the schema).
- A choice from the input mapped to Typst values:

  ```typst
  #let sizes = (normal: 11pt, compact: 9pt)
  #set text(size: sizes.at(opts.density))
  ```

## Tables

```typst
#table(
  columns: (1fr, auto, auto),
  align: (left, right, right),
  fill: (_, y) => if y == 0 { rgb(opts.accentColor) },
  table.header(repeat: true, ..([Label], [Quantity], [Amount]).map(text.with(fill: white, weight: "bold"))),
  ..data.lines.map(l => (l.label, str(l.quantity), l.amount)).flatten(),
)
```

- Tables flow over several pages; `table.header(repeat: true, …)` repeats the header row.
- Each row has one cell per column: `.map(…).flatten()` turns a list of rows into cells.
- Keep a block on one page with `block(breakable: false)[…]`.

## Dates

Dates arrive as `YYYY-MM-DD` strings (`"format": "date"` in the schema).

```typst
#let parse-date(s) = {
  let p = s.split("-").map(int)
  datetime(year: p.at(0), month: p.at(1), day: p.at(2))
}
#parse-date(data.date).display("[month repr:long] [day padding:none], [year]")
```

- `display()` writes month names in English only. For another language, use the bundled
  `datify` package (see packages.md).
- Never use `datetime.today()`: the same input must give the same PDF. Dates come from the input.

## Amounts

Amounts arrive as decimal strings (`"12.50"`). Compute with `decimal`, which is exact; a JSON
number with a fraction would be a `float`, which is not.

```typst
#import "@preview/zero": num

#let total = data.lines.map(l => decimal(l.amount) * l.quantity).sum(default: decimal("0"))
#let money(x) = num(str(calc.round(x, digits: 2)), digits: 2, group: (size: 3, separator: ",", threshold: 4))

Total: #money(total) EUR
```

Typst has no thousands separator of its own: use the bundled `zero` package, or a small
helper.

## Splitting the template

```typst
#import "parts/format.typ": money, parse-date   // functions
#include "parts/footer.typ"                      // a block of content
#image("assets/logo.svg", width: 3cm)
```

Paths are relative to the file that uses them, and must stay inside the version folder.

## Fonts

Always available: **Libertinus Serif**, **New Computer Modern** and **DejaVu Sans Mono**. Other
fonts go as `.ttf`, `.otf` or `.ttc` files in the template's `fonts/` folder; check their
licence. System fonts and fonts inside packages are never loaded. Set the font explicitly, and
when callers may choose it, offer it as a `layout` `enum` with a `default`.

## Packages

Only the packages listed in [packages.md](packages.md) exist, imported by name only:
`#import "@preview/zero": num`, never `@preview/zero:0.7.1`. A package reads only its own files:
load an image in the template and pass it as an argument.

## Common errors

| Diagnostic | Cause |
|---|---|
| `dictionary does not contain key "note"` | An optional property read with `.note`: use `.at("note", default: none)` or `"note" in data` |
| `cannot add integer and string` (or similar) | Concatenating a number with text: use `str(n)` |
| `can only be used when context is known` | `counter(page)` or `here()` outside `context` |
| `unknown variable: layout` / `layout` misbehaves | `sys.inputs.layout` bound to the name `layout`: call it `opts` |
| `file not found` | A path outside the version folder, or a misspelt file name |
