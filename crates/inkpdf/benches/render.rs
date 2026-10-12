//! Rendering pipeline benchmark (SC-001): the `sample` test template with its request, from
//! `tests/fixtures/`. Fine-grained and comparable across versions; the blocking threshold
//! (p95 < 200 ms) is checked by `tests/perf.rs`.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use criterion::{Criterion, criterion_group, criterion_main};
use inkpdf::registry::TemplateEntry;
use inkpdf::registry::loader::{self, LoadOutcome};
use inkpdf::render::compile_pdf;

fn render_sample(c: &mut Criterion) {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let entry: Arc<TemplateEntry> = match loader::load(
        &fixtures.join("templates/sample"),
        "sample".parse().unwrap(),
        u64::MAX,
    ) {
        LoadOutcome::Loaded(entry) => Arc::from(entry),
        LoadOutcome::Unstable => panic!("sample template is changing"),
    };
    let request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixtures.join("requests/sample.json")).unwrap())
            .unwrap();
    let body = entry.schema.as_ref().unwrap().prepare(request).unwrap();

    c.bench_function("render sample (20 rows)", |b| {
        b.iter(|| {
            compile_pdf(
                entry.clone(),
                &body,
                &Default::default(),
                "inkpdf",
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap()
        })
    });
}

criterion_group!(benches, render_sample);
criterion_main!(benches);
