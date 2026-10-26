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

	// Executor-generic: the same code runs against a pool (no tx) or a caller-owned Transaction. The CTE makes "user row + base role" atomic, so registration can't half-succeed.
	pub async fn create_user_with_roles<'e, E: Executor<'e, Database = Postgres>>(executor: E, name: &str, email: &str, password_hash: &str, roles: &[Role]) -> AppResult<i64> {
		let role_names: Vec<String> = roles.iter().map(|r| r.as_str().to_string()).collect();

		let row = sqlx::query!(
			r#"WITH inserted AS (
							INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id
						),
						grants AS (
							INSERT INTO user_roles (user_id, role_id)
							SELECT i.id, r.id FROM inserted i JOIN roles r ON r.name = ANY($4)
						)
						SELECT id FROM inserted"#,
			name,
			email,
			password_hash,
			&role_names
		)
		.fetch_all(self.pool)
		.await?;

		Ok(row.id)
	}
}
