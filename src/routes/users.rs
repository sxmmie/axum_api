use axum::{
	Json, Router,
	extract::{Path, State},
	http::StatusCode,
	routing::get,
};
use sqlx::{query::Query, types::Json};
use validator::Validate;

use crate::{
	dto::user_dto::{Pagination, UpdateUserRequest, UserResponse},
	error::{AppError, AppResult},
	extractors::auth_user::AuthUser,
	service::{self, user_service::UserService},
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
	let user = service.get_profile(user_id).await?;

	Ok(Json(user.into()))
}

async fn show(State(state): State<SharedState>, AuthUser(user_id): AuthUser, Path(id): Path<i64>) -> AppResult<Json<UserResponse>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.get_user(user_id, id).await?));
}

async fn update(State(state): State<SharedState>, AuthUser(user_id): AuthUser, Path(id): Path<i64>, Json(payload): Json<UpdateUserRequest>) {
	payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.update_user(user_id, id, payload).await?)); // Ok(Json(service.update_user(actor_id, target_id, payload).await?));
}

async fn destroy(State(state): State<SharedState>, AuthUser(user_id): AuthUser, Path(id): Path<i64>) -> AppResult<StatusCode> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	service.delete(user_id, id).await?;

	Ok(StatusCode::NO_CONTENT)
}
