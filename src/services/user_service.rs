use sqlx::PgPool;

use crate::repositories::user_repo::UserRepository;

const ACCESS_TOKEN_TTL_SESSION: i64 = 15 * 60;

pub struct UserService<'a> {
	repo: UserRepository<'a>,
	jwt_secret: &'a str,
}

impl<'a> UserService<'a> {
	pub fn new(pool: &'a PgPool, jwt_secret: &'a str) -> Self {
		Self {
			repo: UserRepository::new(pool),
			jwt_secret,
		}
	}
}
