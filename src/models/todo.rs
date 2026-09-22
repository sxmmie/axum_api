use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct Todo {
	pub id: i64,
	pub user_id: i64,
	pub title: String,
	pub description: String,
	pub completed: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}
