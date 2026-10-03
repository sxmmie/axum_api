use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use validator::Validate;

use crate::{
	dto::user_dto::{AuthResponse, LoginRequest, RegisterRequest},
	error::{AppError, AppResult},
	services::user_service::UserService,
	state::SharedState,
};

pub fn routes() -> Router<SharedState> {
	Router::new().route("/auth/register", post(register)).route("/auth/login", post(login))
}

async fn register(State(state): State<SharedState>, Json(payload): Json<RegisterRequest>) -> AppResult<(StatusCode, Json<AuthResponse>)> {
	payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	let resp = service.register(payload).await?;

	Ok((StatusCode::CREATED, Json(resp)))
}

async fn login(State(state): State<SharedState>, Json(payload): Json<LoginRequest>) -> AppResult<Json<AuthResponse>> {
	payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	let resp = service.login(payload).await?;

	Ok(Json(resp))
}
