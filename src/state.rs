// Applications need shared state: database tools, configuration, caches. Axum's state extractor provides type-safe access to shared data.
use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;

pub struct AppState {
	pub db_pool: PgPool,
	pub config: Config,
	// ConnectionManager is cheap to clone (it's an Arc internally) and hhandles reconnection automatically, so storing it directly (rather
	// than behind another Arc/Mutex) is the idiomatic pattern here.
	pub redis: ConnectionManager,
}

pub type SharedState = Arc<AppState>;
