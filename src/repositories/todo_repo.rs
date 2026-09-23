use sqlx::PgPool;

pub struct TodoRepository<'a> {
	pool: &'a PgPool,
}

impl<'a> TodoRepository<'a> {
	pub fn new(pool: &'a PgPool) -> Self {
		Self { pool }
	}
}
