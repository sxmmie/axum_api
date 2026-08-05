// Error handling model that is straightforward: hnadlers return Result<T, E> where both T and E implement IntoResponse

use axum::{http::{Response, StatusCode}, response::IntoResponse};

enum AppError {
	NotFound(String),
	BadRequest(String),
	Unauthorized,
	Internal(String),
}

impl IntoResponse for AppError {
	fn into_response(self) -> Response {
	    let (status, message) = match self {
					AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
		}
	};
}
