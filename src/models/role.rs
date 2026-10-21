use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
	User,
	Admin,
}

impl Role {
	pub fn as_str(&self) -> &'static str {
		match self {
			Role::User => "user",
			Role::Admin => "admin",
		}
	}
}
