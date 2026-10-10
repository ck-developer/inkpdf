// Progress-claim calculations. Every line amount is rounded to the cent; totals are exact
// sums of those rounded amounts.
//
// Per line: contract = quantity × unit price; cumulated = contract × progress;
// previous = contract × previous progress; period = cumulated − previous.
// Per VAT rate: period works (+ price revision) − shared site costs − advance repayment
// = net base; VAT = base × rate (0 under reverse charge).
// Retention = total incl. VAT × rate (mode `retention`). Amount due = total − retention.

#import "format.typ": dec, zero, hundred, cents

#let line-amounts(market, progress, previous-progress) = {
  let cumulated = cents(market * dec(progress) / hundred)
  let previous = cents(market * dec(previous-progress) / hundred)
  (market: market, cumulated: cumulated, previous: previous, current: cumulated - previous)
}

#let sum(items, key) = items.fold(zero, (acc, it) => acc + it.at(key))

/// Computes every displayed value from `data`.
#let compute(data) = {
  let contract = data.contract
  let conditions = data.at("conditions", default: (:))
  let default-rate = contract.at("vatRate", default: 20)
  let reverse-charge = contract.at("subcontracting", default: false)

  // Items, grouped by lot.
  let lots = data.lots.map(lot => {
    let lines = lot.lines.map(line => {
      let market = cents(dec(line.quantity) * dec(line.unitPrice))
      line-amounts(market, line.progress, line.at("previousProgress", default: 0)) + (
        line: line,
        rate: dec(line.at("vatRate", default: default-rate)),
      )
    })
    (
      lot: lot,
      lines: lines,
      market: sum(lines, "market"),
      cumulated: sum(lines, "cumulated"),
      previous: sum(lines, "previous"),
      current: sum(lines, "current"),
    )
  })

  // Amendments.
  let amendments = data.at("amendments", default: ()).map(a => {
    line-amounts(cents(a.amount), a.progress, a.at("previousProgress", default: 0)) + (
      amendment: a,
      rate: dec(a.at("vatRate", default: default-rate)),
    )
  })

  let all-lines = lots.map(l => l.lines).flatten() + amendments
  let totals = (
    market: sum(all-lines, "market"),
    cumulated: sum(all-lines, "cumulated"),
    previous: sum(all-lines, "previous"),
    current: sum(all-lines, "current"),
  )

  // Breakdown per VAT rate.
  let revision = conditions.at("priceRevision", default: none)
  let coefficient = if revision == none { none } else { dec(revision.coefficient) }
  let prorata-rate = dec(conditions.at("prorataRate", default: 0))
  let advance-rate = dec(conditions.at("advanceRepaymentRate", default: 0))

  let rates = all-lines.map(l => l.rate).dedup().sorted(key: r => -r)
  let by-rate = rates.map(rate => {
    let works = sum(all-lines.filter(l => l.rate == rate), "current")
    let revised = if coefficient == none { zero } else { cents(works * (coefficient - decimal("1"))) }
    let base = works + revised
    let prorata = cents(base * prorata-rate / hundred)
    let advance = cents(base * advance-rate / hundred)
    let net = base - prorata - advance
    let vat = if reverse-charge { zero } else { cents(net * rate / hundred) }
    (rate: rate, works: works, revision: revised, prorata: prorata, advance: advance, net: net, vat: vat)
  })

  let net = sum(by-rate, "net")
  let vat = sum(by-rate, "vat")
  let gross = net + vat

  let guarantee = conditions.at("guarantee", default: (mode: "none"))
  let retention = if guarantee.mode == "retention" {
    cents(gross * dec(guarantee.at("rate", default: 5)) / hundred)
  } else { zero }

  (
    lots: lots,
    amendments: amendments,
    totals: totals,
    by-rate: by-rate,
    revision: sum(by-rate, "revision"),
    prorata: sum(by-rate, "prorata"),
    advance: sum(by-rate, "advance"),
    net: net,
    vat: vat,
    gross: gross,
    retention: retention,
    to-pay: gross - retention,
    reverse-charge: reverse-charge,
    guarantee: guarantee,
    coefficient: coefficient,
    prorata-rate: prorata-rate,
    advance-rate: advance-rate,
  )
}
