use axum::{
	Json, Router,
	extract::{Path, State},
	http::StatusCode,
	routing::get,
};
use validator::Validate;

use crate::{
	dto::todo_dto::CreateTodoRequest,
	error::AppResult,
	services::{self, todo_service::TodoService},
	state::SharedState,
};
use crate::{
	dto::todo_dto::{TodoResponse, UpdateTodoRequest},
	error::AppError,
};

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

async fn update_todo(State(state): State<SharedState>, AuthUser(user_id): AuthUser, Path(id): Path<i64>, Json(payload): Json<UpdateTodoRequest>) -> AppResult<Json<TodoResponse>> {
	payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

	let service = TodoService::new(&state.db_pool);
	let todo = service.update(id, user_id, payload).await?;

	Ok(Json(todo))
}

async fn delete_todo(State(state): State<SharedState>, AuthUser(user_id): AuthUser, Path(id): Path<i64>) -> AppResult<StatusCode> {
	let service = TodoService::new(&state.db_pool);
	service.delete(id, user_id).await?;

	Ok(StatusCode::NO_CONTENT)
}
