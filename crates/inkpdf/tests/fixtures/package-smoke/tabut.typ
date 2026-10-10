#import "@preview/tabut": tabut
#let supplies = ((name: "Pencil", price: 1.5), (name: "Notebook", price: 3))
#tabut(supplies, ((header: [Item], func: r => r.name), (header: [Price], func: r => r.price)))
