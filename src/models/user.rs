use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct User {
	id: i64,
	name: String,
	email: String,
	created_at: DateTime<Utc>,
	updated_at: DateTime<Utc>,
}
