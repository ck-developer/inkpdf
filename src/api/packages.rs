//! Paquets Typst mis à disposition des templates (specs/002-typst-packages, contracts/api.md).

use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::packages;

/// Paquet importable par un template.
#[derive(Debug, Serialize, ToSchema)]
pub struct PackageInfo {
    /// Nom du paquet sur Typst Universe.
    #[schema(pattern = "^[a-z0-9][a-z0-9-]*$", example = "zero")]
    pub name: String,
    /// Ligne d'import à écrire dans un template (jamais de version).
    #[schema(example = "@preview/zero")]
    pub import: String,
    /// Version installée dans le service (information seulement).
    #[schema(pattern = "^\\d+\\.\\d+\\.\\d+$", example = "0.7.1")]
    pub version: String,
    pub description: String,
    /// Identifiant ou expression SPDX.
    #[schema(example = "MIT")]
    pub license: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PackageList {
    /// Triés par nom ; une seule entrée par paquet.
    pub packages: Vec<PackageInfo>,
}

/// Paquets disponibles pour les templates
///
/// Les paquets présents seulement comme dépendances d'autres paquets ne sont pas listés : ils
/// ne sont pas importables.
#[utoipa::path(
    get,
    path = "/packages",
    tag = "packages",
    operation_id = "listPackages",
    responses((status = 200, description = "Paquets importables par `#import \"@preview/<nom>\"`.", body = PackageList)),
)]
pub async fn list_packages() -> Json<PackageList> {
    let packages = packages::all()
        .iter()
        .filter(|p| p.is_selected())
        .map(|p| PackageInfo {
            name: p.name().to_owned(),
            import: format!("@{}/{}", packages::NAMESPACE, p.name()),
            version: p.version().to_owned(),
            description: p.description().to_owned(),
            license: p.license().to_owned(),
        })
        .collect();
    Json(PackageList { packages })
}
