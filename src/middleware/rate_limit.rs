use std::net::SocketAddr;

use axum::{
	body::Body,
	extract::{ConnectInfo, Request, State},
	http::{Response, StatusCode},
	middleware::Next,
	response::IntoResponse,
};
use serde_json::json;

use crate::state::SharedState;

const WINDOW_SECONDS: i64 = 60;
const MAX_REQUESTS_PER_WINDOW: i64 = 100;

/// Fixed-window counter per client IP, stored in Redis so the limit is shared correctly across however many API instances you run — an
/// in-process counter would let each replica grant its own 100/min, defeating the point under horizontal scaling.
///
/// Requires the router to be served with connect-info so ConnectInfo can be extracted — see the note in main.rs wiring below.
pub async fn rate_limit(State(state): State<SharedState>, ConnectInfo(addr): ConnectInfo<SocketAddr>, req: Request<Body>, next: Next) -> Response {
	let key = format!("rate_limit: {}", addr.ip());
	let mut conn = state.redis.clone();

	match conn.incr::<_, _, i64>(&key, 1).await {
		Ok(count) => {
			if count == 1 {
				// Only set expiry on the first request in a fresh window — an unconditional EXPIRE on every call would keep
				// sliding the window forward and never actually reset it.
				let _: Result<(), _> = conn.expire(&key, WINDOW_SECONDS).await;
			}

			if count > MAX_REQUESTS_PER_WINDOW {
				let body = json!({"error": "too many requests"});
				return (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
			}
		}
		Err(e) => {
			// Fail open: if Redis is down, don't take the whole API down with it — log loudly and let the request through unrated
			// rather than 500-ing (or wrongly 200-ing) every request.
			tracing::error!(error = ?e, "rate limiter: redis unavailable, failing open");
		}
	}

	next.run(req).await
}
