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

	pub async fn create_todo(&self, user_id: i64, req: CreateTodoRequest) -> AppResult<TodoResponse> {
		let todo = self.repo.create(user_id, &req.title, &req.description).await?;

		Ok(todo.into())
	}

	pub async fn get(&self, id: i64, user_id: i64) -> AppResult<TodoResponse> {
		let todo = self.repo.find_by_id(id, user_id).await?.ok_or(AppError::NotFound);

		Ok(todo.into())
	}

	pub async fn list(&self, user_id: i64) -> AppResult<Vec<TodoResponse>> {
		let todos = self.repo.list_for_user(user_id).await?;

		Ok(todos.into_iter().map(Into::into).collect())
	}
}
