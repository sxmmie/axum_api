use sqlx::PgPool;

use crate::{
	dto::todo_dto::{CreateTodoRequest, TodoResponse},
	error::AppResult,
	repositories::todo_repo::TodoRepository,
};

pub struct TodoService<'a> {
	repo: TodoRepository<'a>,
}

impl<'a> TodoService<'a> {
	pub fn new(pool: &'a PgPool) -> Self {
		Self { repo: TodoRepository::new(pool) }
	}

	pub async fn create(&self, user_id: i64, req: CreateTodoRequest) -> AppResult<TodoResponse> {
		let todo = self.repo.create(user_id, &req.title, &req.description).await?;

		Ok(todo.into())
	}
}
