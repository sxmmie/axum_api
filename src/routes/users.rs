use axum::{Json, Router, extract::State, routing::get};
use sqlx::query::Query;

use crate::{
	dto::user_dto::{Pagination, UserResponse},
	error::AppResult,
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
	let user = service.get_profile(user_id).await?;

	Ok(Json(user.into()))
}
