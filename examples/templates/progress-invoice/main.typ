// French construction progress invoice (facture de situation), multi-page.
// Demo only: legal wording and calculation rules are indicative. Printed text is French.
//
// Inputs (validated by schema.json): `sys.inputs.data` (content) and `sys.inputs.layout`
// (appearance; defaults applied by the service).

#import "@preview/frogst": fr-nb
#import "@preview/sepay": epc-qr-code
#import "parts/format.typ": *
#import "parts/compute.typ": compute

#let data = sys.inputs.data
#let opts = sys.inputs.layout // not `layout`: that would shadow Typst's `layout()` function
#let r = compute(data)

#let company = data.company
#let invoice = data.invoice
#let primary = rgb(opts.primaryColor)
#let accent = rgb(opts.accentColor)
#let muted = luma(95)
#let compact = opts.density == "compact"
#let size = if compact { 8.5pt } else { 9.5pt }
#let inset = if compact { (x: 3pt, y: 2.5pt) } else { (x: 4.5pt, y: 4pt) }

#let address(a) = {
  a.line1
  if "line2" in a [ \ #a.line2]
  [ \ #a.postalCode #a.city]
  if "country" in a [ \ #a.country]
}

// --- Page header and footer (repeated on every page) ---------------------------------------

#let logo = if opts.logo.show {
  let img = image("assets/logo.svg", width: opts.logo.width * 1mm)
  // On the coloured band, the logo sits on a white tile to stay readable.
  if opts.headerStyle == "band" { box(fill: white, inset: 3pt, radius: 2pt, img) } else { img }
}
#let identity = {
  set par(spacing: 0.4em)
  text(size: 1.35em, weight: "bold", fill: if opts.headerStyle == "band" { white } else { primary }, company.name)
  linebreak()
  text(size: 0.85em)[
    #company.address.line1, #company.address.postalCode #company.address.city
    #if "phone" in company [ · #company.phone]
    #if "email" in company [ · #company.email]
  ]
}
#let header-content = {
  let cells = if opts.logo.position == "left" { (logo, identity) } else { (identity, logo) }
  let columns = if logo == none { (1fr,) } else if opts.logo.position == "left" { (auto, 1fr) } else { (1fr, auto) }
  let row = grid(
    columns: columns,
    column-gutter: 10pt,
    align: (if opts.logo.position == "left" { (left + horizon, left + horizon) } else { (left + horizon, right + horizon) }),
    ..cells.filter(c => c != none),
  )
  if opts.headerStyle == "band" {
    block(width: 100%, fill: primary, inset: (x: 10pt, y: 7pt), radius: 3pt, text(fill: white, row))
  } else {
    block(width: 100%, stroke: (bottom: 1.2pt + primary), inset: (bottom: 6pt), row)
  }
}

#let legal-line = {
  let parts = (
    company.name + " " + company.legalForm,
    if "capital" in company [au capital de #money(company.capital)],
    if "rcs" in company [RCS #company.rcs],
    [SIRET #company.siret],
    [TVA #company.vatNumber],
  ).filter(p => p != none)
  parts.join[ · ]
}

#set document(title: [Facture de situation n° #invoice.situation — #invoice.number])
#set text(font: opts.font, size: size, lang: "fr", region: "FR")
#set par(justify: false)
#set page(
  paper: "a4",
  margin: (top: 3.6cm, bottom: 2.2cm, x: 1.6cm),
  header: header-content,
  header-ascent: 22%,
  footer: {
    set text(size: 0.8em, fill: muted)
    line(length: 100%, stroke: 0.4pt + luma(180))
    grid(
      columns: (1fr, auto),
      legal-line,
      context [Facture #invoice.number — page #counter(page).display() / #counter(page).final().first()],
    )
  },
)

// --- Title and references ----------------------------------------------------------------

#let title = if invoice.at("final", default: false) [Situation de solde n° #invoice.situation] else [Facture de situation n° #invoice.situation]

#grid(
  columns: (1fr, auto),
  align: (left + bottom, right + bottom),
  text(size: 1.7em, weight: "bold", fill: primary, title),
  text(size: 1.05em)[*N° #invoice.number* \ du #date-long(invoice.date)],
)
#v(4pt)
Travaux réalisés du #date-long(invoice.periodStart) au #date-long(invoice.periodEnd).

#v(6pt)

#let box-title(t) = text(weight: "bold", fill: primary, upper(t))
#let info-box(title, body) = block(
  width: 100%,
  inset: 8pt,
  radius: 3pt,
  stroke: 0.6pt + primary.lighten(55%),
  [#box-title(title) \ #v(-4pt) #body],
)

#grid(
  columns: (1fr, 1fr),
  column-gutter: 10pt,
  row-gutter: 8pt,
  info-box("Maître d'ouvrage")[
    *#data.client.name* \
    #if "contact" in data.client [#data.client.contact \ ]
    #address(data.client.address)
    #if "vatNumber" in data.client [ \ TVA : #data.client.vatNumber]
  ],
  info-box("Chantier")[
    *#data.worksite.name* \
    #address(data.worksite.address)
    #if "reference" in data.worksite [ \ Réf. chantier : #data.worksite.reference]
  ],
  grid.cell(colspan: 2, info-box("Marché")[
    Marché n° *#data.contract.reference* du #date-short(data.contract.date)
    #if "kind" in data.contract [ — #data.contract.kind]
    #if r.reverse-charge [ — *sous-traitance*]
  ]),
)

// --- Items table -------------------------------------------------------------------------

#let show-unit = opts.columns.unitPrice
#let show-progress = opts.columns.progress
#let show-previous = opts.columns.previous

#let columns = (
  (head: [Code], width: auto, align: left, visible: true),
  (head: [Désignation], width: 1fr, align: left, visible: true),
  (head: [U], width: auto, align: center, visible: show-unit),
  (head: [Qté], width: auto, align: right, visible: show-unit),
  (head: [PU HT], width: auto, align: right, visible: show-unit),
  (head: [Marché HT], width: auto, align: right, visible: true),
  (head: [Avanc.], width: auto, align: right, visible: show-progress),
  (head: [Cumulé HT], width: auto, align: right, visible: show-previous),
  (head: [Précédent HT], width: auto, align: right, visible: show-previous),
  (head: [Période HT], width: auto, align: right, visible: true),
).filter(c => c.visible)
#let ncols = columns.len()

/// Cells of a row in full column order, filtered like the columns.
#let row-cells(code, label, unit, qty, price, market, progress, cumulated, previous, current) = (
  (code, true), (label, true), (unit, show-unit), (qty, show-unit), (price, show-unit),
  (market, true), (progress, show-progress), (cumulated, show-previous), (previous, show-previous),
  (current, true),
).filter(c => c.at(1)).map(c => c.at(0))

