use opentelemetry::global;
use opentelemetry_sdk::{
	propagation::TraceContextPropagator,
	trace::{Config, Sampler},
};
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};

use crate::config::Config as AppConfig;

pub fn init(config: &AppConfig) {
	let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("axum_api=info,tower_http=info"));
	let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

	match &config.otlp_endpoint {
		Some(endpoint) => {
			let provider = opentelemetry_otlp::new_pipeline()
				.tracing()
				.with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint))
				.with_trace_config(Config::default().with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(config.trace_sample_ratio)))))
				.install_batch(opentelemetry_sdk::runtime::Tokio)
				.expect("failed to install OTLP tracer");

			global::set_tracer_provider(provider.clone());
			global::set_text_map_propagator(TraceContextPropagator::new());

			// ParentBased sampling so downstream services' traceparent decisions are honored instead of re-rolled per hop.
			let otel_layer = tracing_opentelemetry::layer().with_tracer(provider.tracer("axum_api"));
			Registry::default().with(env_filter).with(otel_layer).with(fmt_layer).init();
		}
		None => {
			Registry::default().with(env_filter).with(fmt_layer).init();
		}
	}
}

// The batch processor buffers spans in memory; exiting without flushing loses the last seconds of traces.
pub fn shudown() {
	global::shutdown_tracer_provider();
}
