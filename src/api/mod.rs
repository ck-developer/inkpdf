//! HTTP router and OpenAPI document.

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
        description = "Internal service (private VPC, no authentication). Templates are hot-loaded from a mounted volume; the caller sends only JSON (`data` + `layout`), validated against the template's JSON Schema before generation.",
        license(name = "MIT OR Apache-2.0", identifier = "MIT OR Apache-2.0"),
    ),
    servers((url = "http://localhost:3000")),
    tags((name = "templates"), (name = "render"), (name = "packages"), (name = "ops")),
    components(
        schemas(Problem, ErrorCode, Violation, Diagnostic, health::Health),
        responses(ProblemResponse),
    ),
)]
pub struct ApiDoc;

/// Shared error response (`#/components/responses/Problem`).
pub struct ProblemResponse;

impl<'r> utoipa::ToResponse<'r> for ProblemResponse {
    fn response() -> (&'r str, RefOr<openapi::response::Response>) {
        let content = ContentBuilder::new()
            .schema(Some(Ref::from_schema_name("Problem")))
            .build();
        let response = ResponseBuilder::new()
            .description("Error in RFC 9457 format.")
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

/// This document, in JSON format
#[utoipa::path(
    get,
    path = "/openapi.json",
    tag = "ops",
    operation_id = "openapi",
    responses((status = 200, description = "OpenAPI 3.1 document.", body = Object)),
)]
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi())
}

/// OpenAPI document generated from the code.
pub fn openapi() -> utoipa::openapi::OpenApi {
    static DOC: OnceLock<utoipa::openapi::OpenApi> = OnceLock::new();
    DOC.get_or_init(|| router().into_openapi()).clone()
}

/// Builds the complete HTTP application.
pub fn build_app(state: AppState) -> Router {
    let (router, api) = router().split_for_parts();
    router
        .merge(Scalar::with_url("/docs", api))
        // Limit enforced by the body extractor; the handler turns an overflow into a
        // problem+json `413 payload-too-large`.
        .layer(DefaultBodyLimit::max(state.config.max_body_bytes))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
