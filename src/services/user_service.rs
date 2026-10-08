use argon2::{
	Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
	password_hash::{self, SaltString, rand_core::OsRng},
};
use jsonwebtoken::{EncodingKey, Header, encode};
use sqlx::PgPool;

use crate::{
	dto::user_dto::{AuthResponse, LoginRequest, Pagination, RegisterRequest, UpdateUserRequest, UserResponse},
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
		let password_hash = Self::hash_password(&req.password)?;
		let user = self.repo.create(&req.name, &req.email, &password_hash).await?;
		let token = self.issue_token(user.id)?;

		Ok(AuthResponse { token, user: user.into() })
	}

	pub async fn list(&self, page: Pagination) -> AppResult<Vec<UserResponse>> {
		self.repo.find_all(page.limit(), page.offset()).await
	}

	pub async fn login(&self, req: LoginRequest) -> AppResult<AuthResponse> {
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

	pub async fn get_user(&self, actor_id: i64, target_id: i64) -> AppResult<UserResponse> {
		if actor_id != target_id {
			return Err(AppError::Forbidden);
		}

		let user = self.repo.find_by_id(target_id).await?.ok_or(AppError::NotFound)?;

		return Ok(user.into());
	}

	pub async fn update(&self, actor_id: i64, target_id: i64, req: UpdateUserRequest) -> AppResult<UserResponse> {
		if actor_id != target_id {
			return Err(AppError::Forbidden);
		}
		if req.name.is_none() && req.email.is_none() && req.password.is_none() {
			return Err(AppError::Validation("at least one field must be provided".into()));
		}

		let password_hash = req.password.as_deref().map(Self::hash_password).transpose()?;
		let user = self
			.repo
			.update(target_id, req.name.as_deref(), req.email.as_deref(), password_hash.as_deref())
			.await?
			.ok_or(AppError::NotFound)?;

		Ok(user.into())
	}

	pub async fn delete(&self, user_id: i64, id: i64) -> AppResult<()> {
		if user_id != id {
			return Err(AppError::Forbidden);
		}

		if self.repo.delete(id).await? == 0 {
			return Err(AppError::NotFound);
		}

		Ok(())
	}

	// 	pub async fn get_profile(&self, user_id: i64) -> AppResult<User> {
	// 		let user = self.repo.find_by_id(user_id).await?.ok_or(AppError::NotFound)?;
	//
	// 		Ok(user)
	// 	}

	fn issue_token(&self, user_id: i64) -> AppResult<String> {
		let claims = Claims::new(user_id, ACCESS_TOKEN_TTL_SESSION);
		encode(&Header::default(), &claims, &EncodingKey::from_secret(self.jwt_secret.as_bytes())).map_err(|e| AppError::Internal(anyhow::anyhow!("token signing failed: {e}")))
	}

	fn hash_password(password: &str) -> AppResult<String> {
		let salt = SaltString::generate(&mut OsRng);
		Argon2::default()
			.hash_password(password.as_bytes(), &salt)
			.map_err(|e| AppError::Internal(anyhow::anyhow!("passwrod hashing failed: {e}")))
			.map(|h| h.to_string())
	}
}
