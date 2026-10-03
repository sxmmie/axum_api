# Implementation Plan — axum_api

Status: compile errors already fixed in the working tree (11 errors + `todos.rs` path syntax + `main.rs` port binding + `all_routes_builds` router test). This document covers the remaining roadmap: bug fixes, migrations, full user CRUD with `query!` macros, OTel, and RBAC. Sections are ordered — each builds on the previous.

Conventions: rustfmt.toml uses tabs + `max_width = 180`; snippets below match that.

---

## 0. Prerequisites (one-time)

The `query!` macros (sections 2–5) verify queries against a **live schema at compile time**, so `DATABASE_URL` must point at a reachable Postgres for every `cargo check/build/test`. You have brew Postgres but nothing running:

```bash
/opt/homebrew/bin/initdb -D ~/pgdata-dev --auth=trust --no-locale -E UTF8
/opt/homebrew/bin/pg_ctl -D ~/pgdata-dev -l ~/pgdata-dev/log start
createdb axum_api_dev
export DATABASE_URL=postgres://localhost/axum_api_dev
cargo install sqlx-cli --no-default-features --features postgres
cargo sqlx migrate run          # applies migrations/ (after section 1 fix)
```

For CI without a database, generate offline metadata so the macros compile from committed files:

```bash
cargo sqlx prepare --check --workspace   # writes sqlx-data.json (sqlx 0.7) / .sqlx/ (sqlx 0.8)
```

Caveat to accept deliberately: `DATABASE_URL` becomes a build dependency for everyone on the team. The `--check` in CI is what prevents the metadata from silently drifting from the schema.

---

## 1. Bug fixes

### 1a. `src/repositories/todo_repo.rs` — wrong table + typo (live 500 today)

`find_by_id` runs `SELECL * FROM users WHERE id = $1 AND user_id = $2` — misspelled keyword, wrong table. Replace the method:

```rust
	pub async fn find_by_id(&self, id: i64, user_id: i64) -> AppResult<Option<Todo>> {
		let todo = sqlx::query_as::<_, Todo>("SELECT id, user_id, title, description, completed, created_at, updated_at FROM todos WHERE id = $1 AND user_id = $2")
			.bind(id)
			.bind(user_id)
			.fetch_optional(self.pool)
			.await?;

		Ok(todo)
	}
```

(Once section 2 lands, convert to the macro form — the macros are what would have caught this at compile time. See 2f.) Also delete the dead `// use anyhow::Ok;` on line 1.

### 1b. `migrations/00001_create_users.up.sql` — three problems vs the Rust code

- column is `passwordhash`; the Rust model + queries use `password_hash` → every user query fails at runtime
- extra `description TEXT NOT NULL` column that the code never inserts → registration violates NOT NULL
- missing `;` after `EXECUTE FUNCTION set_updated_at()` → migration fails to apply (verify yours; the current file lacks it)

Since this is a dev database that can be recreated, edit the file in place (replace `passwordhash` → `password_hash`, drop `description` from users) rather than writing an ALTER migration. Canonical content:

```sql
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
	NEW.updated_at = now();
	RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE users (
	id            BIGSERIAL PRIMARY KEY,
	name          TEXT        NOT NULL,
	email         TEXT        NOT NULL,
	password_hash TEXT        NOT NULL,
	created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
	updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Keep your existing decision here: lower(email) uniqueness is strictly better than plain UNIQUE
-- because it treats "Sam@x.com" and "sam@x.com" as one account, and find_by_email already does lower(email) = lower($1).
CREATE UNIQUE INDEX users_email_unique_idx ON users (lower(email));

CREATE TRIGGER users_set_updated_at
	BEFORE UPDATE ON users
	FOR EACH ROW
	EXECUTE FUNCTION set_updated_at();
```

`00002_create_todos.up.sql` is already correct (FK has `ON DELETE CASCADE`, which is what makes `DELETE /users/{id}` safe). Add the index:

```sql
CREATE INDEX idx_todos_user_created ON todos (user_id, created_at DESC);
```

Then reset and reapply:

```bash
sqlx database drop && sqlx database create && cargo sqlx migrate run
```

### 1c. Dead middleware files

`src/middleware/logging.rs` is empty; `timing.rs` has no imports and cannot compile. `TraceLayer` + the OTel span (section 3) cover both jobs. Delete both files; `src/middleware/mod.rs` becomes `pub mod trace;` (section 3).

---

## 2. Migrations runner + user CRUD with `query!` macros

### 2a. `src/main.rs` — apply migrations at boot

`migrate` is an sqlx default feature, so no Cargo.toml change needed. After `create_pool`:

```rust
	// Idempotent; applied versions are tracked in _sqlx_migrations. Fail fast, before accepting traffic.
	sqlx::migrate!().run(&db_pool).await.expect("migrations failed");
```

