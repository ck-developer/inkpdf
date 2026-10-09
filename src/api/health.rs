//! Santé du processus et disponibilité du registre.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Starting,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Health {
    pub status: HealthStatus,
    /// Nombre de templates valides.
    pub templates: usize,
    pub version: String,
}

fn health_of(state: &AppState, status: HealthStatus) -> Health {
    Health {
        status,
        templates: state.registry.valid_count(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// Vivacité du processus
#[utoipa::path(
    get,
    path = "/health",
    tag = "ops",
    operation_id = "health",
    responses((status = 200, description = "Le processus répond.", body = Health)),
)]
pub async fn health(State(state): State<AppState>) -> Json<Health> {
    Json(health_of(&state, HealthStatus::Ok))
}

/// Disponibilité (scan initial du volume terminé)
#[utoipa::path(
    get,
    path = "/ready",
    tag = "ops",
    operation_id = "ready",
    responses(
        (status = 200, description = "Prêt.", body = Health),
        (status = 503, description = "Scan initial en cours.", body = Health),
    ),
)]
pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Health>) {
    if state.registry.is_ready() {
        (StatusCode::OK, Json(health_of(&state, HealthStatus::Ok)))
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(health_of(&state, HealthStatus::Starting)),
        )
    }
}
