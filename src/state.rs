use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;

// Applications need shared state: database tools, configuration, caches. Axum's state extractor provides type-safe access to shared data.
pub struct AppState {
	pub db_pool: PgPool,
	pub config: Config,
}

pub type SharedState = Arc<AppState>;
