use chrono::offset;
use sqlx::PgPool;

use crate::{
	dto::user_dto::UserResponse,
	error::{AppError, AppResult},
	models::user::User,
};

pub struct UserRepository<'a> {
	pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
	pub fn new(pool: &'a PgPool) -> Self {
		Self { pool }
	}

	// One mapping point for unique violations: create and update can both hit 23505 on the lower(email) index
	pub async fn map_conflict<T>(result: Result<T, sqlx::Error>) -> AppResult<T> {
		match result {
			Ok(v) => Ok(v),
			Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => Err(AppError::Conflict("email already in use".into())),
			Err(e) => Err(AppError::Database(e)),
		}
	}

	pub async fn create(&self, name: &str, email: &str, password_hash: &str) -> AppResult<User> {
		let result =
			sqlx::query_as::<_, User>(r#"INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id, name, email, password_hash, created_at, updated_at"#)
				.bind(name)
				.bind(email)
				.bind(password_hash)
				.fetch_one(self.pool)
				.await;

		match result {
			Ok(user) => Ok(user),
			// Postgres unique_violation is SQLSTATE 23505. Map it to a domain-meaningful Conflict rather than letting a raw
			// sqlx::Error surface as a generic 500 — the caller (service layer) doesn't need to know it's a DB constraint at all.
			Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => Err(AppError::Conflict("email already in use".into())),
			Err(e) => Err(AppError::Database(e)),
		}
	}

	// Maps directly into UserResponse — password_hash never leaves the repo layer.
	pub async fn find_all(&self, limit: i64, offset: i64) -> AppResult<Vec<UserResponse>> {
		Ok(sqlx::query_as!(
			UserResponse,
			"SELECT id, name, email, created_at FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
			limit,
			offset
		)
		.fetch_all(self.pool)
		.await?)
	}

	pub async fn update(&self, id: i64, name: Option<&str>, email: Option<&str>, password_hash: Option<&str>) -> AppResult<Option<User>> {
		Self::map_conflict(
			sqlx::query!(
				User,
				r#"UPDATE users
    					SET name = COALESCE($2::text, name),
    						 email = COALESCE($3::text, email),
    						 password_hash = COALESCE($4::text, password_hash)
    				 WHERE id = $1
    				 RETURNING id, name, email, password_hash, created_at, updated_at"#,
				id,
				name,
				email,
				password_hash
			)
			.fetch_optional(self.pool)
			.await,
		)
	}

	pub async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
		let user = sqlx::query_as::<_, User>("SELECT id, name, email, password_hash, created_at, updated_at FROM users WHERE lower(email) = lower($1)")
			.bind(email)
			.fetch_optional(self.pool)
			.await?;

		Ok(user)
	}

	pub async fn find_by_id(&self, id: i64) -> AppResult<Option<User>> {
		let user = sqlx::query_as::<_, User>("SELECT id, name, email, password_hash, created_at, updated_at FROM users WHERE id = $1")
			.bind(id)
			.fetch_optional(self.pool)
			.await?;

		Ok(user)
	}

	pub async fn delete(&self, id: i64) -> AppResult<u64> {
		Ok(sqlx::query!("DELETE FROM users WHERE id = $1", id).execute(self.pool).await?.rows_affected())
	}
}
