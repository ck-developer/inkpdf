// Packages bundled with inkpdf: imported by name only, without a version.
#import "@preview/tiaoma"
#import "@preview/zero": num
#import "@preview/cetz"
#import "@preview/cetz-plot": chart

#let data = sys.inputs.data
#let accent = rgb(sys.inputs.layout.accentColor)

#set page(paper: "a4", margin: 2cm)
#set text(lang: "en", size: 11pt)

#text(size: 20pt, weight: "bold", fill: accent, data.title)

#grid(
  columns: (1fr, auto),
  gutter: 1cm,
  [
    Amount: *€#num(data.amount, digits: 2, decimal-separator: ".", group: (size: 3, separator: ",", threshold: 4))*

    Reference: #raw(data.reference)
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
