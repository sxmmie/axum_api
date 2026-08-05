async fn timing_middleware(req: Request, next: Next) -> Response {
	let start = std::time::Instant::now();
	let method = req.method().clone();
	let url = req.uri().clone();

	let response = next.run(req).await;

	let duration = start.elapsed();
	tracing::info("{method} {uri} -> {} in {duration:?}", response.status());

	response
}
