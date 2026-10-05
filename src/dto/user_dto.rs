use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::user::User;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
	#[validate(length(min = 1, max = 100))]
	pub name: String,

	#[validate(email)]
	pub email: String,

	// Length only here; do not encode character-class rules ("must have
	// a symbol") — those don't meaningfully improve security and just
	// frustrate users. Length is the one dimension that matters for
	// brute-force resistance.
	#[validate(length(min = 8, max = 256))]
	pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
	#[validate(email)]
	pub email: String,

	#[validate(length(min = 1))]
	pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
	#[validate(length(min = 1, max = 100))]
	pub name: Option<String>,

	#[validate(email)]
	pub email: Option<String>,

	#[validate(length(min = 8, max = 256))]
	pub password: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
	pub id: i64,
	pub name: String,
	pub email: String,
	pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
	fn from(u: User) -> Self {
		Self {
			id: u.id,
			name: u.name,
			email: u.email,
			created_at: u.created_at,
		}
	}
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
	pub token: String,
	pub user: UserResponse,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
	pub limit: Option<i64>,
	pub offset: Option<i64>,
}

impl Pagination {
	// Clamped rather than validated: an oversized page is a server-protection issue, not a user error.
	pub fn limit(&self) -> i64 {
		self.limit.unwrap_or(20).clamp(1, 100)
	}

	pub fn offset(&self) -> i64 {
		self.offset.unwrap_or(0).max(0)
	}
}