#let subtotal(label, values) = {
  let cells = row-cells(none, none, none, none, none, money(values.market), none,
    money(values.cumulated), money(values.previous), money(values.current))
  // The label spans the empty columns on the left.
  let first-value = cells.position(c => c != none)
  (
    table.cell(colspan: first-value, align: right, text(weight: "bold")[#label]),
    ..cells.slice(first-value).map(c => if c == none { [] } else { text(weight: "bold", c) }),
  )
}

#let body-rows = {
  let rows = ()
  for lot in r.lots {
    rows.push(table.cell(colspan: ncols, fill: primary.lighten(85%), text(weight: "bold", fill: primary)[Lot #lot.lot.code — #lot.lot.name]))
    for l in lot.lines {
      let line = l.line
      rows += row-cells(line.code, line.label, line.unit, quantity(line.quantity), money(line.unitPrice),
        money(l.market), percent(line.progress), money(l.cumulated), money(l.previous), money(l.current))
    }
    if opts.showLotSubtotals {
      rows += subtotal([Sous-total lot #lot.lot.code], lot)
    }
  }
  rows
}

#table(
  columns: columns.map(c => c.width),
  align: columns.map(c => c.align),
  inset: inset,
  stroke: (x, y) => (bottom: 0.4pt + luma(200)),
  fill: (x, y) => if y > 0 and opts.zebraRows and calc.even(y) { luma(247) },
  table.header(
    repeat: true,
    ..columns.map(c => table.cell(fill: primary, text(fill: white, weight: "bold", c.head))),
  ),
  ..body-rows,
  table.footer(repeat: false, ..subtotal([Total des travaux du marché], r.lots.fold((market: zero, cumulated: zero, previous: zero, current: zero), (acc, lot) => (
    market: acc.market + lot.market,
    cumulated: acc.cumulated + lot.cumulated,
    previous: acc.previous + lot.previous,
    current: acc.current + lot.current,
  )))),
)

// --- Amendments ----------------------------------------------------------------------------

#if r.amendments.len() > 0 {
  v(8pt)
  text(weight: "bold", fill: primary, size: 1.1em)[Avenants et travaux supplémentaires]
  table(
    columns: (auto, 1fr, auto, auto, auto, auto, auto),
    align: (left, left, right, right, right, right, right),
    inset: inset,
    stroke: (x, y) => (bottom: 0.4pt + luma(200)),
    table.header(
      ..([Réf.], [Objet], [Montant HT], [Avanc.], [Cumulé HT], [Précédent HT], [Période HT])
        .map(h => table.cell(fill: primary, text(fill: white, weight: "bold", h))),
    ),
    ..r.amendments.map(a => (
      a.amendment.reference, a.amendment.label, money(a.market), percent(a.amendment.progress),
      money(a.cumulated), money(a.previous), money(a.current),
    )).flatten(),
  )
}

// --- Summary -------------------------------------------------------------------------------

#let recap-line(label, value, strong: false, color: none) = (
  text(weight: if strong { "bold" } else { "regular" }, label),
  text(weight: if strong { "bold" } else { "regular" }, fill: if color == none { black } else { color }, value),
)

#let recap = {
  let rows = ()
  rows += recap-line[Montant total du marché HT (avenants compris)][#money(r.totals.market)]
  rows += recap-line[Travaux cumulés à fin de période HT][#money(r.totals.cumulated)]
  rows += recap-line[Déduction des situations précédentes HT][− #money(r.totals.previous)]
  rows += recap-line([Travaux de la période HT], [#money(r.totals.current)], strong: true)
  if r.coefficient != none {
    rows += recap-line[Révision de prix (#data.conditions.priceRevision.index, coef. #number(r.coefficient, decimals: 4))][#money(r.revision)]
  }
  if r.prorata != zero {
    rows += recap-line[Compte prorata (#percent(r.prorata-rate))][− #money(r.prorata)]
  }
  if r.advance != zero {
    rows += recap-line[Remboursement de l'avance (#percent(r.advance-rate))][− #money(r.advance)]
  }
  rows += recap-line([Total HT], [#money(r.net)], strong: true)
  if r.reverse-charge {
    rows += recap-line[TVA][Autoliquidation]
  } else {
    for t in r.by-rate {
      rows += recap-line[TVA #percent(t.rate) sur #money(t.net)][#money(t.vat)]
    }
  }
  rows += recap-line([Total TTC], [#money(r.gross)], strong: true)
  if r.retention != zero {
    rows += recap-line[Retenue de garantie (#percent(r.guarantee.at("rate", default: 5)) du TTC)][− #money(r.retention)]
  }
  rows
}

#v(10pt)
#block(breakable: false)[
  #grid(
    columns: (1fr, 9.5cm),
    column-gutter: 12pt,
    [
      #if opts.showAmountInWords {
        let euros = int(calc.trunc(r.to-pay))
        let centimes = int((r.to-pay - calc.trunc(r.to-pay)) * hundred)
        block(inset: 8pt, radius: 3pt, fill: luma(246), width: 100%)[
          Arrêtée la présente facture à la somme de *#fr-nb(euros) euro#if euros > 1 [s]*#if centimes > 0 [ *et #fr-nb(centimes) centime#if centimes > 1 [s]*] TTC.
        ]
      }
    ],
    [
      #table(
        columns: (1fr, auto),
        align: (left, right),
        inset: inset,
        stroke: (x, y) => (bottom: 0.4pt + luma(210)),
        ..recap,
      )
      #block(width: 100%, fill: accent, inset: 8pt, radius: 3pt, text(fill: white, weight: "bold", size: 1.15em)[
        #grid(columns: (1fr, auto), [Net à payer], [#money(r.to-pay)])
      ])
    ],
  )
]

// --- Terms and payment ---------------------------------------------------------------------

#v(10pt)
#block(breakable: false)[
  #text(weight: "bold", fill: primary, size: 1.1em)[Conditions]
  #set list(marker: text(fill: primary)[▸])
  - Échéance : *#date-long(invoice.dueDate)*#if "paymentTerms" in invoice [ — #invoice.paymentTerms].
  #if r.reverse-charge [
    - *Autoliquidation* : TVA due par le preneur (article 283-2 nonies du CGI). Montants facturés hors taxe.
  ]
  #if r.guarantee.mode == "retention" [
    - Retenue de garantie de #percent(r.guarantee.at("rate", default: 5)) (loi n° 75-1334 du 16 juillet 1975), libérée à l'expiration du délai de garantie de parfait achèvement.
  ] else if r.guarantee.mode == "bank-guarantee" [
    - Retenue de garantie remplacée par une caution bancaire#if "guarantor" in r.guarantee [ délivrée par #r.guarantee.guarantor].
  ]
  - En cas de retard de paiement : pénalités au taux de trois fois le taux d'intérêt légal et indemnité forfaitaire pour frais de recouvrement de 40 € (articles L. 441-10 et D. 441-5 du Code de commerce). Pas d'escompte pour paiement anticipé.
  #if opts.showInsurance and "insurance" in company [
    - Assurance décennale : #company.insurance.insurer, police n° #company.insurance.policy#if "coverage" in company.insurance [ (#company.insurance.coverage)].
  ]
  #for note in data.at("notes", default: ()) [
    - #note
  ]
]

#if "iban" in company {
  v(8pt)
  block(breakable: false, width: 100%, inset: 8pt, radius: 3pt, stroke: 0.6pt + primary.lighten(55%))[
    #grid(
      columns: (1fr, auto),
      column-gutter: 12pt,
      align: horizon,
      [
        #box-title("Règlement par virement") \
        Bénéficiaire : #company.name \
        IBAN : #company.iban #if "bic" in company [ — BIC : #company.bic] \
        Référence : #invoice.at("paymentReference", default: invoice.number)
      ],
      if opts.showPaymentQr and r.to-pay > zero {
        epc-qr-code(
          company.name,
          company.iban,
          amount: float(r.to-pay),
          bic: company.at("bic", default: none),
          reference: invoice.at("paymentReference", default: invoice.number),
          width: 2.6cm,
          height: 2.6cm,
        )
      },
    )
  ]
}
