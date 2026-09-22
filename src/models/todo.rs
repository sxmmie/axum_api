use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct Todo {
	id: i64,
	user_id: i64,
	title: String,
	description: String,
	completed: bool,
	created_at: DateTime<Utc>,
	updated_at: DateTime<Utc>,
}
