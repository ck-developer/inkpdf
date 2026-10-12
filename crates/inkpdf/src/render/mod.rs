//! Rendering pipeline: validation, slot, timeout, Typst compilation, PDF export.

pub mod fonts;
pub mod metadata;
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
use crate::registry::{TemplateEntry, TemplateStatus};
use metadata::DocumentMetadata;
use world::SandboxWorld;

/// Age kept by the compilation cache between two renders.
const CACHE_MAX_AGE: usize = 10;

/// Interface a render request came through; recorded in the `render` log event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interface {
    Http,
    Mcp,
}

impl Interface {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Mcp => "mcp",
        }
    }
}

/// Validate-and-render pipeline shared by the HTTP API and the MCP interface, from the
/// template the caller looked up and a parsed request body (`{ data, layout?, metadata? }`) to
/// the PDF.
///
/// In order: template status, `metadata` extraction, schema preparation (all violations
/// together), the `render` log event, then [`render`]. The caller attaches the template id to
/// the error ([`ApiError::for_template`]).
pub async fn run(
    state: &AppState,
    entry: Arc<TemplateEntry>,
    body: serde_json::Value,
    interface: Interface,
) -> Result<Vec<u8>, ApiError> {
    let schema = match (&entry.status, &entry.schema) {
        (TemplateStatus::Valid, Some(schema)) => schema,
        (TemplateStatus::Invalid { reason }, _) => {
            return Err(ApiError::TemplateInvalid {
                reason: reason.clone(),
            });
        }
        (TemplateStatus::Valid, None) => {
            return Err(ApiError::TemplateInvalid {
                reason: "schema unavailable".into(),
            });
        }
    };

    // No body data is logged (FR-022).
    let started = Instant::now();
    let log = |outcome: &str| {
        tracing::info!(
            event = "render",
            templateId = %entry.id,
            templateVersion = entry.version.as_deref(),
            interface = interface.as_str(),
            durationMs = started.elapsed().as_millis() as u64,
            outcome,
        );
    };

    // `metadata` is validated by the service's fixed schema, `data` and `layout` by the
    // template's; all violations are returned together.
    let mut body = body;
    let metadata = match body.as_object_mut().map(metadata::extract) {
        Some(Ok(metadata)) => Ok(metadata),
        Some(Err(violations)) => Err(violations),
        None => Ok(Default::default()),
    };
    let prepared = schema.prepare(body);
    let (prepared, metadata) = match (prepared, metadata) {
        (Ok(prepared), Ok(metadata)) => (prepared, metadata),
        (prepared, metadata) => {
            let mut violations = metadata.err().unwrap_or_default();
            violations.extend(prepared.err().unwrap_or_default());
            log("validation_failed");
            return Err(ApiError::ValidationFailed { violations });
        }
    };

    let result = render(entry.clone(), prepared, metadata, state).await;
    log(match &result {
        Ok(_) => "ok",
        Err(ApiError::RenderTimeout) => "timeout",
        Err(ApiError::Overloaded) => "overloaded",
        Err(_) => "render_failed",
    });
    result
}

/// Generates a template's PDF from an already validated body.
///
/// - waits at most `queue_timeout` for a slot (otherwise [`ApiError::Overloaded`]);
/// - responds at the latest after `render_timeout` (otherwise [`ApiError::RenderTimeout`]) and
///   then requests cancellation of the compilation, effective on the next world access;
/// - the slot is released only when the compilation thread actually finishes.
pub async fn render(
    entry: Arc<TemplateEntry>,
    body: serde_json::Value,
    metadata: DocumentMetadata,
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
    let default_author = state.config.default_author.clone();
    let task = {
        let cancel = cancel.clone();
        tokio::task::spawn_blocking(move || {
            let _guard = RenderGuard {
                _permit: permit,
                template_id: entry.id.to_string(),
                cancel: cancel.clone(),
                started: Instant::now(),
            };
            compile_pdf(entry, &body, &metadata, &default_author, cancel)
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

/// Holds the render slot until the compilation thread actually finishes, including when it
/// ends with a panic.
///
/// After cancellation, the world returns errors where it used to serve files: in debug,
/// `comemo`'s purity assertion turns this into a panic of the abandoned thread; in release,
/// the compilation fails normally. In both cases, the slot is released here.
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

/// Compilation and export, blocking.
pub fn compile_pdf(
    entry: Arc<TemplateEntry>,
    body: &serde_json::Value,
    metadata: &DocumentMetadata,
    default_author: &str,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<u8>, ApiError> {
    let ident = format!("{}@{}", entry.id, entry.fingerprint);
    let template_name = entry.name.clone();
    let world = SandboxWorld::new(entry, body, cancel);

    let Warned { output, .. } = typst::compile::<PagedDocument>(&world);
    let result = output
        .and_then(|mut document| {
            let defaults = metadata::Defaults {
                title: &template_name,
                author: default_author,
            };
            metadata::apply(document.info_mut(), metadata, &defaults);
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
