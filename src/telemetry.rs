use opentelemetry_sdk::trace::{Config, Sampler};
use tracing_subscriber::EnvFilter;

use crate::config::Config as AppConfig;

pub fn init(config: &AppConfig) {
	let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("axum_api=info,tower_http=info"));
	let fmt_lalyer = tracing_subscriber::fmt::layer().with_target(false);

	match &config.otlp_endpoint {
		Some(endpoint) => {
			let provider = opentelemetry_otlp::new_pipeline()
				.tracing()
				.with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint))
				.with_trace_config(Config::default().with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(config.trace_sample_ratio)))))
				.install_batch(opentelemetry_sdk::runtime::Tokio)
				.expect("failed to install OTLP tracer");
		}
	}
}
