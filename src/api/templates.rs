//! Découverte des templates : liste, détail, schéma.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use utoipa::{IntoParams, ToSchema};

use crate::AppState;
use crate::error::{ApiError, TemplateError};
use crate::registry::{TemplateEntry, TemplateStatus};
use crate::template::TemplateId;

/// Paramètre de chemin `templateId` ; un identifiant hors format est traité comme inconnu.
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub struct TemplatePath {
    /// Identifiant du template (nom de son dossier).
    #[serde(rename = "templateId")]
    #[param(
        rename = "templateId",
        pattern = "^[a-z0-9][a-z0-9_-]{0,63}$",
        example = "sample"
    )]
    pub template_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[schema(as = TemplateStatus)]
pub enum TemplateStatusDto {
    Valid,
    Invalid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TemplateSummary {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub version: Option<String>,
    pub status: TemplateStatusDto,
    /// Présent si `status` vaut `invalid`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TemplateLinks {
    #[schema(example = "/templates/sample/schema")]
    pub schema: String,
    #[schema(example = "/templates/sample/render")]
    pub render: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDetail {
    #[serde(flatten)]
    pub summary: TemplateSummary,
    #[schema(format = DateTime)]
    pub loaded_at: String,
    /// JSON Schema de l'entrée (absent si le template est invalide).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>, nullable = false)]
    pub schema: Option<serde_json::Value>,
    pub links: TemplateLinks,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TemplateList {
    pub templates: Vec<TemplateSummary>,
}

impl From<&TemplateEntry> for TemplateSummary {
    fn from(entry: &TemplateEntry) -> Self {
        let (status, reason) = match &entry.status {
            TemplateStatus::Valid => (TemplateStatusDto::Valid, None),
            TemplateStatus::Invalid { reason } => {
                (TemplateStatusDto::Invalid, Some(reason.clone()))
            }
        };
        Self {
            id: entry.id.to_string(),
            name: entry.name.clone(),
            description: entry.description.clone(),
            version: entry.version.clone(),
            status,
            reason,
        }
    }
}

impl From<&TemplateEntry> for TemplateDetail {
    fn from(entry: &TemplateEntry) -> Self {
        Self {
            summary: entry.into(),
            loaded_at: entry.loaded_at.format(&Rfc3339).unwrap_or_default(),
            schema: entry.schema.as_ref().map(|s| s.value().clone()),
            links: TemplateLinks {
                schema: format!("/templates/{}/schema", entry.id),
                render: format!("/templates/{}/render", entry.id),
            },
        }
    }
}

fn find(state: &AppState, template_id: &str) -> Result<Arc<TemplateEntry>, TemplateError> {
    let Ok(id) = template_id.parse::<TemplateId>() else {
        return Err(ApiError::TemplateNotFound.into());
    };
    state
        .registry
        .get(&id)
        .ok_or_else(|| ApiError::TemplateNotFound.for_template(id.as_str()))
}

/// Lister les templates du volume
#[utoipa::path(
    get,
    path = "/templates",
    tag = "templates",
    operation_id = "listTemplates",
    responses((status = 200, description = "Templates connus, valides ou invalides, triés par identifiant.", body = TemplateList)),
)]
pub async fn list_templates(State(state): State<AppState>) -> Json<TemplateList> {
    let templates = state
        .registry
        .list()
        .iter()
        .map(|entry| TemplateSummary::from(entry.as_ref()))
        .collect();
    Json(TemplateList { templates })
}

/// Détail d'un template, schéma compris
#[utoipa::path(
    get,
    path = "/templates/{templateId}",
    tag = "templates",
    operation_id = "getTemplate",
    params(TemplatePath),
    responses(
        (status = 200, description = "Détail du template.", body = TemplateDetail),
        (status = 404, response = crate::api::ProblemResponse),
    ),
)]
pub async fn get_template(
    State(state): State<AppState>,
    Path(TemplatePath { template_id }): Path<TemplatePath>,
) -> Response {
    match find(&state, &template_id) {
        Ok(entry) => Json(TemplateDetail::from(entry.as_ref())).into_response(),
        Err(error) => error.into_response(),
    }
}

/// JSON Schema brut de l'entrée du template
///
/// Utilisable directement par un validateur ou un générateur de formulaires.
#[utoipa::path(
    get,
    path = "/templates/{templateId}/schema",
    tag = "templates",
    operation_id = "getTemplateSchema",
    params(TemplatePath),
    responses(
        (status = 200, description = "JSON Schema (draft 2020-12).",
            content_type = "application/schema+json", body = Object),
        (status = 404, response = crate::api::ProblemResponse),
        (status = 409, response = crate::api::ProblemResponse),
    ),
)]
pub async fn get_template_schema(
    State(state): State<AppState>,
    Path(TemplatePath { template_id }): Path<TemplatePath>,
) -> Response {
    let entry = match find(&state, &template_id) {
        Ok(entry) => entry,
        Err(error) => return error.into_response(),
    };
    match (&entry.status, &entry.schema) {
        (TemplateStatus::Valid, Some(schema)) => {
            // Octets d'origine du fichier, exposés tels quels (constitution VI).
            let mut response = (StatusCode::OK, schema.raw().as_slice().to_vec()).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/schema+json"),
            );
            response
        }
        _ => ApiError::TemplateInvalid {
            reason: entry
                .invalid_reason()
                .unwrap_or("schema unavailable")
                .to_owned(),
        }
        .for_template(entry.id.as_str())
        .into_response(),
    }
}
