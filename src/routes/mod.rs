use axum::Router;

use crate::state::{self, SharedState};

mod auth;
mod health;
mod todos;
mod users;

pub fn all_routes() -> Router<SharedState> {
	Router::new()
		.merge(users::routes())
		.merge(todos::routes())
		.merge(auth::routes())
		// .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
		.with_state(state);
}