(`sqlx::migrate!()` embeds the `migrations/` dir at compile time; at runtime it applies any pending files. This is also why the SQL files must exist and be applied to the dev DB **before** the macros can compile — chicken/egg resolved by `cargo sqlx migrate run` in section 0.)

### 2b. `src/repositories/user_repo.rs` — full replacement

```rust
use sqlx::PgPool;

use crate::{
	dto::user_dto::UserResponse,
	error::{AppError, AppResult},
	models::user::User,
};

pub struct UserRepository<'a> {
	pool: &'a PgPool,
}

impl<'a> UserRepository<'a> {
	pub fn new(pool: &'a PgPool) -> Self {
		Self { pool }
	}

	// One mapping point for unique violations: create and update can both hit 23505 on the lower(email) index.
	fn map_conflict<T>(result: Result<T, sqlx::Error>) -> AppResult<T> {
		match result {
			Ok(v) => Ok(v),
			Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => Err(AppError::Conflict("email already in use".into())),
			Err(e) => Err(AppError::Database(e)),
		}
	}

	pub async fn create(&self, name: &str, email: &str, password_hash: &str) -> AppResult<User> {
		Self::map_conflict(
			sqlx::query_as!(
				User,
				r#"INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3) RETURNING id, name, email, password_hash, created_at, updated_at"#,
				name,
				email,
				password_hash
			)
			.fetch_one(self.pool)
			.await,
		)
	}

	pub async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
		Ok(sqlx::query_as!(User, "SELECT id, name, email, password_hash, created_at, updated_at FROM users WHERE lower(email) = lower($1)", email)
			.fetch_optional(self.pool)
			.await?)
	}

	pub async fn find_by_id(&self, id: i64) -> AppResult<Option<User>> {
		Ok(sqlx::query_as!(User, "SELECT id, name, email, password_hash, created_at, updated_at FROM users WHERE id = $1", id)
			.fetch_optional(self.pool)
			.await?)
	}

	// Maps directly into UserResponse — password_hash never leaves the repo layer.
	pub async fn list(&self, limit: i64, offset: i64) -> AppResult<Vec<UserResponse>> {
		Ok(sqlx::query_as!(UserResponse, "SELECT id, name, email, created_at FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2", limit, offset)
			.fetch_all(self.pool)
			.await?)
	}

	pub async fn update(&self, id: i64, name: Option<&str>, email: Option<&str>, password_hash: Option<&str>) -> AppResult<Option<User>> {
		Self::map_conflict(
			sqlx::query_as!(
				User,
				r#"UPDATE users
						SET name = COALESCE($2::text, name),
							 email = COALESCE($3::text, email),
							 password_hash = COALESCE($4::text, password_hash)
					 WHERE id = $1
					 RETURNING id, name, email, password_hash, created_at, updated_at"#,
				id,
				name,
				email,
				password_hash
			)
			.fetch_optional(self.pool)
			.await,
		)
	}

	pub async fn delete(&self, id: i64) -> AppResult<u64> {
		Ok(sqlx::query!("DELETE FROM users WHERE id = $1", id).execute(self.pool).await?.rows_affected())
	}
}
```

Macro notes:
- `query_as!` against a named struct requires **exact** field/column correspondence — this is the compile-time guarantee you're buying.
- The `$n::text` casts inside `COALESCE` are needed because the macro cannot infer a nullable bind's type through `COALESCE`.

### 2c. `src/dto/user_dto.rs` — append

```rust
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
	#[validate(length(min = 1, max = 100))]
	pub name: Option<String>,

	#[validate(email)]
	pub email: Option<String>,

	#[validate(length(min = 8, max = 256))]
	pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
	pub limit: Option<i64>,
	pub offset: Option<i64>,
}

impl Pagination {
	// Clamped rather than validated: an oversized page is a server-protection issue, not a user error.
	pub fn limit(&self) -> i64 {
		self.limit.unwrap_or(20).clamp(1, 100)
	}

	pub fn offset(&self) -> i64 {
		self.offset.unwrap_or(0).max(0)
	}
}
```

### 2d. `src/services/user_service.rs` — refactor hashing, add CRUD

Add `Pagination, UpdateUserRequest, UserResponse` to the `dto::user_dto` import.

