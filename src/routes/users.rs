use axum::{Json, Router, extract::State, routing::get};

use crate::{dto::user_dto::UserResponse, error::AppResult, extractors::auth_user::AuthUser, services::user_service::UserService, state::SharedState};

pub fn routes() -> Router<SharedState> {
	Router::new().route("/users/me", get(me))
}

async fn me(State(state): State<SharedState>, AuthUser(user_id): AuthUser) -> AppResult<Json<UserResponse>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	let user = service.get_profile(user_id).await?;

	Ok(Json(user.into()))
}
