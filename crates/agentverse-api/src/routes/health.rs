use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::state::AppState;

/// GET /health — liveness probe (always 200 while the process is alive)
pub async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

/// GET /ready — readiness probe; returns 503 when the DB is unreachable.
///
/// We use a cheap `find_by_id` with a nil UUID as a lightweight connectivity
/// probe. The query always returns `None` but exercises the full DB path.
pub async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    let db_ok = state.users.find_by_id(Uuid::nil()).await.is_ok();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(serde_json::json!({
            "status": if db_ok { "ready" } else { "not_ready" },
            "db": if db_ok { "ok" } else { "unreachable" },
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}
