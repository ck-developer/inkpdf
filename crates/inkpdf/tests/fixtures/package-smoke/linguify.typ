#import "@preview/linguify": set-database, linguify
#set-database((conf: (default-lang: "en"), lang: (en: (hello: "Hello"), fr: (hello: "Bonjour"))))
#set text(lang: "fr")
#linguify("hello")
