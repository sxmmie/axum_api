use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use sqlx::PgPool;

use crate::{
	dto::user_dto::{AuthResponse, RegisterRequest},
	error::{AppError, AppResult},
	repositories::user_repo::UserRepository,
};

const ACCESS_TOKEN_TTL_SESSION: i64 = 15 * 60;

pub struct UserService<'a> {
	repo: UserRepository<'a>,
	jwt_secret: &'a str,
}

impl<'a> UserService<'a> {
	pub fn new(pool: &'a PgPool, jwt_secret: &'a str) -> Self {
		Self {
			repo: UserRepository::new(pool),
			jwt_secret,
		}
	}

	pub async fn register(&self, req: RegisterRequest) -> AppResult<AuthResponse> {
		let salt = SaltString::generate(&mut OsRng);
		let argon2 = Argon2::default();

		let password_hash = argon2
			.hash_password(req.password.as_bytes(), &salt)
			.map_err(|e| AppError::Internal(anyhow::anyhow!("password hashing failed: {e}")))? // Hashing itself only fails on malformed input params, not user-controllable data — treat as an internal error.
			.to_string();

		let user = self.repo.create(&req.name, &req.email, &password_hash).await?;
		let token = self.issue_token(user.id)?;

		Ok(AuthResponse { token, user: user.into() })
	}
}
