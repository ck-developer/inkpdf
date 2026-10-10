// French formatting of numbers and dates. Amounts stay `decimal` (exact arithmetic);
// no `float` is involved in totals.

#import "@preview/datify": custom-date-format

/// Converts a JSON string, integer or float to `decimal`.
#let dec(x) = if type(x) == decimal { x } else { decimal(str(x)) }

#let zero = decimal("0")
#let hundred = decimal("100")

/// Rounds to the cent (half away from zero).
#let cents(x) = calc.round(dec(x), digits: 2)

/// Groups digits by thousands with a narrow no-break space.
#let group-digits(digits) = {
  let chars = digits.clusters()
  let n = chars.len()
  let out = ""
  for (i, c) in chars.enumerate() {
    if i > 0 and calc.rem(n - i, 3) == 0 { out += "\u{202F}" }
    out += c
  }
  out
}

/// French number: `1 234,50`, with `decimals` digits after the comma (zero-padded).
#let number(x, decimals: 2) = {
  let v = calc.round(dec(x), digits: decimals)
  let negative = v < zero
  let parts = str(calc.abs(v)).split(".")
  let integer = group-digits(parts.at(0))
  let fraction = if parts.len() > 1 { parts.at(1) } else { "" }
  while fraction.len() < decimals { fraction += "0" }
  (if negative { "−" } else { "" }) + integer + (if decimals > 0 { "," + fraction } else { "" })
}

/// Amount in euros: `1 234,56 €`.
#let money(x) = number(x) + "\u{00A0}€"

/// Percentage without useless decimals: `37,5 %`, `100 %`.
#let percent(x) = {
  let v = dec(x)
  let decimals = if v == calc.round(v, digits: 0) { 0 } else if v == calc.round(v, digits: 1) { 1 } else { 2 }
  number(v, decimals: decimals) + "\u{00A0}%"
}

/// Quantity: up to 3 decimals, without trailing zeros.
#let quantity(x) = {
  let v = dec(x)
  let decimals = if v == calc.round(v, digits: 0) { 0 } else if v == calc.round(v, digits: 1) { 1 } else if v == calc.round(v, digits: 2) { 2 } else { 3 }
  number(v, decimals: decimals)
}

/// `"2026-10-10"` → datetime.
#let parse-date(s) = {
  let p = s.split("-").map(int)
  datetime(year: p.at(0), month: p.at(1), day: p.at(2))
}

/// `"2026-10-10"` → « 10 octobre 2026 » (French long date).
#let date-long(s) = custom-date-format(parse-date(s), pattern: "long", lang: "fr")

/// `"2026-10-10"` → « 10/10/2026 ».
#let date-short(s) = parse-date(s).display("[day]/[month]/[year]")
