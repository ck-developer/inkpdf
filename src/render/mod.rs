//! Pipeline de rendu : créneau, délai, compilation Typst, export PDF.

pub mod fonts;
pub mod value;
pub mod world;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use tokio::sync::OwnedSemaphorePermit;
use typst::diag::{SourceDiagnostic, Warned};
use typst::foundations::Smart;
use typst::{World, WorldExt};
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::AppState;
use crate::error::{ApiError, Diagnostic};
use crate::registry::TemplateEntry;
use world::SandboxWorld;

/// Profondeur conservée par le cache de compilation entre deux rendus.
const CACHE_MAX_AGE: usize = 10;

/// Génère le PDF d'un template à partir d'un corps déjà validé.
///
/// - attend un créneau au plus `queue_timeout` (sinon [`ApiError::Overloaded`]) ;
/// - répond au plus tard après `render_timeout` (sinon [`ApiError::RenderTimeout`]) et demande
///   alors l'annulation de la compilation, effective au prochain accès au monde ;
/// - le créneau n'est rendu qu'à la fin réelle du thread de compilation.
pub async fn render(
    entry: Arc<TemplateEntry>,
    body: serde_json::Value,
    state: &AppState,
) -> Result<Vec<u8>, ApiError> {
    let permit = match tokio::time::timeout(
        state.config.queue_timeout,
        state.render_slots.clone().acquire_owned(),
    )
    .await
    {
        Ok(Ok(permit)) => permit,
        _ => return Err(ApiError::Overloaded),
    };

    let cancel = Arc::new(AtomicBool::new(false));
    let task = {
        let cancel = cancel.clone();
        tokio::task::spawn_blocking(move || {
            let _guard = RenderGuard {
                _permit: permit,
                template_id: entry.id.to_string(),
                cancel: cancel.clone(),
                started: Instant::now(),
            };
            compile_pdf(entry, &body, cancel)
        })
    };

    match tokio::time::timeout(state.config.render_timeout, task).await {
        Ok(Ok(result)) => result,
        Ok(Err(join_error)) => Err(ApiError::RenderFailed {
            diagnostics: vec![Diagnostic {
                message: format!("internal error during rendering: {join_error}"),
                file: None,
                line: None,
                column: None,
                hints: Vec::new(),
            }],
        }),
        Err(_) => {
            cancel.store(true, Ordering::Relaxed);
            Err(ApiError::RenderTimeout)
        }
    }
}

/// Tient le créneau de rendu jusqu'à la fin réelle du thread de compilation, y compris quand
/// celle-ci se termine par une panique.
///
/// Après annulation, le monde renvoie des erreurs là où il servait des fichiers : en debug,
/// l'assertion de pureté de `comemo` transforme cela en panique du thread abandonné ; en
/// release, la compilation échoue normalement. Dans les deux cas, le créneau est rendu ici.
struct RenderGuard {
    _permit: OwnedSemaphorePermit,
    template_id: String,
    cancel: Arc<AtomicBool>,
    started: Instant,
}

impl Drop for RenderGuard {
    fn drop(&mut self) {
        if self.cancel.load(Ordering::Relaxed) {
            tracing::warn!(
                event = "render.overrun",
                templateId = %self.template_id,
                durationMs = self.started.elapsed().as_millis() as u64,
                "render finished after its deadline"
            );
        }
    }
}

/// Compilation et export, bloquants.
pub fn compile_pdf(
    entry: Arc<TemplateEntry>,
    body: &serde_json::Value,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<u8>, ApiError> {
    let ident = format!("{}@{}", entry.id, entry.fingerprint);
    let world = SandboxWorld::new(entry, body, cancel);

    let Warned { output, .. } = typst::compile::<PagedDocument>(&world);
    let result = output
        .and_then(|document| {
            let options = PdfOptions {
                ident: Smart::Custom(ident),
                timestamp: None,
                ..PdfOptions::default()
            };
            typst_pdf::pdf(&document, &options)
        })
        .map_err(|errors| ApiError::RenderFailed {
            diagnostics: errors.iter().map(|e| to_diagnostic(&world, e)).collect(),
        });

    comemo::evict(CACHE_MAX_AGE);
    result
}

fn to_diagnostic(world: &SandboxWorld, error: &SourceDiagnostic) -> Diagnostic {
    let mut diagnostic = Diagnostic {
        message: error.message.to_string(),
        file: None,
        line: None,
        column: None,
        hints: error.hints.iter().map(|h| h.v.to_string()).collect(),
    };
    if let Some(id) = error.span.id() {
        diagnostic.file = SandboxWorld::relative_path(id);
        let position = world.range(error.span).and_then(|range| {
            let source = world.source(id).ok()?;
            source.lines().byte_to_line_column(range.start)
        });
        if let Some((line, column)) = position {
            diagnostic.line = Some(line + 1);
            diagnostic.column = Some(column + 1);
        }
    }
    diagnostic
}
