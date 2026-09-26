use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
	pub sub: String,
	pub exp: usize,
	pub iat: usize,
}

impl Claims {
	pub fn new(user_id: i64, ttl_seconds: i64) -> Self {
		let now = chrono::Utc::now();
		Self {
			sub: user_id.to_string(),
			exp: (now + chrono::Duration::seconds(ttl_seconds).timestamp() as usize),
			iat: now.timestamp() as usize,
		}
	}

	pub fn user_id(&self) -> Option<i64> {
		self.sub.parse().ok()
	}
}
