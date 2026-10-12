//! Template discovery: list, detail, schema.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use utoipa::{IntoParams, ToSchema};

use crate::AppState;
use crate::api::render::form_urlencoded_pairs;
use crate::error::{ApiError, TemplateError};
use crate::registry::{TemplateEntry, TemplateStatus, TemplateVersions, VersionSelector};
use crate::template::TemplateId;

/// `templateId` path parameter; a malformed identifier is treated as unknown.
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub struct TemplatePath {
    /// Template identifier (its folder name).
    #[serde(rename = "templateId")]
    #[param(
        rename = "templateId",
        pattern = "^[a-z0-9][a-z0-9_-]{0,63}$",
        example = "sample"
    )]
    pub template_id: String,
}

/// `version` query parameter of the template operations (research R9 of feature 005).
#[derive(Debug, Default, PartialEq, Eq, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct VersionQuery {
    /// Template version: a major version (`2`, the highest valid `2.x.y`) or an exact version
    /// (`2.0.0`). Default: the highest valid version.
    #[param(value_type = Option<String>, pattern = r"^(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*))?$", example = "2.0.0")]
    pub version: Option<VersionSelector>,
}

impl VersionQuery {
    fn parse(query: Option<&str>) -> Result<Self, ApiError> {
        let mut parsed = Self::default();
        for (key, value) in query.map(form_urlencoded_pairs).unwrap_or_default() {
            if key == "version" {
                parsed.version = Some(parse_version(&value)?);
            }
        }
        Ok(parsed)
    }
}

/// Rule of the `version` parameter, shared with the MCP tools: a major version or an exact
/// version.
pub(crate) fn parse_version(value: &str) -> Result<VersionSelector, ApiError> {
    value.parse().map_err(
        |e: crate::registry::version::NotASelector| ApiError::InvalidParameter {
            name: "version",
            message: format!("{e}, got `{value}`"),
        },
    )
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
    /// The version described: in the list, the default version (the highest valid one); in a
    /// template's detail, the version selected. Absent when the template has no version.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false, example = "2.0.0")]
    pub version: Option<String>,
    pub status: TemplateStatusDto,
    /// Present when `status` is `invalid`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub reason: Option<String>,
    /// Every version of the template, newest first, valid or not. Empty when the template has
    /// no version.
    pub versions: Vec<TemplateVersionSummary>,
}

