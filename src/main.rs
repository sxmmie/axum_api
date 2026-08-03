use axum::{Router, routing::get};
use tokio::signal;

#[tokio::main]
async fn main() {
	let app = Router::new().route("/", get(|| async { "Hello, world!" }));

	// HTTP server setup
	let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
	println!("Listening on 0.0.0.0:3000");
	axum::serve(listener, app).await.unwrap()
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
