// Neutral demo template: a title and a table of labels and values.
// The service passes the validated input in `sys.inputs`.
#let data = sys.inputs.data
#let opts = sys.inputs.layout  // not `layout`: that name would shadow Typst's `layout()` function

#let primary = rgb(opts.primaryColor)
#let aligns = (left: left, center: center, right: right)

#set page(paper: "a4", margin: 2cm)
#set text(font: "Libertinus Serif", size: 11pt)

#align(aligns.at(opts.align))[
  #text(fill: primary, size: 20pt, weight: "bold")[#data.title]
]

#table(
  columns: (1fr, auto),
  fill: (_, y) => if y == 0 { primary },
  table.header(
    text(fill: white, weight: "bold")[Label],
    text(fill: white, weight: "bold")[Value],
  ),
  ..data.items.map(i => (i.label, str(i.value))).flatten(),
)

#if opts.showFooter [ #include "parts/footer.typ" ]
