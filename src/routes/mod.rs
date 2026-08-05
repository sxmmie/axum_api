pub fn all_routes() -> Router<Arc<AppState>> {
	Router::new().merge(users::router()).merge(todos::router()).merge(auth::router())
}
