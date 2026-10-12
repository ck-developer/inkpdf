//! inkpdf: a generic service that generates PDFs from Typst templates.
//!
//! The service is content-agnostic: it validates the `{ data, layout }` body against the
//! template's schema, then hands it to Typst without knowing anything about its meaning.

pub mod api;
pub mod config;
pub mod error;
pub mod mcp;
pub mod packages;
pub mod registry;
pub mod render;
pub mod template;

use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub use api::{ApiDoc, build_app, openapi};
pub use config::Config;
pub use registry::Registry;

use registry::watcher::{self, WatcherHandle};

/// State shared by the handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub registry: Arc<Registry>,
    /// Concurrent render slots.
    pub render_slots: Arc<Semaphore>,
    /// Cached live OpenAPI document.
    pub live_doc: Arc<api::live::LiveDoc>,
    /// Cancelled once the server has shut down and drained its requests; stops the MCP
    /// service.
    pub shutdown: CancellationToken,
}

impl AppState {
    /// Creates the state; the registry stays empty until `scan_all` is called.
    pub fn new(config: Config) -> Self {
        let registry = Registry::new(config.templates_dir.clone(), config.max_template_bytes);
        Self {
            render_slots: Arc::new(Semaphore::new(config.max_concurrent_renders)),
            live_doc: Arc::default(),
            shutdown: CancellationToken::new(),
            registry: Arc::new(registry),
            config: Arc::new(config),
        }
    }
}

/// Starts what runs next to the server, once the initial scan is done (research R10 of feature
/// 005): with hot reload on (`INKPDF_HOT_RELOAD`), the volume watcher and its periodic rescan;
/// otherwise nothing, and the registry keeps what it loaded at startup.
///
/// The returned handle must be kept while the server runs: dropping it stops watching. Must
/// be called inside a tokio runtime.
pub fn spawn_background_tasks(state: &AppState) -> Option<WatcherHandle> {
    state
        .config
        .hot_reload
        .then(|| watcher::spawn(state.registry.clone(), state.config.rescan_interval))
}
