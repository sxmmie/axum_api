use axum::{
	Json, Router,
	extract::{Path, Query, State},
	http::StatusCode,
	routing::get,
};

use validator::Validate;

use crate::{
	dto::user_dto::{Pagination, UpdateUserRequest, UserResponse},
	error::{AppError, AppResult},
	extractors::auth_user::AuthUser,
	services::user_service::UserService,
	state::SharedState,
};

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/users", get(list))
		.route("/users/me", get(me))
		.route("/users/{id}", get(show).put(update).delete(destroy))
}

async fn list(State(state): State<SharedState>, AuthUser(_): AuthUser, Query(page): Query<Pagination>) -> AppResult<Json<Vec<UserResponse>>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.list(page).await?))
}

async fn me(State(state): State<SharedState>, AuthUser(user_id): AuthUser) -> AppResult<Json<UserResponse>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.get_user(user_id, user_id).await?))
}

async fn show(State(state): State<SharedState>, AuthUser(actor_id): AuthUser, Path(id): Path<i64>) -> AppResult<Json<UserResponse>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.get_user(actor_id, id).await?))
}

async fn update(State(state): State<SharedState>, AuthUser(actor_id): AuthUser, Path(id): Path<i64>, Json(payload): Json<UpdateUserRequest>) -> AppResult<Json<UserResponse>> {
	payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.update(actor_id, id, payload).await?))
}

async fn destroy(State(state): State<SharedState>, AuthUser(user_id): AuthUser, Path(id): Path<i64>) -> AppResult<StatusCode> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	service.delete(user_id, id).await?;

	Ok(StatusCode::NO_CONTENT)
}
