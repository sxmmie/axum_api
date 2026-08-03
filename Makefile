.PHONY: help build run test lint fmt fmt-check clean docker-build docker-up docker-down docker-clean migrate migrate-revert sqlx-prepare check watch

DATABASE_URL=postgres://axum_postgres:adminPass@localhost:5432/axum_postgres_db
BINARY_NAME=axum_postgres

# self-documenting help target — a common Makefile pattern that turns your ## comments into a auto-generated menu,
help:
	@grep -E '^[a-zA-Z_-]+:.*?##' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build release binary
	cargo build --release

run: ## Run in dev mode
	cargo run

watch: ## Auto-reload on change (requires cargo-watch)
	cargo watch -x run

build: ## Build the application
	cargo build --release

test: ## Run all tests
	SQLX_OFFLINE=true cargo test -- --nocapture

check: ## Fast type-check without codegen
	cargo check --all-targets

lint: ## Run clippy, deny warnings
	cargo clippy --all-targets --all-features -- -D warnings

fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check formatting without writing
	cargo fmt --all -- --check

clean: ## Remove artifacts
	cargo clean

docker-build: ## Build production image
	docker build -t ${BINARY_NAME}:latest .

docker-up: ## Start local stack (app + db) docker-compose services
	docker-compose up -d --build

docker-down: ## Stop local stack
	docker-compose down

migrate: ## Apply pending migrations
	sqlx migrate run --database-url ${DATABASE_URL}

migrate-revert: ## Revert last migration
	sqlx migrate revert --database-url ${DATABASE_URL}

sqlx-prepare: ## Regenerate offline query cache for CI builds
	cargo sqlx prepare --database-url "$(DATABASE_URL)" -- --all-targets --all-features

ci: fmt-check lint test ## Full CI pipeline locally