```rust
	fn hash_password(password: &str) -> AppResult<String> {
		let salt = SaltString::generate(&mut OsRng);
		Argon2::default()
			.hash_password(password.as_bytes(), &salt)
			.map_err(|e| AppError::Internal(anyhow::anyhow!("password hashing failed: {e}")))
			.map(|h| h.to_string())
	}

	pub async fn register(&self, req: RegisterRequest) -> AppResult<AuthResponse> {
		let password_hash = Self::hash_password(&req.password)?;
		let user = self.repo.create(&req.name, &req.email, &password_hash).await?;
		let token = self.issue_token(user.id)?;

		Ok(AuthResponse { token, user: user.into() })
	}

	pub async fn list(&self, page: Pagination) -> AppResult<Vec<UserResponse>> {
		self.repo.list(page.limit(), page.offset()).await
	}

	pub async fn get_user(&self, actor_id: i64, target_id: i64) -> AppResult<UserResponse> {
		if actor_id != target_id {
			return Err(AppError::Forbidden);
		}
		let user = self.repo.find_by_id(target_id).await?.ok_or(AppError::NotFound)?;

		Ok(user.into())
	}

	pub async fn update(&self, actor_id: i64, target_id: i64, req: UpdateUserRequest) -> AppResult<UserResponse> {
		if actor_id != target_id {
			return Err(AppError::Forbidden);
		}
		if req.name.is_none() && req.email.is_none() && req.password.is_none() {
			return Err(AppError::Validation("at least one field must be provided".into()));
		}

		let password_hash = req.password.as_deref().map(Self::hash_password).transpose()?;
		let user = self
			.repo
			.update(target_id, req.name.as_deref(), req.email.as_deref(), password_hash.as_deref())
			.await?
			.ok_or(AppError::NotFound)?;

		Ok(user.into())
	}

	pub async fn delete(&self, actor_id: i64, target_id: i64) -> AppResult<()> {
		if actor_id != target_id {
			return Err(AppError::Forbidden);
		}
		if self.repo.delete(target_id).await? == 0 {
			return Err(AppError::NotFound);
		}

		Ok(())
	}
```

