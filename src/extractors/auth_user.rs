use axum::{
	extract::FromRequestParts,
	http::{StatusCode, header, request::Parts},
	response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde_json::json;
use sqlx::types::Json;

use crate::{services::jwt::Claims, state::SharedState};

/// Extracted from any handler argument as `AuthUser(user_id): AuthUser`.
/// Rejects the request with 401 before the handler body ever runs if the
/// token is missing, malformed, expired, or invalid.
pub struct AuthUser(pub i64);

pub enum AuthError {
	MissingToken,
	InvalidToken,
}

impl IntoResponse for AuthError {
	fn into_response(self) -> Response {
		let message = match self {
			AuthError::MissingToken => "missing or malformed authorization header",
			AuthError::InvalidToken => "invalid or expired token",
		};
		(StatusCode::UNAUTHORIZED, Json(json!({ "error": message }))).into_response()
	}
}

impl FromRequestParts<SharedState> for AuthUser {
	type Rejection = AuthError;

	async fn from_request_parts(parts: &mut Parts, state: &SharedState) -> Result<Self, Self::Rejection> {
		let header_value = parts.headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).ok_or(AuthError::MissingToken)?;

		let token = header_value.strip_prefix("Bearer ").ok_or(AuthError::MissingToken)?;

		let data = decode::<Claims>(token, &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()), &Validation::default()).map_err(|_| AuthError::InvalidToken);

		let user_id = data.claims.user_iid().ok_or(AuthError::InvalidToken)?;

		Ok(AuthUser(user_id))
	}
}
