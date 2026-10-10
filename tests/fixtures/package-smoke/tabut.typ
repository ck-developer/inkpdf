#import "@preview/tabut": tabut
#let supplies = ((name: "Crayon", price: 1.5), (name: "Cahier", price: 3))
#tabut(supplies, ((header: [Article], func: r => r.name), (header: [Prix], func: r => r.price)))
