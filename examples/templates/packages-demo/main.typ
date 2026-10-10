// Paquets intégrés à inkpdf : importés par leur seul nom, sans version.
#import "@preview/tiaoma"
#import "@preview/zero": num
#import "@preview/cetz"
#import "@preview/cetz-plot": chart

#let data = sys.inputs.data
#let accent = rgb(sys.inputs.layout.accentColor)

#set page(paper: "a4", margin: 2cm)
#set text(lang: "fr", size: 11pt)

#text(size: 20pt, weight: "bold", fill: accent, data.title)

#grid(
  columns: (1fr, auto),
  gutter: 1cm,
  [
    Montant : *#num(data.amount, digits: 2, decimal-separator: ",", group: (size: 3, separator: sym.space, threshold: 4)) €*

    Référence : #raw(data.reference)
  ],
  box(width: 3cm, tiaoma.qrcode(data.reference)),
)

#v(1cm)

#cetz.canvas({
  chart.columnchart(
    data.series.map(item => (item.label, item.value)),
    size: (10, 5),
    bar-style: (fill: accent, stroke: none),
  )
})
