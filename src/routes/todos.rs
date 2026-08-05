pub fn router() -> Router<Arc<AppState>> {
	Router::new()
		.route("/todos", get(get_todo))
		.route("/todos", post(create_todo))
		.route("/todos/:id", put(update_todo))
		.route("/todos/:id", delete(delete_todo))
}
