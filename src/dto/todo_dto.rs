use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::todo::Todo;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTodoRequest {
	#[validate(length(min = 1, max = 200))]
	title: String,
	#[validate(length(max = 2000))]
	description: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTodoRequest {
	#[validate(length(min = 1, max = 200))]
	pub title: Option<String>,

	#[validate(length(max = 2000))]
	description: Option<String>,

	completed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TodoResponse {
	id: i64,
	title: String,
	description: String,
	completed: bool,
	created_at: DateTime<Utc>,
	updated_at: DateTime<Utc>,
}

impl From<Todo> for TodoResponse {
	fn from(t: Todo) -> Self {
		Self {
			id: t.id,
			title: t.title,
			description: t.description,
			completed: t.completed,
			created_at: t.created_at,
			updated_at: t.updated_at,
		}
	}
}