`login` is unchanged. `get_profile` can now be deleted — the routes call `get_user(user_id, user_id)` for `me`. (The `actor_id/target_id` split exists so section 5's RBAC only has to widen one guard.)

### 2e. `src/routes/users.rs` — full replacement

```rust
use axum::{
	Json, Router,
	extract::{Path, Query, State},
	http::StatusCode,
	routing::{delete, get, put},
};
use validator::Validate;

use crate::{
	dto::user_dto::{Pagination, UpdateUserRequest, UserResponse},
	error::{AppError, AppResult},
	extractors::auth_user::AuthUser,
	services::user_service::{Actor, UserService},
	state::SharedState,
};

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/users", get(list))
		.route("/users/me", get(me))
		.route("/users/{id}", get(show).put(update).delete(destroy))
}

async fn list(State(state): State<SharedState>, AuthUser(_): AuthUser, Query(page): Query<Pagination>) -> AppResult<Json<Vec<UserResponse>>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.list(page).await?))
}

async fn me(State(state): State<SharedState>, AuthUser(user_id): AuthUser) -> AppResult<Json<UserResponse>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.get_user(user_id, user_id).await?))
}

async fn show(State(state): State<SharedState>, AuthUser(actor_id): AuthUser, Path(id): Path<i64>) -> AppResult<Json<UserResponse>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.get_user(actor_id, id).await?))
}

async fn update(State(state): State<SharedState>, AuthUser(actor_id): AuthUser, Path(id): Path<i64>, Json(payload): Json<UpdateUserRequest>) -> AppResult<Json<UserResponse>> {
	payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);

	Ok(Json(service.update(actor_id, id, payload).await?))
}

async fn destroy(State(state): State<SharedState>, AuthUser(actor_id): AuthUser, Path(id): Path<i64>) -> AppResult<StatusCode> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	service.delete(actor_id, id).await?;

	Ok(StatusCode::NO_CONTENT)
}
```

Notes: `Query` is a `FromRequest` extractor so it must be last in the handler argument list. `Actor` import is forward-referenced from section 5 — if implementing 2e before 5, import nothing and pass `actor_id` plain. Honest caveat: at this stage `GET /users` returns every account's name+email to any authenticated user. Section 5 is what closes it — do not deploy between 2 and 5 with the list route exposed, or drop `/users` from this file until then.

### 2f. `src/repositories/todo_repo.rs` — convert to macros (optional but recommended)

```rust
	pub async fn create(&self, user_id: i64, title: &str, description: &str) -> AppResult<Todo> {
		Ok(sqlx::query_as!(
			Todo,
			r#"INSERT INTO todos (user_id, title, description, completed)
					VALUES ($1, $2, $3, false)
					RETURNING id, user_id, title, description, completed, created_at, updated_at"#,
			user_id,
			title,
			description
		)
		.fetch_one(self.pool)
		.await?)
	}

	pub async fn find_by_id(&self, id: i64, user_id: i64) -> AppResult<Option<Todo>> {
		Ok(sqlx::query_as!(Todo, "SELECT id, user_id, title, description, completed, created_at, updated_at FROM todos WHERE id = $1 AND user_id = $2", id, user_id)
			.fetch_optional(self.pool)
			.await?)
	}

	pub async fn list_for_user(&self, user_id: i64) -> AppResult<Vec<Todo>> {
		Ok(sqlx::query_as!(Todo, "SELECT id, user_id, title, description, completed, created_at, updated_at FROM todos WHERE user_id = $1 ORDER BY created_at DESC", user_id)
			.fetch_all(self.pool)
			.await?)
	}

	pub async fn update(&self, id: i64, user_id: i64, title: Option<String>, description: Option<String>, completed: Option<bool>) -> AppResult<Option<Todo>> {
		Ok(sqlx::query_as!(
			Todo,
			r#"UPDATE todos
					SET title = COALESCE($3::text, title),
						 description = COALESCE($4::text, description),
						 completed = COALESCE($5::boolean, completed)
					WHERE id = $1 AND user_id = $2
					RETURNING id, user_id, title, description, completed, created_at, updated_at"#,
			id,
			user_id,
			title,
			description,
			completed
		)
		.fetch_optional(self.pool)
		.await?)
	}

	pub async fn delete(&self, id: i64, user_id: i64) -> AppResult<u64> {
		Ok(sqlx::query!("DELETE FROM todos WHERE id = $1 AND user_id = $2", id, user_id)
			.execute(self.pool)
			.await?
			.rows_affected())
	}
```

After all of this: `cargo check` compiles every query against the real schema — the macros are free schema tests.

---

## 3. OTel — telemetry init + W3C trace propagation

### 3a. `Cargo.toml` — add (pinned family; tracing-opentelemetry 0.28 pairs with otel 0.27. Do not grab latest — the 0.29+ SDK API is restructured)

```toml
opentelemetry = "0.27"
opentelemetry_sdk = { version = "0.27", features = ["rt-tokio"] }
tracing-opentelemetry = "0.28"
opentelemetry-otlp = "0.27"
```

(tonic pulls its own tower 0.5 internally; it coexists with your pinned tower 0.4 — cargo treats major versions as distinct crates.)

### 3b. `src/telemetry.rs` (new)

```rust
use opentelemetry::{
	global,
	trace::TracerProvider as _,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
	propagation::TraceContextPropagator,
	trace::{Config, Sampler},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

use crate::config::Config as AppConfig;

pub fn init(config: &AppConfig) {
	let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("axum_api=info,tower_http=info"));
	let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

	match &config.otlp_endpoint {
		Some(endpoint) => {
			let provider = opentelemetry_otlp::new_pipeline()
				.tracing()
				.with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint))
				.with_trace_config(Config::default().with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(config.trace_sample_ratio)))))
				.install_batch(opentelemetry_sdk::runtime::Tokio)
				.expect("failed to install OTLP tracer");

			global::set_tracer_provider(provider.clone());
			global::set_text_map_propagator(TraceContextPropagator::new());

			// ParentBased sampling so downstream services' traceparent decisions are honored instead of re-rolled per hop.
			let otel_layer = tracing_opentelemetry::layer().with_tracer(provider.tracer("axum_api"));
			Registry::default().with(env_filter).with(otel_layer).with(fmt_layer).init();
		}
		None => {
			Registry::default().with(env_filter).with(fmt_layer).init();
		}
	}
}

// The batch processor buffers spans in memory; exiting without flushing loses the last seconds of traces.
pub fn shutdown() {
	global::shutdown_tracer_provider();
}
```

### 3c. `src/middleware/trace.rs` (new) — and update `middleware/mod.rs` to `pub mod trace;`

```rust
use axum::{
	http::{HeaderMap, Request},
	middleware::Next,
	response::Response,
};
use opentelemetry::global;
use tracing_opentelemetry::propagation::{Extractor, Inserter};

/// Extracts the incoming W3C traceparent into the OTel context, makes it the parent of this
/// request's span, and echoes the resulting traceparent back on the response for client-side joins.
pub async fn otel_context(request: Request, next: Next) -> Response {
	let parent_cx = global::get_text_map_propagator(|prop| prop.extract(&Extractor::new(request.headers())));
	let _cx_guard = parent_cx.attach();

	let method = request.method().clone();
	let route = request.uri().path().to_string();
	let span = tracing::info_span!("HTTP request", otel.kind = "server", method = %method, route = %route);
	let mut response = span.in_scope(|| next.run(request)).await;

	let mut headers = HeaderMap::new();
	global::get_text_map_propagator(|prop| prop.inject(&mut Inserter::new(&mut headers)));
	for (name, value) in headers {
		if let Some(name) = name {
			response.headers_mut().insert(name, value);
		}
	}

	response
}
```

### 3d. `src/config.rs` — add two fields + `from_env` entries

```rust
	pub otlp_endpoint: Option<String>,
	pub trace_sample_ratio: f64,
```

```rust
			otlp_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok().filter(|s| !s.is_empty()),
			trace_sample_ratio: env::var("OTEL_TRACES_SAMPLER_RATIO").unwrap_or_else(|_| "1.0".into()).parse().map_err(|_| ConfigError::Invalid("OTEL_TRACES_SAMPLER_RATIO"))?,
```

### 3e. `src/main.rs` — full wiring for sections 2a + 3

```rust
mod config;
mod dto;
mod error;
mod extractors;
mod middleware;
mod models;
mod repositories;
mod routes;
mod services;
mod state;
mod telemetry;

#[tokio::main]
async fn main() {
	let config = Config::from_env().expect("failed to load config");
	telemetry::init(&config); // after config so endpoint/sampling come from env; the expect() above still prints to stderr

	let db_pool = create_pool(&config.database_url).await;
	sqlx::migrate!().run(&db_pool).await.expect("migrations failed");

	let port = config.server_port;
	let state = Arc::new(AppState { db_pool, config });

	// Layers added later wrap those added earlier, so otel_context is outermost:
	// the traceparent is extracted before TraceLayer or any handler code runs.
	let app = routes::all_routes()
		.with_state(state)
		.layer(TraceLayer::new_for_http())
		.layer(axum::middleware::from_fn(middleware::trace::otel_context));

	let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
	tracing::info!("Listening on 0.0.0.0:{port}");

	axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await.unwrap();

	telemetry::shutdown();
}
```

If `global::set_text_map_propagator` complains about the argument type on your resolved otel 0.27.x patch, wrap: `Box::new(TraceContextPropagator::new())` — the signature flipped from `Box<dyn ...>` to `impl` during the 0.27 series.

---

## 4. RBAC — roles table + `RequireRole` extractor

Design decisions baked in:
- **Roles are read from Postgres per request, not from the JWT.** Claims freeze role state at issuance; DB reads give instant revocation. The extractor is the seam where a cache gets added if it ever shows up in a profile.
- **Join table + audit trail** — grant provenance (`granted_by`) and an immutable audit row in the same transaction as every mutation. If you will *only* ever have two roles, a CHECK'd column on `users` is simpler — this is the choice for when role growth is on the roadmap.
- **The requirement is a marker type, not a runtime string** — a handler cannot accidentally guard with the wrong role.

### 4a. Migration — `sqlx migrate add rbac`

```sql
CREATE TABLE roles (
	id   BIGSERIAL PRIMARY KEY,
	name TEXT NOT NULL UNIQUE CHECK (name IN ('user', 'admin'))
);

INSERT INTO roles (name) VALUES ('user'), ('admin');

CREATE TABLE user_roles (
	user_id    BIGINT      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
	role_id    BIGINT      NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
	granted_by BIGINT      REFERENCES users(id) ON DELETE SET NULL,
	granted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
	PRIMARY KEY (user_id, role_id)
);

CREATE INDEX idx_user_roles_user ON user_roles (user_id);

-- Audit rows outlive the grants they describe (SET NULL, not CASCADE) — this table is evidence.
CREATE TABLE role_audit (
	id         BIGSERIAL PRIMARY KEY,
	actor_id   BIGINT      NOT NULL REFERENCES users(id),
	target_id  BIGINT      NOT NULL REFERENCES users(id),
	role       TEXT        NOT NULL,
	action     TEXT        NOT NULL CHECK (action IN ('grant', 'revoke')),
	created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Backfill: everyone existing gets the base role, or the first deploy 403s every user.
INSERT INTO user_roles (user_id, role_id)
SELECT u.id, r.id FROM users u CROSS JOIN roles r WHERE r.name = 'user'
ON CONFLICT DO NOTHING;
```

```bash
cargo sqlx migrate run   # before cargo check — the macros below need these tables live
```

### 4b. `src/models/role.rs` (new) — `models/mod.rs` += `pub mod role;`

```rust
use std::{fmt, str::FromStr};

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

impl fmt::Display for Role {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

#[derive(Debug, thiserror::Error)]
#[error("unknown role: {0}")]
pub struct UnknownRole(pub String);

impl FromStr for Role {
	type Err = UnknownRole;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"user" => Ok(Role::User),
			"admin" => Ok(Role::Admin),
			other => Err(UnknownRole(other.into())),
		}
	}
}
```

`Ord` on the variants gives you a role *ceiling* later (require-min-role instead of exact-set match) — free optionality, zero code today.

### 4c. `src/extractors/require_role.rs` (new) — `extractors/mod.rs` += `pub mod require_role;`

```rust
use std::collections::BTreeSet;

use axum::{
	extract::FromRequestParts,
	http::request::Parts,
};

use super::auth_user::AuthUser;
use crate::{
	error::AppError,
	models::role::Role,
	repositories::role_repo::RoleRepository,
	state::SharedState,
};

pub trait RoleRequirement {
	const ROLES: &'static [Role];
}

#[derive(Clone, Copy)]
pub struct Admin;

impl RoleRequirement for Admin {
	const ROLES: &'static [Role] = &[Role::Admin];
}

#[derive(Clone, Copy)]
pub struct UserOrAdmin;

impl RoleRequirement for UserOrAdmin {
	const ROLES: &'static [Role] = &[Role::User, Role::Admin];
}

pub struct RequireRole<R: RoleRequirement> {
	pub user_id: i64,
	pub roles: BTreeSet<Role>,
}

impl<R: RoleRequirement> RequireRole<R> {
	pub fn is_admin(&self) -> bool {
		self.roles.contains(&Role::Admin)
	}
}

impl<R: RoleRequirement> FromRequestParts<SharedState> for RequireRole<R> {
	type Rejection = AppError;

	async fn from_request_parts(parts: &mut Parts, state: &SharedState) -> Result<Self, Self::Rejection> {
		let AuthUser(user_id) = AuthUser::from_request_parts(parts, state).await?;

		let roles = RoleRepository::new(&state.db_pool).roles_for_user(user_id).await?;

		if !R::ROLES.iter().any(|r| roles.contains(r)) {
			return Err(AppError::Forbidden);
		}

		Ok(Self { user_id, roles })
	}
}
```

Rejection is `AppError`, not `AuthError`, on purpose: a DB failure must surface as a 500, not an accidental "no roles → 403". Failure ≠ denial — don't "simplify" that by mapping `Database` errors to Forbidden.

### 4d. `src/extractors/auth_user.rs` — append (so `?` works in RequireRole)

```rust
impl From<AuthError> for AppError {
	fn from(e: AuthError) -> Self {
		// The detailed 401 body from AuthError::into_response is lost here; clients get the generic "unauthorized" message.
		match e {
			AuthError::MissingToken | AuthError::InvalidToken => AppError::Unauthorized,
		}
	}
}
```

### 4e. `src/repositories/role_repo.rs` (new) — `repositories/mod.rs` += `pub mod role_repo;`

```rust
use std::collections::BTreeSet;

use sqlx::{Executor, PgPool, Postgres, Transaction};

use crate::{
	error::AppResult,
	models::role::Role,
};

pub struct RoleRepository<'a> {
	pool: &'a PgPool,
}

impl<'a> RoleRepository<'a> {
	pub fn new(pool: &'a PgPool) -> Self {
		Self { pool }
	}

	// Runs on every authenticated request: two indexed lookups, single round-trip.
	pub async fn roles_for_user(&self, user_id: i64) -> AppResult<BTreeSet<Role>> {
		let rows = sqlx::query!(
			r#"SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id = r.id WHERE ur.user_id = $1"#,
			user_id
		)
		.fetch_all(self.pool)
		.await?;

		Ok(rows.iter().filter_map(|r| r.name.parse().ok()).collect())
	}

	// FOR UPDATE OF ur locks only the grant rows, not the shared roles row — otherwise concurrent
	// grants to different users would serialize on role_id.
	pub async fn roles_for_user_in(tx: &mut Transaction<'_, Postgres>, user_id: i64) -> AppResult<BTreeSet<Role>> {
		let rows = sqlx::query!(
			r#"SELECT r.name FROM roles r JOIN user_roles ur ON ur.role_id = r.id WHERE ur.user_id = $1 FOR UPDATE OF ur"#,
			user_id
		)
		.fetch_all(&mut **tx)
		.await?;

		Ok(rows.iter().filter_map(|r| r.name.parse().ok()).collect())
	}

	// Executor-generic: the same code runs against a pool (no tx) or a caller-owned Transaction.
	// The CTE makes "user row + base role" atomic, so registration can't half-succeed.
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
		.fetch_one(executor)
		.await?;

		Ok(row.id)
	}

	pub async fn grant_or_revoke(tx: &mut Transaction<'_, Postgres>, actor_id: i64, target_id: i64, role: Role, grant: bool) -> AppResult<()> {
		if grant {
			sqlx::query!(
				r#"INSERT INTO user_roles (user_id, role_id, granted_by)
						SELECT $1, r.id, $2 FROM roles r WHERE r.name = $3
						ON CONFLICT DO NOTHING"#,
				target_id,
				actor_id,
				role.as_str()
			)
			.execute(&mut **tx)
			.await?;
		} else {
			sqlx::query!(
				r#"DELETE FROM user_roles
					 WHERE user_id = $1 AND role_id = (SELECT id FROM roles WHERE name = $2)"#,
				target_id,
				role.as_str()
			)
			.execute(&mut **tx)
			.await?;
		}

		sqlx::query!(
			"INSERT INTO role_audit (actor_id, target_id, role, action) VALUES ($1, $2, $3, $4)",
			actor_id,
			target_id,
			role.as_str(),
			if grant { "grant" } else { "revoke" }
		)
		.execute(&mut **tx)
		.await?;

		Ok(())
	}
}
```

`ANY($4)` requires binding `&Vec<String>` — the enum array is not a Postgres type; convert via `as_str` as shown.

### 4f. `src/services/user_service.rs` — RBAC changes

New `Actor` type (top level of the file), new `pool` field, and the guards widen:

```rust
use crate::{
	error::{AppError, AppResult},
	models::role::Role,
	repositories::role_repo::RoleRepository,
	...
};

#[derive(Debug, Clone, Copy)]
pub struct Actor {
	pub id: i64,
	pub is_admin: bool,
}

impl Actor {
	fn owns(&self, target_id: i64) -> bool {
		self.id == target_id || self.is_admin
	}
}

pub struct UserService<'a> {
	pool: &'a PgPool,
	repo: UserRepository<'a>,
	jwt_secret: &'a str,
}

impl<'a> UserService<'a> {
	pub fn new(pool: &'a PgPool, jwt_secret: &'a str) -> Self {
		Self {
			repo: UserRepository::new(pool),
			pool,
			jwt_secret,
		}
	}
```

Replace the three `actor_id != target_id` guards from section 2d with `if !actor.owns(target_id) { return Err(AppError::Forbidden); }` — signatures become `get_user(&self, actor: Actor, target_id: i64)`, `update(&self, actor: Actor, ...)`, `delete(&self, actor: Actor, ...)`. `register` swaps `self.repo.create(...)` for the atomic create-with-base-role:

```rust
	pub async fn register(&self, req: RegisterRequest) -> AppResult<AuthResponse> {
		let password_hash = Self::hash_password(&req.password)?;
		// Base role applied atomically with the user row; an admin can never be minted through registration by construction.
		let id = RoleRepository::create_user_with_roles(self.pool, &req.name, &req.email, &password_hash, &[Role::User]).await?;
		let user = self.repo.find_by_id(id).await?.ok_or(AppError::Internal(anyhow::anyhow!("user vanished after create")))?;
		let token = self.issue_token(user.id)?;

		Ok(AuthResponse { token, user: user.into() })
	}
```

Add the role mutation:

```rust
	pub async fn set_role(&self, actor: Actor, target_id: i64, role: Role, grant: bool) -> AppResult<()> {
		if !grant && role == Role::User {
			return Err(AppError::BadRequest("cannot revoke base role".into()));
		}

		let mut tx = self.pool.begin().await?;
		// Lock the grant set first: serializes against a concurrent revoke on the same user,
		// and the self-demotion check is only race-free under this lock.
		let current = RoleRepository::roles_for_user_in(&mut tx, target_id).await?;
		if actor.id == target_id && !grant && current.contains(&Role::Admin) && current.len() == 1 {
			return Err(AppError::BadRequest("cannot revoke your last admin role".into()));
		}

		RoleRepository::grant_or_revoke(&mut tx, actor.id, target_id, role, grant).await?;
		tx.commit().await?;

		tracing::info!(actor = actor.id, target = target_id, role = %role, grant, "rbac mutation");

		Ok(())
	}
```

### 4g. `src/dto/user_dto.rs` — append

```rust
#[derive(Debug, Deserialize)]
pub struct SetRoleRequest {
	pub role: Role,
}
```

(`serde(rename_all = "lowercase")` on `Role` validates the string at deserialization — an unknown role 422s before the service sees it. Import `crate::models::role::Role`.)

### 4h. `src/routes/users.rs` — swap guards from `AuthUser` to `RequireRole`

```rust
use crate::{
	dto::user_dto::{Pagination, SetRoleRequest, UpdateUserRequest, UserResponse},
	extractors::require_role::{Admin, RequireRole, UserOrAdmin},
	models::role::Role,
	services::user_service::{Actor, UserService},
	...
};

pub fn routes() -> Router<SharedState> {
	Router::new()
		.route("/users", get(list))
		.route("/users/me", get(me))
		.route("/users/{id}", get(show).put(update).delete(destroy))
		.route("/users/{id}/roles", put(set_role).delete(revoke_role))
}

// Was AuthUser(_) — any authenticated user could enumerate every account. Now admin-gated at the type level.
async fn list(State(state): State<SharedState>, RequireRole { .. }: RequireRole<Admin>, Query(page): Query<Pagination>) -> AppResult<Json<Vec<UserResponse>>> { ... unchanged body ... }

async fn me(State(state): State<SharedState>, actor: RequireRole<UserOrAdmin>) -> AppResult<Json<UserResponse>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	let actor = Actor { id: actor.user_id, is_admin: actor.is_admin() };

	Ok(Json(service.get_user(actor, actor.id).await?))
}

async fn show(State(state): State<SharedState>, actor: RequireRole<UserOrAdmin>, Path(id): Path<i64>) -> AppResult<Json<UserResponse>> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	let actor = Actor { id: actor.user_id, is_admin: actor.is_admin() };

	Ok(Json(service.get_user(actor, id).await?))
}

async fn set_role(State(state): State<SharedState>, RequireRole { user_id, .. }: RequireRole<Admin>, Path(target_id): Path<i64>, Json(payload): Json<SetRoleRequest>) -> AppResult<StatusCode> {
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	service.set_role(Actor { id: user_id, is_admin: true }, target_id, payload.role, true).await?;

	Ok(StatusCode::NO_CONTENT)
}

async fn revoke_role(State(state): State<SharedState>, RequireRole { user_id, .. }: RequireRole<Admin>, Path((target_id, role_str)): Path<(i64, String)>) -> AppResult<StatusCode> {
	let role = role_str.parse::<Role>().map_err(|_| AppError::BadRequest(format!("unknown role: {role_str}")))?;
	let service = UserService::new(&state.db_pool, &state.config.jwt_secret);
	service.set_role(Actor { id: user_id, is_admin: true }, target_id, role, false).await?;

	Ok(StatusCode::NO_CONTENT)
}
```

`update`/`destroy` follow `show`'s pattern (build `Actor` from the extractor, pass into the service).

### 4i. Admin bootstrap — do not skip

A fresh DB has **zero admins**, and granting requires one — you can't climb out of that with the API. One-time idempotent grant at startup:

`config.rs` += `pub admin_email: Option<String>,` with `admin_email: env::var("ADMIN_EMAIL").ok().filter(|s| !s.is_empty()),`, then in `main.rs` after `migrate!()`:

```rust
	if let Some(email) = &config.admin_email {
		// First admin is always a claim of an already-authenticated identity — never a seeded known-password account.
		sqlx::query!(
			r#"INSERT INTO user_roles (user_id, role_id)
					SELECT u.id, r.id FROM users u, roles r WHERE lower(u.email) = lower($1) AND r.name = 'admin'
					ON CONFLICT DO NOTHING"#,
			email
		)
		.execute(&db_pool)
		.await
		.expect("admin bootstrap failed");
	}
```

(Or skip the hook and `psql` that exact statement by hand once. Same result.)

---

## 5. Verification

```bash
# schema state
cargo sqlx migrate run

# compile-time query verification + tests + lints
cargo check
cargo test && cargo clippy
cargo fmt

# runtime smoke
DATABASE_URL=... ADMIN_EMAIL=sam@example.com JWT_SECRET=devsecret cargo run

curl -s -X POST localhost:3000/auth/register -H 'content-type: application/json' \
  -d '{"name":"Sam","email":"sam@example.com","password":"supersecret1"}'
# grab token -> ALICE_TOKEN; then:
curl -si localhost:3000/users -H "authorization: Bearer $ALICE_TOKEN"     # 200 only after 4i bootstrap ran
curl -s localhost:3000/users/me -H "authorization: Bearer $ALICE_TOKEN"   # 200, self
curl -si localhost:3000/users/999 -H "authorization: Bearer $ALICE_TOKEN" # 403 (or 404 if admin)
curl -s -X PUT localhost:3000/users/1 -H "authorization: Bearer $ALICE_TOKEN" -H 'content-type: application/json' -d '{"name":"Sam Jr"}'
curl -s -X PUT localhost:3000/users/1/roles -H "authorization: Bearer $ALICE_TOKEN" -H 'content-type: application/json' -d '{"role":"admin"}'
curl -si -X PUT localhost:3000/users/2/roles -H "authorization: Bearer $BOB_TOKEN" -H 'content-type: application/json' -d '{"role":"admin"}'  # 403 — non-admin
psql axum_api_dev -c "SELECT * FROM role_audit ORDER BY id DESC LIMIT 5"  # grant recorded with actor

# OTel: endpoint unset = plain fmt logs (silent path is intended); run a collector locally, then:
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 OTEL_TRACES_SAMPLER_RATIO=1.0 cargo run
curl -si localhost:3000/health | grep -i traceparent   # response echoes trace id
```

## Known sharp edges (fix after the above is green)

1. `GET /users/{id}` lets *any* authenticated user fetch any *other* user's name/email once they know the numeric id (enumerable!). Either require `owns()` (already the case: `show` returns Forbidden for strangers unless admin — double check that's what you want vs a public profile), or drop `email` from non-self `UserResponse`.
2. JWT has no `jti`/revocation list; a token stays valid for its full 15-min TTL even after `DELETE /users/{id}` — because the user row is gone, `RequireRole`'s join returns nothing → 403, which happens to close the hole for RBAC-gated routes but not for `AuthUser`-only routes. Refresh-token work will address it properly.
3. sqlx-postgres 0.7.4 has a future-incompat note; flipping to the commented sqlx 0.8 line in Cargo.toml is the upgrade path (also gives you the `.sqlx/` offline directory format).
