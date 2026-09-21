use std::env;

#[derive(Clone, Debug)]
pub struct Config {
	pub database_url: String,
	pub server_port: u16,
	pub jwt_secret: String,
	pub environment: Environment,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Environment {
	Development,
	Production,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
	#[error("missing required env var: {0}")]
	Missing(&'static str),
	#[error("invalid value env var: {0}")]
	Invalid(&'static str),
}

impl Config {
	pub fn from_env() -> Result<Self, ConfigError> {
		dotenvy::dotenv().ok(); // no-op if .env absent (e.g. prod)

		Ok(Config {
			database_url: env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?,
			server_port: env::var("PORT").unwrap_or_else(|_| "3000".into()).parse().map_err(|_| ConfigError::Invalid("PORT"))?,
			jwt_secret: env::var("JWT_SECRET").map_err(|_| ConfigError::Missing("JWT_SECRET"))?,
			environment: match env::var("APP_ENV").as_deref() {
				Ok("production") => Environment::Production,
				_ => Environment::Development,
			},
		})
	}
}
