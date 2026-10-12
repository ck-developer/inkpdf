// Neutral demo template: a header, a recipient, a table of lines with a total, and a note.
// The service passes the validated input in `sys.inputs`, with every `layout` default filled in.
// Defaults never apply to `data`: optional properties are read with `.at(…, default: …)`.
#import "@preview/zero": num

#let data = sys.inputs.data
#let opts = sys.inputs.layout // not `layout`: that name would hide Typst's layout() function

#let accent = rgb(opts.accentColor)
#let compact = opts.density == "compact"
#let status = data.at("status", default: "draft")
#let urgent = data.at("urgent", default: false)

/// `"2026-10-12"` → a datetime (the schema guarantees a valid calendar date).
#let parse-date(s) = {
  let p = s.split("-").map(int)
  datetime(year: p.at(0), month: p.at(1), day: p.at(2))
}

/// Exact amount (Typst `decimal`) → `1,234.50`, formatted by the bundled `zero` package.
#let money(x) = num(
  str(calc.round(x, digits: 2)),
  digits: 2,
  decimal-separator: ".",
  group: (size: 3, separator: ",", threshold: 4),
  math: false, // document font and weight instead of the math font
)

/// Free text with line breaks kept.
#let lines-of(s) = s.split("\n").join(linebreak())

#set document(title: data.title)
#set page(
  paper: "a4",
  margin: if compact { 1.5cm } else { 2cm },
  background: if status == "draft" {
    rotate(-40deg, text(110pt, weight: "bold", fill: luma(235))[DRAFT])
  },
  footer: context align(right, text(9pt, fill: luma(90))[
    page #counter(page).display() / #counter(page).final().first()
  ]),
)
#set text(font: opts.font, size: if compact { 9.5pt } else { 11pt }, lang: "en")
#set par(spacing: if compact { 0.8em } else { 1.2em })

// Header: title and badges on the left, reference, date and status on the right.
#grid(
  columns: (1fr, auto),
  gutter: 1em,
  [
    #text(20pt, weight: "bold", fill: accent, data.title)
    #if urgent {
      h(0.5em)
      box(
        fill: rgb("#b42318"),
        inset: (x: 0.5em, y: 0.3em),
        radius: 3pt,
        baseline: 20%,
        text(9pt, weight: "bold", fill: white)[URGENT],
      )
    }
  ],
  align(right)[
    #data.reference \
    #parse-date(data.date).display("[month repr:long] [day padding:none], [year]") \
    #text(weight: "bold", fill: accent, upper(status))
  ],
)

#v(if compact { 0.5em } else { 1em })

// Recipient.
#block[
  #text(9pt, fill: luma(90))[To] \
  *#data.recipient.name* \
  #lines-of(data.recipient.address)
  #if "email" in data.recipient [ \ #data.recipient.email ]
]

#v(if compact { 0.5em } else { 1em })

// Lines, with a header repeated on every page.
#let line-amount(l) = decimal(l.amount) * l.quantity
#table(
  columns: (1fr, auto, auto, auto),
  align: (left, right, right, right),
  inset: if compact { 4pt } else { 6pt },
  stroke: (x: none, y: 0.5pt + luma(200)),
  fill: (_, y) => if y == 0 { accent },
  table.header(
    repeat: true,
    ..([Label], [Quantity], [Unit amount (EUR)], [Amount (EUR)]).map(text.with(fill: white, weight: "bold")),
  ),
  ..data.lines.map(l => (l.label, str(l.quantity), money(decimal(l.amount)), money(line-amount(l)))).flatten(),
)

#if opts.showTotal {
  let total = data.lines.map(line-amount).sum()
  align(right, text(weight: "bold")[Total: #money(total) EUR])
}

#if "note" in data {
  v(if compact { 0.5em } else { 1em })
  block(inset: (left: 0.8em), stroke: (left: 2pt + accent), emph(lines-of(data.note)))
}
