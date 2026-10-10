use axum::{http::{HeaderMap, Request}, middleware::Next, response::Response};
use opentelemetry::{global, propagation::Extractor};

pub async fn otel_context(request: Request, next: Next) -> Response {
	let parent_cx = global::get_text_map_propagator(|prop| prop.extract(&Extractor::new(request.headers())
	let _cx_guard = parent_cx.attach();

	let method = request.method().clone();
	let route = request.uri().path().to_string();
	let span = tracing::info_span!("HTTP request", otel.kind = "server", method = %method, route = %route);
	let mut response = span.in_scope(|| next.run(request)).await;

	let mut headers = HeaderMap::new();
	global::get_text_map_propagator(|prop| prop.inject(&mut Inserter::new(&mut headers)));
	for (name, value) in headers {
	    if let Some(name) = name {
			response.headers_mut().insert(name, value);
		}
	}

	response
}
