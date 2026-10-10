//! Benchmark du pipeline de rendu (SC-001) : `examples/templates/sample` avec
//! `examples/requests/sample.json`. Mesure fine et comparable entre versions ; le seuil
//! bloquant (p95 < 200 ms) est vérifié par `tests/perf.rs`.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use criterion::{Criterion, criterion_group, criterion_main};
use inkpdf::registry::TemplateEntry;
use inkpdf::registry::loader::{self, LoadOutcome};
use inkpdf::render::compile_pdf;

fn render_sample(c: &mut Criterion) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let entry: Arc<TemplateEntry> = match loader::load(
        &root.join("examples/templates/sample"),
        "sample".parse().unwrap(),
        u64::MAX,
    ) {
        LoadOutcome::Loaded(entry) => Arc::from(entry),
        LoadOutcome::Unstable => panic!("sample template is changing"),
    };
    let request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("examples/requests/sample.json")).unwrap())
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
