use std::sync::Arc;

use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::signal;
use tower_http::trace::TraceLayer;

use crate::{config::Config, state::AppState};

mod config;
mod dto;
mod error;
mod models;
mod repositories;
mod routes;
mod services;
mod state;

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt::init();

	// from_env() returns Result<Config, ConfigError> — must be handled
	let config = Config::from_env().expect("failed to load config");

	let db_pool = create_pool(&config.database_url).await;
	let port = config.server_port;

	let state = Arc::new(AppState { db_pool, config });

	let app = routes::all_routes().with_state(state).layer(TraceLayer::new_for_http());

	// HTTP server setup
	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:3000")).await.unwrap();

	tracing::info!("Listening on 0.0.0.0:{port}");

	axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await.unwrap()
}

async fn create_pool(database_url: &str) -> PgPool {
	PgPoolOptions::new().max_connections(10).connect(database_url).await.expect("failed to connect to database")
}

// Adding graceful shutdown
async fn shutdown_signal() {
	let ctrl_c = async {
		signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
	};

	#[cfg(unix)]
	let terminate = async {
		signal::unix::signal(signal::unix::SignalKind::terminate())
			.expect("Failed to listen for SIGTERM (terminate signal)")
			.recv()
			.await
			.expect("Failed to receive terminate signal");
	};

	#[cfg(not(unix))]
	let terminate = std::future::pending::<()>();

	tokio::select! {
		_ = ctrl_c => {}
		_ = terminate => {},
	}

	tracing::info!("Shutting down geacefully...");
}
