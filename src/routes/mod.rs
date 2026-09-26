use axum::Router;

use crate::state::SharedState;

mod auth;
mod todos;
mod users;

pub fn all_routes() -> Router<SharedState> {
	Router::new().merge(users::routes()).merge(todos::routes()).merge(auth::routes())
}
