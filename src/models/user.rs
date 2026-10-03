use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
	pub id: i64,
	pub name: String,
	pub email: String,
	pub password_hash: String,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}
