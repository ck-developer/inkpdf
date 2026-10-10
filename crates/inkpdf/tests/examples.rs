//! 002/US8: every example request renders, and the progress-invoice totals are right to the cent.

mod common;

use std::fs;
use std::path::PathBuf;

use axum::http::StatusCode;
use common::*;
use serde_json::Value;

fn examples() -> PathBuf {
    repo_root().join("examples")
}

fn volume() -> TestVolume {
    let volume = TestVolume::new();
    for template in ["sample", "packages-demo", "progress-invoice"] {
        volume.copy_template(&examples().join("templates").join(template), template);
    }
    volume
}

fn request(path: &str) -> Value {
    serde_json::from_slice(&fs::read(examples().join("requests").join(path)).unwrap()).unwrap()
}

async fn render(app: &axum::Router, template: &str, body: &Value) -> Vec<u8> {
    let (status, _, pdf) = post_json(app, &format!("/templates/{template}/render"), body).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "{template}: {}",
        String::from_utf8_lossy(&pdf)
    );
    pdf.to_vec()
}

/// Number of pages (`/Type/Page` objects, not `/Type/Pages`).
fn page_count(pdf: &[u8]) -> usize {
    pdf.windows(11)
        .filter(|w| w.starts_with(b"/Type/Page") && w[10] != b's')
        .count()
}

/// PDF text with every whitespace removed (amounts use narrow no-break spaces).
fn compact_text(pdf: &[u8]) -> String {
    pdf_text(pdf)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// French amount as printed by the template, without spaces: `8550,00€`.
fn euros(cents: i128) -> String {
    format!("{},{:02}€", cents / 100, (cents % 100).abs())
}

/// `"18.50"` → 1850 (amounts in the examples have at most 2 decimals).
fn to_cents(amount: &str) -> i128 {
    let (int, frac) = amount.split_once('.').unwrap_or((amount, ""));
    let frac = format!("{frac:0<2}");
    int.parse::<i128>().unwrap() * 100 + frac[..2].parse::<i128>().unwrap()
}

/// `value × percent / 100`, rounded to the cent, half away from zero.
fn percent_of(cents: i128, percent: i128) -> i128 {
    let scaled = cents * percent;
    (scaled + scaled.signum() * 50) / 100
}

#[tokio::test]
async fn every_example_request_renders() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    render(&app, "sample", &request("sample.json")).await;
    render(&app, "packages-demo", &request("packages-demo.json")).await;
    let mut invoices = fs::read_dir(examples().join("requests/progress-invoice"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    invoices.sort();
    assert_eq!(invoices.len(), 4);
    for path in invoices {
        let body: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let pdf = render(&app, "progress-invoice", &body).await;
        assert!(pdf.starts_with(b"%PDF"), "{}", path.display());
    }
}

/// SC-009: the first claim is checked to the cent against an independent computation.
#[tokio::test]
async fn first_claim_totals_are_exact() {
    let body = request("progress-invoice/01-first-claim.json");
    let data = &body["data"];

    // Independent computation, in integer cents (quantities are integers in this example).
    let (mut contract, mut period) = (0i128, 0i128);
    for lot in data["lots"].as_array().unwrap() {
        for line in lot["lines"].as_array().unwrap() {
            let quantity: i128 = line["quantity"].as_str().unwrap().parse().unwrap();
            let market = quantity * to_cents(line["unitPrice"].as_str().unwrap());
            let progress = line["progress"].as_i64().unwrap() as i128;
            let previous = line["previousProgress"].as_i64().unwrap_or(0) as i128;
            contract += market;
            period += percent_of(market, progress) - percent_of(market, previous);
        }
    }
    let vat = percent_of(period, 20);
    let gross = period + vat;
    let retention = percent_of(gross, 5);
    let due = gross - retention;
    assert_eq!(
        (contract, period, vat, gross, retention, due),
        (2_250_000, 750_000, 150_000, 900_000, 45_000, 855_000)
    );

    let volume = volume();
    let app = test_app(test_config(&volume));
    let text = compact_text(&render(&app, "progress-invoice", &body).await);
    for amount in [contract, period, vat, gross, retention, due] {
        assert!(
            text.contains(&euros(amount)),
            "{} missing in:\n{text}",
            euros(amount)
        );
    }
    assert!(text.contains("huitmillecinqcentcinquanteeuros"), "{text}");
}

#[tokio::test]
async fn long_claim_spans_at_least_three_pages() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let pdf = render(
        &app,
        "progress-invoice",
        &request("progress-invoice/02-long-claim-all-conditions.json"),
    )
    .await;
    assert!(page_count(&pdf) >= 3, "{} pages", page_count(&pdf));
    let text = compact_text(&pdf);
    for label in [
        "Révisiondeprix",
        "Compteprorata",
        "Remboursementdel",
        "Retenuedegarantie",
        "Avenants",
    ] {
        assert!(text.contains(label), "{label} missing");
    }
}

#[tokio::test]
async fn subcontractor_claim_uses_reverse_charge() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let pdf = render(
        &app,
        "progress-invoice",
        &request("progress-invoice/03-subcontractor-reverse-charge.json"),
    )
    .await;
    let text = compact_text(&pdf);
    assert!(text.contains("Autoliquidation"), "{text}");
    assert!(!text.contains("TVA20"), "no VAT line expected:\n{text}");
    assert!(
        !text.contains("Retenuedegarantie("),
        "bank guarantee replaces the retention:\n{text}"
    );
}

