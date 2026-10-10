#import "@preview/modern-mailmerge": mail-merge, field
#mail-merge(((name: "Alice"), (name: "Bob")), record => [Bonjour #field(record, "name")])
