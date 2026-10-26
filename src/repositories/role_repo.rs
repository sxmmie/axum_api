pub struct RoleRepository<'a> {
	pool: &'a PgPool,
}

impl<'a> RoleRepository<'a> {
	pub fn new(pool: &'a PgPool) -> Self {
		Self { pool }
	}
}