#[tokio::test]
async fn several_vat_rates_are_broken_down() {
    let volume = volume();
    let app = test_app(test_config(&volume));
    let pdf = render(
        &app,
        "progress-invoice",
        &request("progress-invoice/04-final-claim-multiple-vat-rates.json"),
    )
    .await;
    let text = compact_text(&pdf);
    for rate in ["TVA20%", "TVA10%", "TVA5,5%"] {
        assert!(text.contains(rate), "{rate} missing:\n{text}");
    }
    assert!(text.contains("Situationdesolde"), "{text}");
}

/// All `.bru` files of the Bruno collection, recursively.
fn bruno_requests() -> Vec<PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().unwrap() != "environments" {
                    walk(&path, out);
                }
            } else if path.extension().is_some_and(|e| e == "bru")
                && !["folder.bru", "collection.bru"]
                    .contains(&path.file_name().unwrap().to_str().unwrap())
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&examples().join("bruno"), &mut out);
    out.sort();
    out
}

/// Content of a top-level `name { … }` block, without its two-space indentation.
fn bru_block(text: &str, name: &str) -> Option<String> {
    let start = text
        .find(&format!("\n{name} {{\n"))
        .or_else(|| text.starts_with(&format!("{name} {{\n")).then_some(0))?;
    let body = &text[start..];
    let body = &body[body.find('\n').unwrap() + 1..];
    let body = &body[body.find(" {\n").map_or(0, |i| i + 3)..];
    let end = body.find("\n}\n")?;
    Some(
        body[..end]
            .lines()
            .map(|l| l.strip_prefix("  ").unwrap_or(l))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// FR-024: the Bruno collection only targets existing routes, and its bodies are the example
/// requests (no silent drift).
#[test]
fn bruno_collection_matches_the_api_and_the_examples() {
    let openapi: Value =
        serde_json::from_slice(&fs::read(repo_root().join("openapi/openapi.json")).unwrap())
            .unwrap();
    let paths = openapi["paths"].as_object().unwrap();
    let matches = |template: &str, path: &str| {
        let (t, p): (Vec<&str>, Vec<&str>) =
            (template.split('/').collect(), path.split('/').collect());
        t.len() == p.len() && t.iter().zip(&p).all(|(t, p)| t.starts_with('{') || t == p)
    };

    let requests = bruno_requests();
    assert!(requests.len() >= 15, "{} requests", requests.len());
    let mut covered = std::collections::BTreeSet::new();
    for file in &requests {
        let text = fs::read_to_string(file).unwrap();
        let (method, block) = ["get", "post"]
            .iter()
            .find_map(|m| bru_block(&text, m).map(|b| (*m, b)))
            .unwrap_or_else(|| panic!("{}: no request block", file.display()));
        let url = block.lines().find_map(|l| l.strip_prefix("url: ")).unwrap();
        let path = url
            .strip_prefix("{{baseUrl}}")
            .unwrap()
            .split('?')
            .next()
            .unwrap();
        let (template, item) = paths
            .iter()
            .find(|(template, _)| matches(template, path))
            .unwrap_or_else(|| panic!("{}: unknown route {path}", file.display()));
        assert!(
            item.get(method).is_some(),
            "{}: {method} {template}",
            file.display()
        );
        covered.insert(template.clone());

        if let Some(source) = bru_block(&text, "docs").and_then(|d| {
            d.lines()
                .find_map(|l| l.strip_prefix("Source: ").map(str::to_owned))
        }) {
            let body: Value =
                serde_json::from_str(&bru_block(&text, "body:json").unwrap()).unwrap();
            let expected: Value =
                serde_json::from_slice(&fs::read(repo_root().join(&source)).unwrap()).unwrap();
            assert_eq!(body, expected, "{} differs from {source}", file.display());
        }
    }
    let all: std::collections::BTreeSet<String> = paths.keys().cloned().collect();
    let missing: Vec<_> = all.difference(&covered).collect();
    assert!(
        missing.is_empty(),
        "routes without a Bruno request: {missing:?}"
    );
}
