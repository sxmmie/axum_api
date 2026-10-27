# AGENTS.md

Guidance for AI coding agents working in this repository.

## Project Overview

A layered REST API written in Rust using **axum 0.8**, **sqlx 0.7** (Postgres, runtime-tokio-rustls), and **tokio**. Edition 2024. Domain: user auth (JWT) + todos, with role-based access control in progress.

`docs/IMPLEMENTATION.md` is the de-facto contributor guide and roadmap. Read it before making changes on the current feature branch — it contains the authoritative plans and corrected code snippets for in-flight work.

## Architecture

Request flow: **routes (handlers) → services → repositories → Postgres**.

```
src/
├── main.rs          # Entry point. Declares all modules (binary-crate style; lib.rs is empty/unused).
│                    # Boots: config → telemetry → PgPool → Arc<AppState> → router → serve with graceful shutdown
├── config.rs        # Config::from_env() via dotenvy + std::env::var (no config crate). Env: DATABASE_URL, PORT, JWT_SECRET, APP_ENV, OTEL_*
├── state.rs         # AppState { db_pool, config, ... } and type SharedState = Arc<AppState>
├── error.rs         # AppError (thiserror) + IntoResponse impl + `pub type AppResult<T> = Result<T, AppError>`
├── telemetry.rs     # tracing-subscriber EnvFilter + optional OTLP/OpenTelemetry pipeline
├── dto/             # Request/response structs (serialization boundary). *_dto.rs
├── extractors/      # FromRequestParts impls, e.g. AuthUser(i64) — JWT bearer extraction
├── middleware/      # trace.rs (otel_context); rate_limit/timing/logging are WIP stubs
├── models/          # DB row structs (sqlx::FromRow): user.rs, todo.rs, role.rs
├── repositories/    # All raw SQL lives here: *_repo.rs. Maps SQLSTATE 23505 → AppError::Conflict
├── routes/          # Per-resource handlers + `pub fn routes() -> Router<SharedState>`; mod.rs::all_routes() merges them
└── services/        # Business logic, authz (ownership checks), Argon2 hashing, JWT issuance
```

Key patterns:
- Handlers are thin private `async fn`s co-located with their routes. Services are constructed per-request with borrowed state: `UserService::new(&state.db_pool, &state.config.jwt_secret)`.
- Repositories/services are lifetime-borrowed structs over `&'a PgPool` — no trait abstractions, no mocks.
- Models convert to responses via `impl From<Model> for *Response` in the dto files. `password_hash` must never escape the repository layer.
- Handlers return `AppResult<T>` or `AppResult<(StatusCode, Json<T>)>`; deletes return `StatusCode::NO_CONTENT`.
- Errors map to uniform JSON `{ "error": message }`: Validation→422, Conflict→409, Unauthorized→401, Forbidden→403, NotFound→404, Database/Internal→500 (logged with `tracing::error!`, generic message to client).
- Auth: HS256 JWT (`jsonwebtoken`, `Header::default()`), 15-min TTL, claims `{ sub, exp, iat }`. Protected handlers take `AuthUser(user_id): AuthUser` as second argument. Authorization is currently per-service ownership checks; RBAC (`RequireRole` extractor, roles read from DB per request) is the active WIP feature — see IMPLEMENTATION.md §4.
- Validation: `validator` crate; request DTOs derive `Validate`, handlers call `payload.validate()` and map errors to `AppError::Validation`.
- Routing: **axum 0.8 path syntax** — `/users/{id}`, not `:id`. No API versioning.

## Database

- Migrations in `migrations/` using `sqlx migrate` naming. Postgres via `DATABASE_URL`; pool max_connections(10).
- **Mixed query styles**: runtime `sqlx::query_as::<_, Model>(...)` AND compile-time `sqlx::query!` macros. The macros require a live `DATABASE_URL` at compile time (no offline `.sqlx/` cache is committed yet). Either run Postgres (e.g. `make docker-up`) or run `make sqlx-prepare` before building/clippy.
- Schema conventions: BIGSERIAL ids, `lower(email)` unique index, `set_updated_at()` trigger, FK `ON DELETE CASCADE`.

## Commands

Use the Makefile (run `make help` for the full list):

| Task | Command |
|---|---|
| Build / run | `make build` / `make run` (`make watch` for cargo-watch) |
| Full check | `make ci` = `fmt-check` + `lint` (clippy `-D warnings`) + `test` (`SQLX_OFFLINE=true cargo test`) |
| Format | `make fmt` (rustfmt.toml: **hard tabs, max_width 180**, nightly-only import granularity options) |
| Migrations | `make migrate` / `make migrate-revert` (hardcoded local DATABASE_URL) |
| Docker dev stack | `make docker-up` — api:3030, postgres:16, redis:7, adminer:8081 |

## Testing

- Very sparse: one unit test (`all_routes_builds` in `src/routes/mod.rs`). No `tests/` dir, no integration tests or fixtures.
- Verification today is largely: compile-time sqlx macro checks as "free schema tests" + manual `curl` smoke tests documented in IMPLEMENTATION.md §5 (register → login → bearer token → hit endpoints, check 401/403/404 semantics).
- If you add tests, prefer unit tests on services/DTOs; integration tests need a live or mocked Postgres, which is not yet set up.

## Conventions

- File naming: `*_repo.rs`, `*_service.rs`, `*_dto.rs`. Handler names: `list`, `me`, `show`, `create`, `update`, `destroy`.
- JSON is snake_case (serde default). Requests: `Deserialize + Validate`; responses: `Serialize`.
- Errors via `AppResult`/`?` with `.ok_or(AppError::NotFound)?` for Option handling; early-return guard clauses for authz.
- Login must return identical errors for unknown email vs wrong password (anti-enumeration).
- The codebase intentionally carries heavy explanatory comments about security and design rationale — preserve and match this style.
- Logging: structured `tracing` events (`tracing::error!(error = ?e, ...)`); `RUST_LOG` controls levels.

## Current State Caveats (branch `chore/add-rbac`)

- **The working tree may not compile.** Known WIP breakage: missing imports in `src/repositories/role_repo.rs`, `redis` field in `state.rs` without the crate in Cargo.toml, misplaced module-scope fields in `config.rs`, syntax error in `middleware/trace.rs`, `FROM check` typo in Dockerfile.
- When fixing vs adding features, prefer the corrected snippets in `docs/IMPLEMENTATION.md` over improvising.
- Stray dead artifacts at repo root: empty `validation.rs`, empty `pagination/` and `response/` dirs, empty `lib.rs`, unused `uuid` dep — do not treat them as architecture.

## Do Not

- Do not change layering: no SQL in handlers/services, no business logic in repositories, no DTOs leaking into models.
- Do not add trait/mock abstraction around repos unless explicitly asked — concrete `&PgPool` borrowing is the established pattern.
- Do not put secrets in code or logs; `JWT_SECRET` and `DATABASE_URL` come from env (`.env.example` documents them).
