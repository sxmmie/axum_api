use sqlx::PgPool;

use crate::{error::{AppError, AppResult}, models::user::{self, User}};

pub struct UserRepository {
	pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
	pub fn new(pool: &'a PgPool) -> Self {
		Self { pool }
	}

	pub async fn create(&self, name: &str, email: &str, password_hash: &str) -> AppResult<User> {
	    let result = sqlx::query_as<_, User>(r#"INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id, name, email, password_hash, created_at, updated_at"#, )
            .bind(name)
            .bind(email)
            .bind(password_hash)
            .fetch_one(self.pool)
            .await?;

		match result {
		    Ok(user) => Ok(user)
			// Postgres unique_violation is SQLSTATE 23505. Map it to a domain-meaningful Conflict rather than letting a raw
            // sqlx::Error surface as a generic 500 — the caller (service layer) doesn't need to know it's a DB constraint at all.
			Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
			    Err(AppError::Conflict("email already in use".into()))
			}
			Err(e) => Err(AppError::Database(e))
		}
	}

	pub async fn find_by_email(self, email: &str) -> AppResult<Option<User>> {
	    let user = sqlx::query_as::<_, User>("SELECT id, name, email, password_hash, created_at, updated_at FROM users WHERE lower(email) = lower($1)",)
            .bind(email)
            .fetch_optional(self.pool).await?;

		Ok(user)
	}

	pub async fn find_by_id(self, id: i64) ->  AppResult<Option<User>> {
	    let user = sqlx::query_as::<_, User>("SELECT id, name, email, password_hash, created_at, updated_at FROM users WHERE id = $1",)
			.bind(self)
			.fetch_optional(self.pool).await?;

		Ok(user);
	}
}
