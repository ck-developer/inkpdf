//! Erreurs de l'API au format RFC 9457 (`application/problem+json`).

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Préfixe des URI `type` des erreurs.
pub const ERROR_TYPE_BASE: &str = "https://github.com/ck-developer/inkpdf/errors/";

/// Code machine d'une erreur (FR-019).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    TemplateNotFound,
    InvalidJson,
    UnsupportedMediaType,
    ValidationFailed,
    PayloadTooLarge,
    TemplateInvalid,
    RenderFailed,
    RenderTimeout,
    Overloaded,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TemplateNotFound => "template-not-found",
            Self::InvalidJson => "invalid-json",
            Self::UnsupportedMediaType => "unsupported-media-type",
            Self::ValidationFailed => "validation-failed",
            Self::PayloadTooLarge => "payload-too-large",
            Self::TemplateInvalid => "template-invalid",
            Self::RenderFailed => "render-failed",
            Self::RenderTimeout => "render-timeout",
            Self::Overloaded => "overloaded",
        }
    }

    pub fn status(self) -> StatusCode {
        match self {
            Self::TemplateNotFound => StatusCode::NOT_FOUND,
            Self::InvalidJson => StatusCode::BAD_REQUEST,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::ValidationFailed => StatusCode::UNPROCESSABLE_ENTITY,
            Self::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::TemplateInvalid => StatusCode::CONFLICT,
            Self::RenderFailed => StatusCode::INTERNAL_SERVER_ERROR,
            Self::RenderTimeout => StatusCode::GATEWAY_TIMEOUT,
            Self::Overloaded => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::TemplateNotFound => "Template not found",
            Self::InvalidJson => "Invalid JSON",
            Self::UnsupportedMediaType => "Unsupported media type",
            Self::ValidationFailed => "Validation failed",
            Self::PayloadTooLarge => "Payload too large",
            Self::TemplateInvalid => "Template invalid",
            Self::RenderFailed => "Render failed",
            Self::RenderTimeout => "Render timeout",
            Self::Overloaded => "Overloaded",
        }
    }
}

/// Violation du schéma d'un template par le corps d'une requête.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    /// JSON Pointer dans le corps de la requête.
    pub path: String,
    /// JSON Pointer dans le schéma.
    pub schema_path: String,
    pub message: String,
}

/// Diagnostic de compilation Typst.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct Diagnostic {
    pub message: String,
    /// Chemin relatif au dossier du template.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hints: Vec<String>,
}

/// Corps d'erreur RFC 9457.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    #[serde(rename = "type")]
    #[schema(format = "uri")]
    pub type_: String,
    pub title: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub detail: Option<String>,
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub template_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub violations: Option<Vec<Violation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub diagnostics: Option<Vec<Diagnostic>>,
}

/// Erreur renvoyée par un handler ; une variante par code du data-model.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("template not found")]
    TemplateNotFound,
    #[error("{message}")]
    InvalidJson { message: String },
    #[error("expected Content-Type: application/json")]
    UnsupportedMediaType,
    #[error("{} violation(s)", violations.len())]
    ValidationFailed { violations: Vec<Violation> },
    #[error("request body exceeds {limit} bytes")]
    PayloadTooLarge { limit: usize },
    #[error("{reason}")]
    TemplateInvalid { reason: String },
    #[error("template compilation failed")]
    RenderFailed { diagnostics: Vec<Diagnostic> },
    #[error("rendering exceeded the maximum duration")]
    RenderTimeout,
    #[error("no render slot available, retry later")]
    Overloaded,
}

impl ApiError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::TemplateNotFound => ErrorCode::TemplateNotFound,
            Self::InvalidJson { .. } => ErrorCode::InvalidJson,
            Self::UnsupportedMediaType => ErrorCode::UnsupportedMediaType,
            Self::ValidationFailed { .. } => ErrorCode::ValidationFailed,
            Self::PayloadTooLarge { .. } => ErrorCode::PayloadTooLarge,
            Self::TemplateInvalid { .. } => ErrorCode::TemplateInvalid,
            Self::RenderFailed { .. } => ErrorCode::RenderFailed,
            Self::RenderTimeout => ErrorCode::RenderTimeout,
            Self::Overloaded => ErrorCode::Overloaded,
        }
    }

    /// Associe l'identifiant du template concerné à l'erreur.
    pub fn for_template(self, template_id: impl Into<String>) -> TemplateError {
        TemplateError {
            error: self,
            template_id: Some(template_id.into()),
        }
    }

    pub fn into_problem(self, template_id: Option<String>) -> Problem {
        let code = self.code();
        let detail = Some(self.to_string());
        let (violations, diagnostics) = match self {
            Self::ValidationFailed { violations } => (Some(violations), None),
            Self::RenderFailed { diagnostics } => (None, Some(diagnostics)),
            _ => (None, None),
        };
        Problem {
            type_: format!("{ERROR_TYPE_BASE}{}", code.as_str()),
            title: code.title().to_string(),
            status: code.status().as_u16(),
            detail,
            code,
            template_id,
            violations,
            diagnostics,
        }
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_vec(&self).expect("Problem is always serializable");
        let mut response = (status, body).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        self.into_problem(None).into_response()
    }
}

/// [`ApiError`] accompagnée de l'identifiant du template concerné.
#[derive(Debug)]
pub struct TemplateError {
    pub error: ApiError,
    pub template_id: Option<String>,
}

impl From<ApiError> for TemplateError {
    fn from(error: ApiError) -> Self {
        Self {
            error,
            template_id: None,
        }
    }
}

impl IntoResponse for TemplateError {
    fn into_response(self) -> Response {
        self.error.into_problem(self.template_id).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<(ApiError, u16)> {
        vec![
            (ApiError::TemplateNotFound, 404),
            (
                ApiError::InvalidJson {
                    message: "x".into(),
                },
                400,
            ),
            (ApiError::UnsupportedMediaType, 415),
            (ApiError::ValidationFailed { violations: vec![] }, 422),
            (ApiError::PayloadTooLarge { limit: 1 }, 413),
            (ApiError::TemplateInvalid { reason: "r".into() }, 409),
            (
                ApiError::RenderFailed {
                    diagnostics: vec![],
                },
                500,
            ),
            (ApiError::RenderTimeout, 504),
            (ApiError::Overloaded, 503),
        ]
    }

    #[test]
    fn each_variant_has_its_status_and_problem_json_content_type() {
        for (error, status) in all() {
            let code = error.code();
            let response = error.into_response();
            assert_eq!(response.status().as_u16(), status, "{code:?}");
            assert_eq!(
                response.headers()[header::CONTENT_TYPE],
                "application/problem+json"
            );
        }
    }

    #[test]
    fn problem_fields_are_camel_case() {
        let problem = ApiError::ValidationFailed {
            violations: vec![Violation {
                path: "/data".into(),
                schema_path: "/properties/data/required".into(),
                message: "m".into(),
            }],
        }
        .into_problem(Some("sample".into()));
        let json = serde_json::to_value(&problem).unwrap();
        assert_eq!(
            json["type"],
            "https://github.com/ck-developer/inkpdf/errors/validation-failed"
        );
        assert_eq!(json["code"], "validation-failed");
        assert_eq!(json["status"], 422);
        assert_eq!(json["templateId"], "sample");
        assert_eq!(
            json["violations"][0]["schemaPath"],
            "/properties/data/required"
        );
        assert!(json.get("diagnostics").is_none());
    }
}
