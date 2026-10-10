//! inkpdf : service générique de génération de PDF à partir de templates Typst.
//!
//! Le service est agnostique du contenu : il valide le corps `{ data, layout }` contre le
//! schéma du template puis le transmet à Typst, sans rien connaître de sa signification.

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

/// État partagé par les handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub registry: Arc<Registry>,
    /// Créneaux de rendu simultanés.
    pub render_slots: Arc<Semaphore>,
}

impl AppState {
    /// Crée l'état ; le registre est vide tant que `scan_all` n'a pas été appelé.
    pub fn new(config: Config) -> Self {
        let registry = Registry::new(config.templates_dir.clone(), config.max_template_bytes);
        Self {
            render_slots: Arc::new(Semaphore::new(config.max_concurrent_renders)),
            registry: Arc::new(registry),
            config: Arc::new(config),
        }
    }
}
