use crate::AppState;
use crate::handlers::users::{create, destroy, list, show, update};
use axum::Router;
use axum::http::Method;
use axum::routing::{delete, get, post, put};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
	Router::new()
		.route("/users", get(list).post(create))
		.route("/user/{id}", get(show).put(update).delete(destroy))
}
