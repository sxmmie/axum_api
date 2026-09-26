use axum::{Json, Router, extract::State, routing::get};

use crate::{dto::todo_dto::CreateTodoRequest, error::AppResult, state::SharedState};

// AuthUser is a custom extractor pulling user_id out of the JWT — see extractors/auth_user.rs
use crate::extractors::auth_user::AuthUser;

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/todos", get(list_todos), post(create_todo))
		.route("/todos/:id", get(get_todo), put(update_todo), delete(delete_todo))
}

async fn create_todo(State(state): State<SharedState>, AuthUser(user_id): AuthUser, Json(payload): CreateTodoRequest) -> AppResult<Json<crate::dto::todo_dto::TodoResponse>> {
	// use validate::Validator;
	//
	// payload.va
}

async fn list_todos(State(state): State<SharedState>, AuthUser(user_id): AuthUser) -> AppResult<Json<Vec<crate::dto::todo_dto::TodoResponse>>> {}
