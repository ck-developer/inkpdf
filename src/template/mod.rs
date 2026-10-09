//! Modèle d'un template : identifiant, manifeste, schéma.

pub mod id;
pub mod manifest;
pub mod schema;

pub use id::{TEMPLATE_ID_PATTERN, TemplateId};
pub use manifest::Manifest;
pub use schema::TemplateSchema;
