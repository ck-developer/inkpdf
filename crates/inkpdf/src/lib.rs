//! inkpdf: a generic service that generates PDFs from Typst templates.
//!
//! The service is content-agnostic: it validates the `{ data, layout }` body against the
//! template's schema, then hands it to Typst without knowing anything about its meaning.

pub mod api;
pub mod config;
pub mod error;
pub mod packages;
pub mod registry;
pub mod render;
pub mod template;

use std::sync::Arc;

use tokio::sync::Semaphore;

pub use api::{ApiDoc, build_app, openapi};
pub use config::Config;
pub use registry::Registry;

/// State shared by the handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub registry: Arc<Registry>,
    /// Concurrent render slots.
    pub render_slots: Arc<Semaphore>,
    /// Cached live OpenAPI document.
    pub live_doc: Arc<api::live::LiveDoc>,
}

impl AppState {
    /// Creates the state; the registry stays empty until `scan_all` is called.
    pub fn new(config: Config) -> Self {
        let registry = Registry::new(config.templates_dir.clone(), config.max_template_bytes);
        Self {
            render_slots: Arc::new(Semaphore::new(config.max_concurrent_renders)),
            live_doc: Arc::default(),
            registry: Arc::new(registry),
            config: Arc::new(config),
        }
    }
}
