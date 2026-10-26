use std::collections::BTreeMap;

use crate::error::AppResult;

pub struct RoleRepository<'a> {
	pool: &'a PgPool,
}

impl<'a> RoleRepository<'a> {
	pub fn new(pool: &'a PgPool) -> Self {
		Self { pool }
	}

	// Runs on every authenticated request: two indexed lookups, single round-trip.
	pub async fn roles_for_user(&self, user_id: i64) -> AppResult<BTreeMap<Role>> {
		let rows = sqlx::query!(r#"SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id = r.id WHERE ur.user_id = $1"#, user_id)
			.fetch_all(self.pool)
			.await?;

		Ok(rows.iter().filter_map(|r| r.name.parse().ok()).collect())
	}
}
