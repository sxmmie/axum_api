// Error handling model that is straightforward: hnadlers return Result<T, E> where both T and E implement IntoResponse
use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
enum AppError {
	#[error("resource not found")]
	NotFound,

	#[error("validation error: {0}")]
	Validation(String),

	#[error("bad request")]
	BadRequest(String),

	#[error("unauthorized")]
	Unauthorized,

	#[error("forbidden")]
	Forbidden,

	#[error("conflict: {0}")]
	Conflict(String),

	#[error("database error")]
	Database(#[from] sqlx::Error),

	#[error("internal server error")]
	Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
		let (status, message) = match &self {
			AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
			AppError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
			AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
			AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
			AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
			AppError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
			AppError::Database(e) => {
				tracing::error!(error = ?e, "database error");
				(StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
			}
			AppError::Internal(msg) => {
				tracing::error!("Internal error: {msg}");
				(StatusCode::INTERNAL_SERVER_ERROR, "something went wrong".into())
			}
		};
		// (status, Json(serde_json::json!({ "error": message }))).into_response()
		(status, Json(json!({ "error": message }))).into_response()
	}
}

pub type AppResult<T> = Result<T, AppError>;
