use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde_json::json;

use crate::state::SharedState;

pub fn routes() -> Router<SharedState> {
	Router::new().route("/health", get(health))
}

async fn health(State(state): State<SharedState>) -> (StatusCode, Json<serde_json::Value>) {
	match sqlx::query("SELECT 1").execute(&state.db_pool).await {
		Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))),
		Err(e) => {
			tracing::error!(error = ?e, "health check: database unreachable");
			(StatusCode::SERVICE_UNAVAILABLE, Json(json!({ "status": "unhealthy", "reason": "database unreachable" })))
		}
	}
}
