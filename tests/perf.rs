//! SC-001 : garde-fou de régression de latence. Exécuté en release par la CI :
//! `cargo test --release --test perf -- --ignored`. Document d'une page : un titre et un tableau de 20 lignes.

mod common;

use std::time::{Duration, Instant};

use axum::http::StatusCode;
use common::*;

const RENDERS: usize = 50;
const P95_BUDGET: Duration = Duration::from_millis(200);

#[tokio::test(flavor = "multi_thread")]
#[ignore = "performance test, run with `cargo test --release --test perf -- --ignored`"]
async fn sample_p95_is_below_200ms() {
    let volume = TestVolume::new();
    volume.copy_template(&sample_template(), "sample");
    let app = test_app(test_config(&volume));
    let body = sample_request();

    // Rendu de chauffe (polices, caches).
    let (status, _, _) = post_json(&app, "/templates/sample/render", &body).await;
    assert_eq!(status, StatusCode::OK);

    let mut durations = Vec::with_capacity(RENDERS);
    for i in 0..RENDERS {
        // Données différentes à chaque rendu : le cache de compilation ne peut pas servir le
        // document entier.
        let mut body = body.clone();
        body["data"]["title"] = format!("Exemple {i}").into();
        let started = Instant::now();
        let (status, _, _) = post_json(&app, "/templates/sample/render", &body).await;
        durations.push(started.elapsed());
        assert_eq!(status, StatusCode::OK);
    }
    durations.sort();
    let p95 = durations[(RENDERS * 95).div_ceil(100) - 1];
    println!("p50 = {:?}, p95 = {p95:?}", durations[RENDERS / 2]);
    assert!(p95 < P95_BUDGET, "p95 = {p95:?} exceeds {P95_BUDGET:?}");
}

/// SC-006 (002) : template utilisant des paquets (QR code, montant, graphique) sous 1 s au p95.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "performance test, run with `cargo test --release --test perf -- --ignored`"]
async fn packages_demo_p95_is_below_1s() {
    const BUDGET: Duration = Duration::from_secs(1);
    let volume = TestVolume::new();
    volume.copy_template(
        &repo_root().join("examples/templates/packages-demo"),
        "packages-demo",
    );
    let app = test_app(test_config(&volume));
    let body: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo_root().join("examples/requests/packages-demo.json")).unwrap(),
    )
    .unwrap();

    let (status, _, _) = post_json(&app, "/templates/packages-demo/render", &body).await;
    assert_eq!(status, StatusCode::OK);

    let mut durations = Vec::with_capacity(RENDERS);
    for i in 0..RENDERS {
        let mut body = body.clone();
        body["data"]["reference"] = format!("DEMO-2026-{i:04}").into();
        let started = Instant::now();
        let (status, _, _) = post_json(&app, "/templates/packages-demo/render", &body).await;
        durations.push(started.elapsed());
        assert_eq!(status, StatusCode::OK);
    }
    durations.sort();
    let p95 = durations[(RENDERS * 95).div_ceil(100) - 1];
    println!(
        "packages-demo: p50 = {:?}, p95 = {p95:?}",
        durations[RENDERS / 2]
    );
    assert!(p95 < BUDGET, "p95 = {p95:?} exceeds {BUDGET:?}");
}
