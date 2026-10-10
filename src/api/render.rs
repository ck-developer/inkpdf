//! `POST /templates/{templateId}/render`.

use std::time::Instant;

use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::{Map, Value};
use utoipa::ToSchema;
use utoipa::openapi::RefOr;
use utoipa::openapi::schema::{ObjectBuilder, Schema, Type};

use crate::AppState;
use crate::api::templates::TemplatePath;
use crate::error::{ApiError, Problem};
use crate::registry::TemplateStatus;
use crate::render;
use crate::template::TemplateId;

/// Forme générique ; la forme exacte de `data` et `layout` est donnée par le schéma du template
/// (`GET /templates/{templateId}/schema`).
#[derive(Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct RenderRequest {
    /// Données métier.
    data: Map<String, Value>,
    /// Paramètres de mise en page ; les propriétés absentes prennent leur `default`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    layout: Option<Map<String, Value>>,
}

/// Document PDF renvoyé tel quel.
pub struct PdfDocument;

impl utoipa::PartialSchema for PdfDocument {
    fn schema() -> RefOr<Schema> {
        ObjectBuilder::new()
            .schema_type(Type::String)
            .content_media_type("application/pdf")
            .into()
    }
}

impl ToSchema for PdfDocument {}

/// Générer un PDF
///
/// Valide le corps contre le schéma du template (après application des valeurs `default` de
/// `layout`), puis renvoie le PDF. Aucune génération n'est tentée si la validation échoue.
#[utoipa::path(
    post,
    path = "/templates/{templateId}/render",
    tag = "render",
    operation_id = "renderTemplate",
    params(TemplatePath),
    request_body(content = RenderRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Document généré.", content_type = "application/pdf",
            body = inline(PdfDocument),
            headers(("Content-Disposition" = String, description = "inline; filename=\"<templateId>.pdf\""))),
        (status = 400, response = crate::api::ProblemResponse),
        (status = 404, response = crate::api::ProblemResponse),
        (status = 409, response = crate::api::ProblemResponse),
        (status = 413, response = crate::api::ProblemResponse),
        (status = 415, response = crate::api::ProblemResponse),
        (status = 422, description = "Corps non conforme au schéma du template.",
            content_type = "application/problem+json", body = Problem),
        (status = 500, response = crate::api::ProblemResponse),
        (status = 503, response = crate::api::ProblemResponse),
        (status = 504, response = crate::api::ProblemResponse),
    ),
)]
pub async fn render_template(
    State(state): State<AppState>,
    Path(TemplatePath { template_id }): Path<TemplatePath>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let Ok(id) = template_id.parse::<TemplateId>() else {
        return ApiError::TemplateNotFound.into_response();
    };
    let Some(entry) = state.registry.get(&id) else {
        return ApiError::TemplateNotFound
            .for_template(id.as_str())
            .into_response();
    };
    let fail = |error: ApiError| error.for_template(id.as_str()).into_response();

    if !is_json(&headers) {
        return fail(ApiError::UnsupportedMediaType);
    }
    let body = match body {
        Ok(body) => body,
        Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
            return fail(ApiError::PayloadTooLarge {
                limit: state.config.max_body_bytes,
            });
        }
        Err(rejection) => {
            return fail(ApiError::InvalidJson {
                message: rejection.body_text(),
            });
        }
    };
    let body: Value = match serde_json::from_slice(&body) {
        Ok(Value::Object(map)) => Value::Object(map),
        Ok(_) => {
            return fail(ApiError::InvalidJson {
                message: "the request body must be a JSON object".into(),
            });
        }
        Err(e) => {
            return fail(ApiError::InvalidJson {
                message: e.to_string(),
            });
        }
    };

    let schema = match (&entry.status, &entry.schema) {
        (TemplateStatus::Valid, Some(schema)) => schema,
        (TemplateStatus::Invalid { reason }, _) => {
            return fail(ApiError::TemplateInvalid {
                reason: reason.clone(),
            });
        }
        (TemplateStatus::Valid, None) => {
            return fail(ApiError::TemplateInvalid {
                reason: "schema unavailable".into(),
            });
        }
    };

    // Aucune donnée du corps n'est journalisée (FR-022).
    let started = Instant::now();
    let log = |outcome: &str| {
        tracing::info!(
            event = "render",
            templateId = %id,
            durationMs = started.elapsed().as_millis() as u64,
            outcome,
        );
    };

    let prepared = match schema.prepare(body) {
        Ok(prepared) => prepared,
        Err(violations) => {
            log("validation_failed");
            return fail(ApiError::ValidationFailed { violations });
        }
    };

    match render::render(entry.clone(), prepared, &state).await {
        Ok(pdf) => {
            log("ok");
            let disposition = format!("inline; filename=\"{id}.pdf\"");
            let mut response = (StatusCode::OK, pdf).into_response();
            let headers = response.headers_mut();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/pdf"),
            );
            if let Ok(value) = HeaderValue::from_str(&disposition) {
                headers.insert(header::CONTENT_DISPOSITION, value);
            }
            response
        }
        Err(error) => {
            log(match error {
                ApiError::RenderTimeout => "timeout",
                ApiError::Overloaded => "overloaded",
                _ => "render_failed",
            });
            fail(error)
        }
    }
}

/// `Content-Type: application/json`, paramètres (`charset`) tolérés.
fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
}