/// One version of a template.
#[derive(Debug, Serialize, ToSchema)]
pub struct TemplateVersionSummary {
    #[schema(example = "2.0.0")]
    pub version: String,
    pub status: TemplateStatusDto,
    /// Present when `status` is `invalid`.
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
    /// Input JSON Schema (absent when the template is invalid).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>, nullable = false)]
    pub schema: Option<serde_json::Value>,
    /// A complete, valid request body to start from (`data`, and optionally `layout` and
    /// `metadata`), as the template's `example.json` gives it. Absent when the template ships
    /// none, or when it does not pass the template's schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>, nullable = false)]
    pub example: Option<serde_json::Value>,
    pub links: TemplateLinks,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TemplateList {
    pub templates: Vec<TemplateSummary>,
}

fn status(entry: &TemplateEntry) -> (TemplateStatusDto, Option<String>) {
    match &entry.status {
        TemplateStatus::Valid => (TemplateStatusDto::Valid, None),
        TemplateStatus::Invalid { reason } => (TemplateStatusDto::Invalid, Some(reason.clone())),
    }
}

impl TemplateSummary {
    /// The summary of `entry`, one of `template`'s versions.
    pub fn new(template: &TemplateVersions, entry: &TemplateEntry) -> Self {
        let (status, reason) = status(entry);
        let versions = template
            .versions()
            .into_iter()
            .filter_map(|version| {
                let (status, reason) = self::status(version);
                Some(TemplateVersionSummary {
                    version: version.version.clone()?,
                    status,
                    reason,
                })
            })
            .collect();
        Self {
            id: entry.id.to_string(),
            name: entry.name.clone(),
            description: entry.description.clone(),
            version: entry.version.clone(),
            status,
            reason,
            versions,
        }
    }
}

impl TemplateDetail {
    /// The detail of `entry`, one of `template`'s versions.
    pub fn new(template: &TemplateVersions, entry: &TemplateEntry) -> Self {
        Self {
            summary: TemplateSummary::new(template, entry),
            loaded_at: entry.loaded_at.format(&Rfc3339).unwrap_or_default(),
            schema: entry.schema.as_ref().map(|s| s.value().clone()),
            example: entry.example.clone(),
            links: TemplateLinks {
                schema: format!("/templates/{}/schema", entry.id),
                render: format!("/templates/{}/render", entry.id),
            },
        }
    }
}

/// Looks a template up by its raw id; a malformed id is unknown and not echoed in the error.
pub(crate) fn find(
    state: &AppState,
    template_id: &str,
) -> Result<Arc<TemplateVersions>, TemplateError> {
    let Ok(id) = template_id.parse::<TemplateId>() else {
        return Err(ApiError::TemplateNotFound.into());
    };
    state
        .registry
        .get(&id)
        .ok_or_else(|| ApiError::TemplateNotFound.for_template(id.as_str()))
}

/// The version of `template` a caller asks for (research R9 of feature 005); `None` gives the
/// default version.
pub(crate) fn select(
    template: &TemplateVersions,
    selector: Option<VersionSelector>,
) -> Result<Arc<TemplateEntry>, TemplateError> {
    template.resolve(selector).cloned().map_err(|not_found| {
        let id = template.default_entry().id.to_string();
        ApiError::TemplateVersionNotFound {
            template_id: id.clone(),
            selector: selector.map(|s| s.to_string()).unwrap_or_default(),
            available: not_found.available,
        }
        .for_template(id)
    })
}

/// [`find`] then [`select`], with the `version` query parameter parsed in between: an unknown
/// template is reported before a malformed `version`.
fn find_version(
    state: &AppState,
    template_id: &str,
    query: Option<&str>,
) -> Result<(Arc<TemplateVersions>, Arc<TemplateEntry>), TemplateError> {
    let template = find(state, template_id)?;
    let query = VersionQuery::parse(query)
        .map_err(|e| e.for_template(template.default_entry().id.as_str()))?;
    let entry = select(&template, query.version)?;
    Ok((template, entry))
}

/// List the templates in the volume
#[utoipa::path(
    get,
    path = "/templates",
    tag = "templates",
    operation_id = "listTemplates",
    responses((status = 200, description = "Known templates, valid or invalid, sorted by identifier.", body = TemplateList)),
)]
pub async fn list_templates(State(state): State<AppState>) -> Json<TemplateList> {
    Json(template_list(&state))
}

/// Every known template, valid or invalid, sorted by id: the body of `GET /templates`.
pub(crate) fn template_list(state: &AppState) -> TemplateList {
    let templates = state
        .registry
        .list()
        .iter()
        .map(|template| TemplateSummary::new(template, template.default_entry()))
        .collect();
    TemplateList { templates }
}

/// Template detail, schema included
#[utoipa::path(
    get,
    path = "/templates/{templateId}",
    tag = "templates",
    operation_id = "getTemplate",
    params(TemplatePath, VersionQuery),
    responses(
        (status = 200, description = "Template detail, of the version selected.", body = TemplateDetail),
        (status = 400, response = crate::api::ProblemResponse),
        (status = 404, response = crate::api::ProblemResponse),
    ),
)]
pub async fn get_template(
    State(state): State<AppState>,
    Path(TemplatePath { template_id }): Path<TemplatePath>,
    RawQuery(query): RawQuery,
) -> Response {
    match find_version(&state, &template_id, query.as_deref()) {
        Ok((template, entry)) => Json(TemplateDetail::new(&template, &entry)).into_response(),
        Err(error) => error.into_response(),
    }
}

/// Raw JSON Schema of the template input
///
/// Usable directly by a validator or a form generator.
#[utoipa::path(
    get,
    path = "/templates/{templateId}/schema",
    tag = "templates",
    operation_id = "getTemplateSchema",
    params(TemplatePath, VersionQuery),
    responses(
        (status = 200, description = "JSON Schema (draft 2020-12).",
            content_type = "application/schema+json", body = Object),
        (status = 400, response = crate::api::ProblemResponse),
        (status = 404, response = crate::api::ProblemResponse),
        (status = 409, response = crate::api::ProblemResponse),
    ),
)]
pub async fn get_template_schema(
    State(state): State<AppState>,
    Path(TemplatePath { template_id }): Path<TemplatePath>,
    RawQuery(query): RawQuery,
) -> Response {
    let entry = match find_version(&state, &template_id, query.as_deref()) {
        Ok((_, entry)) => entry,
        Err(error) => return error.into_response(),
    };
    match (&entry.status, &entry.schema) {
        (TemplateStatus::Valid, Some(schema)) => {
            // The file's original bytes, exposed as is (constitution VI).
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
