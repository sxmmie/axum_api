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
	pub fn from_env() -> Result<Self, ConfigError> {}
}
