use std::sync::Arc;

use axum::{Router, routing::get};
use sqlx::PgPool;
use tokio::signal;

mod config;
mod error;
mod state;

struct AppState {
	db_pool: PgPool,
	config: AppConfig,
}

#[tokio::main]
async fn main() {
	let state = Arc::new(AppState {
		db_pool: create_pool().await,
		config: load_config(),
	});

	let app = routes::all_routes().layer(TraceLayer::new_for_http().with_state(state));

	// HTTP server setup
	let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

	println!("Listening on 0.0.0.0:3000");

	axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await.unwrap()
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
