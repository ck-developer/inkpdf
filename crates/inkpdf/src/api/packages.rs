//! Typst packages made available to templates (specs/002-typst-packages, contracts/api.md).

use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::packages;

/// Package a template can import.
#[derive(Debug, Serialize, ToSchema)]
pub struct PackageInfo {
    /// Package name on Typst Universe.
    #[schema(pattern = "^[a-z0-9][a-z0-9-]*$", example = "zero")]
    pub name: String,
    /// Import line to write in a template (never with a version).
    #[schema(example = "@preview/zero")]
    pub import: String,
    /// Version installed in the service (informational only).
    #[schema(pattern = "^\\d+\\.\\d+\\.\\d+$", example = "0.7.1")]
    pub version: String,
    pub description: String,
    /// SPDX identifier or expression.
    #[schema(example = "MIT")]
    pub license: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PackageList {
    /// Sorted by name; one entry per package.
    pub packages: Vec<PackageInfo>,
}

/// Packages available to templates
///
/// Packages present only as dependencies of other packages are not listed: they
/// cannot be imported.
#[utoipa::path(
    get,
    path = "/packages",
    tag = "packages",
    operation_id = "listPackages",
    responses((status = 200, description = "Packages importable with `#import \"@preview/<name>\"`.", body = PackageList)),
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
