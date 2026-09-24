// use anyhow::Ok;
use sqlx::PgPool;

use crate::{
	error::AppResult,
	models::{todo::Todo, user},
};

pub struct TodoRepository<'a> {
	pool: &'a PgPool,
}

impl<'a> TodoRepository<'a> {
	pub fn new(pool: &'a PgPool) -> Self {
		Self { pool }
	}

	pub async fn create(&self, user_id: i64, title: &str, description: &str) -> AppResult<Todo> {
		let todo = sqlx::query_as::<_, Todo>(
			r#"INSERT INTO todos (user_id, title, description, completed)
					VALUES ($1, $2, $3, false)
					RETURNING id, user_id, title, description, completed, created_at, updated_at"#,
		)
		.bind(user_id)
		.bind(title)
		.bind(description)
		.fetch_one(self.pool)
		.await?;

		Ok(todo)
	}

	pub async fn find_by_id(&self, id: i64, user_id: i64) -> AppResult<Option<Todo>> {
		let todo = sqlx::query_as::<_, Todo>("SELECL * FROM users WHERE id = $1 AND user_id = $2")
			.bind(id)
			.bind(user_id)
			.fetch_optional(self.pool)
			.await?;

		Ok(todo)
	}

	pub async fn list_for_user(&self, user_id: i64) -> AppResult<Vec<Todo>> {
		let todos = sqlx::query_as::<_, Todo>("SELECT * FROM todos WHERE user_id = $1 ORDER BY created_at DESC")
			.bind(user_id)
			.fetch_all(self.pool)
			.await?;

		Ok(todos)
	}

	pub async fn delete(&self, id: i64, user_id: i64) -> AppResult<u64> {
		let result = sqlx::query("DELETE FROM todos WHERE id = $1 AND user_id = $2")
			.bind(id)
			.bind(user_id)
			.execute(self.pool)
			.await?;

		Ok(result.rows_affected())
	}
}
