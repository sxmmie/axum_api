use axum::Router;

use crate::state::SharedState;

mod auth;
mod health;
mod todos;
mod users;

pub fn all_routes() -> Router<SharedState> {
	Router::new().merge(health::routes()).merge(auth::routes()).merge(users::routes()).merge(todos::routes())
	// .layer(middleware::from_fn_with_state(state.clone(), rate_limit))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn all_routes_builds() {
		// axum 0.8 validates path syntax (e.g. {id} params) when routes are registered,
		// so merely constructing the router catches malformed paths that would panic at boot.
		let _router: Router<SharedState> = all_routes();
	}
}
