use argon2::{
	Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
	password_hash::{SaltString, rand_core::OsRng},
};
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::PgPool;

use crate::{
	dto::user_dto::{AuthResponse, LoginRequest, RegisterRequest},
	error::{AppError, AppResult},
	models::user::User,
	repositories::user_repo::UserRepository,
	services::jwt::Claims,
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

	pub async fn login(&self, req: LoginRequest) -> AppResult<AuthResponse> {
		// let user = self.repo.find_by_email(&req.email).await?.ok_or(AppError::Unauthorized)?;
		let user = self
			.repo
			.find_by_email(&req.email)
			.await?
			// Deliberately the same error for "no such user" and "wrong password" below — don't let the API confirm which emails
			// are registered via response differences (user enumeration).
			.ok_or(AppError::Unauthorized)?;

		let parsed_hash = PasswordHash::new(&user.password_hash).map_err(|e| AppError::Internal(anyhow::anyhow!("stored hash unavailable: {e}")))?;

		Argon2::default()
			.verify_password(req.password.as_bytes(), &parsed_hash)
			.map_err(|_| AppError::Unauthorized)?;

		let token = self.issue_token(user.id)?;

		Ok(AuthResponse { token, user: user.into() })
	}

	pub async fn get_profile(&self, user_id: i64) -> AppResult<User> {
		let user = self.repo.find_by_id(user_id).await?.ok_or(AppError::NotFound)?;

		Ok(user.into())
	}

	fn issue_token(&self, user_id: i64) -> AppResult<String> {
		let claims = Claims::new(user_id, ACCESS_TOKEN_TTL_SESSION);
		encode(&Header::default(), &claims, &EncodingKey::from_secret(self.jwt_secret.as_bytes())).map_err(|e| AppError::Internal(anyhow::anyhow!("token signing failed: {e}")))
	}
}
