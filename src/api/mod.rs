//! Routeur HTTP et document OpenAPI.

pub mod health;
pub mod packages;
pub mod render;
pub mod templates;

use std::sync::OnceLock;

use axum::extract::DefaultBodyLimit;
use axum::{Json, Router};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa::openapi::{self, ContentBuilder, Ref, RefOr, ResponseBuilder};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_scalar::{Scalar, Servable};

use crate::AppState;
use crate::error::{Diagnostic, ErrorCode, Problem, Violation};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "inkpdf",
        version = env!("CARGO_PKG_VERSION"),
        description = "Service interne (VPC privé, sans authentification). Les templates sont lus à chaud depuis un volume monté ; l'appelant n'envoie que du JSON (`data` + `design`), validé contre le JSON Schema du template avant génération.",
        license(name = "TODO(LICENSE) — MIT OR Apache-2.0 proposé"),
    ),
    servers((url = "http://localhost:3000")),
    tags((name = "templates"), (name = "render"), (name = "packages"), (name = "ops")),
    components(
        schemas(Problem, ErrorCode, Violation, Diagnostic, health::Health),
        responses(ProblemResponse),
    ),
)]
pub struct ApiDoc;

/// Réponse d'erreur partagée (`#/components/responses/Problem`).
pub struct ProblemResponse;

impl<'r> utoipa::ToResponse<'r> for ProblemResponse {
    fn response() -> (&'r str, RefOr<openapi::response::Response>) {
        let content = ContentBuilder::new()
            .schema(Some(Ref::from_schema_name("Problem")))
            .build();
        let response = ResponseBuilder::new()
            .description("Erreur au format RFC 9457.")
            .content("application/problem+json", content)
            .build();
        ("Problem", response.into())
    }
}

fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(templates::list_templates))
        .routes(routes!(templates::get_template))
        .routes(routes!(templates::get_template_schema))
        .routes(routes!(render::render_template))
        .routes(routes!(packages::list_packages))
        .routes(routes!(health::health))
        .routes(routes!(health::ready))
        .routes(routes!(openapi_json))
}

/// Ce document, au format JSON
#[utoipa::path(
    get,
    path = "/openapi.json",
    tag = "ops",
    operation_id = "openapi",
    responses((status = 200, description = "Document OpenAPI 3.1.", body = Object)),
)]
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi())
}

/// Document OpenAPI généré depuis le code.
pub fn openapi() -> utoipa::openapi::OpenApi {
    static DOC: OnceLock<utoipa::openapi::OpenApi> = OnceLock::new();
    DOC.get_or_init(|| router().into_openapi()).clone()
}

/// Construit l'application HTTP complète.
pub fn build_app(state: AppState) -> Router {
    let (router, api) = router().split_for_parts();
    router
        .merge(Scalar::with_url("/docs", api))
        // Limite appliquée par l'extracteur du corps ; le dépassement est converti en
        // `413 payload-too-large` au format problem+json par le handler.
        .layer(DefaultBodyLimit::max(state.config.max_body_bytes))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
