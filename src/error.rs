// Error handling model that is straightforward: hnadlers return Result<T, E> where both T and E implement IntoResponse
use axum::{
	Json,
	http::StatusCode,
	response::{IntoResponse, Response},
};

#[derive(Debug, thiserror::Error)]
enum AppError {
	#[error("resource not found")]
	NotFound,

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
			AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
			AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
			AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
			AppError::Internal(msg) => {
				tracing::error!("Internal error: {msg}");
				(StatusCode::INTERNAL_SERVER_ERROR, "something went wrong".into())
			}
		};
		(status, Json(serde_json::json!({ "error": message }))).into_response()
	}
}
