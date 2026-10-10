//! Template model: id, manifest, schema.

pub mod id;
pub mod imports;
pub mod manifest;
pub mod schema;

pub use id::{TEMPLATE_ID_PATTERN, TemplateId};
pub use manifest::Manifest;
pub use schema::TemplateSchema;
