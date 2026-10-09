// Template de démonstration neutre : un titre et un tableau de libellés/valeurs.
// L'entrée validée est fournie par le service dans `sys.inputs`.
#let data = sys.inputs.data
#let design = sys.inputs.design

#let primary = rgb(design.primaryColor)
#let aligns = (left: left, center: center, right: right)

#set page(paper: "a4", margin: 2cm)
#set text(font: "Libertinus Serif", size: 11pt)

#align(aligns.at(design.align))[
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

#if design.showFooter [ #include "parts/footer.typ" ]
