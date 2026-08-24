# Development Guide

This document summarizes how to get the project running for the first time, manage its dependencies, and run tests.

## 1. Prerequisites

- [Install Rust] https://www.rust-lang.org/tools/install

```bash
# Check the Rust compiler version
rustc --version

# Check the Cargo version
cargo --version
```

- [Install Docker] https://docs.docker.com/get-started/get-docker/

```bash
# Check the Docker version
docker --version

# Check the Docker Compose version
docker compose version
```

## 2. Project Initialization

If you have already cloned the repository, do not run `cargo init` again in the project root.

```bash
git clone https://github.com/1Tsw0rd/rust-axum-layered-rest-api.git
cd rust-axum-layered-rest-api
cargo build
```

Since `Cargo.toml` and `Cargo.lock` are already included in the repository, `cargo build` will download the dependencies without needing a separate `cargo add`.

## 3. Cargo Commands

```bash
# Build with the development profile
cargo build

# Run the executable
cargo run

# Quickly check that dependencies and code compile, without producing a binary
cargo check

# Run the full test suite
cargo test

# Run tests while also showing output logs
cargo test -- --nocapture

# Note: this project's integration tests share state,
# so it is recommended to use `cargo test -- --test-threads=1` when actually running tests

# fmt reformats code for readability without changing its behavior
# Apply formatting
cargo fmt
# Check formatting
cargo fmt -- --check

# Clippy static analysis
# Clippy is a static analysis tool that points out things you could improve or potential mistakes in your Rust code
# Basic static analysis
cargo clippy
# Strictly check all targets and all features
# -- -D(deny) warnings: promotes every lint at the warning level to an error
cargo clippy --all-targets --all-features -- -D warnings
```

## 4. Commands Used to Add Dependencies

The commands below are a record of what was actually used to set up the project.
After cloning the repository, you do not need to run them again — just run `cargo build`.

Running `cargo add` in the project root records the feature flags in `Cargo.toml`, so they are managed together with the command.

### Application Dependencies

```bash
# Update dependencies to their latest version within the version constraints in Cargo.toml
# To update only a specific package (cargo update -p axum)
cargo update

# Web framework: an async HTTP web framework developed by the Tokio team
cargo add axum --features "macros"

# Async runtime: provides the async execution environment for network I/O, timers, tasks, concurrency, etc.
cargo add tokio --features "full"

# Data conversion: converts JSON byte data to and from Rust structs (automates serialization/deserialization)
cargo add serde --features "derive"

# A library for validating request DTO field rules using #[derive(Validate)] and #[validate(...)]
cargo add validator --features "derive"

# info!, warn!, error! logging macros
cargo add tracing

# Log printer
# fmt: handles printing logs to the terminal
# env-filter: supports filtering log levels per module based on environment variables
cargo add tracing-subscriber --features "fmt,env-filter"

# HTTP request/response tracing middleware
cargo add tower-http --features "trace"

# Time type mapper: enables Serde integration so DateTime structs are serialized to JSON correctly
cargo add chrono --features "serde"

# DB integration: an async SQL toolkit
# runtime-tokio: Tokio integration
# tls-rustls: encrypted communication
# postgres: Postgres driver
# uuid: UUID type mapping
# chrono: time type mapping
# macros: compile-time query verification macros
cargo add sqlx --features "runtime-tokio,tls-rustls,postgres,uuid,chrono,macros"

# UUID v4 generation and Serde serialization/deserialization
cargo add uuid --features "v4,serde"

# .env environment variable loader
cargo add dotenvy

# Argon2id password hashing and verification
cargo add argon2

# JWT generation, signing, and verification
# rust_crypto: uses the RustCrypto family of cryptographic implementations
cargo add jsonwebtoken --features "rust_crypto"

# OS-based low-level cryptographic random (random byte) generation
cargo add getrandom

# SHA-256 hashing
cargo add sha2

# A client for communicating asynchronously with and managing connections to Redis or Dragonfly
cargo add redis --features "tokio-comp,connection-manager"

# Extension for reading and creating cookies
cargo add axum-extra --features "cookie"
```

### Test-only Dependencies

```bash
# Enables a utility (.oneshot) that simulates HTTP requests in memory without actually starting the server
cargo add tower --dev --features "util"

# HTTP body collection/verification utility: collects and converts the streamed byte body data Axum returns into human-readable JSON/text
cargo add http-body-util --dev

# Creates and verifies JSON requests/responses for tests
cargo add serde_json --dev
```

## 5. Environment Variables

`.env` is for local execution and `.env.test` is for integration tests.

| Variable | Description |
|---|---|
| `DATABASE_URL` | The PostgreSQL URL; the environment variable name SQLx recognizes by default |
| `TEST_DATABASE_URL` | PostgreSQL connection string used for integration tests |
| `DB_USER` / `DB_PASSWORD` | PostgreSQL account credentials |
| `DB_HOST` | PostgreSQL host |
| `DB_PORT` | Port used to access PostgreSQL from the host |
| `DB_NAME` | PostgreSQL database name |
| `JWT_SECRET` | JWT signing key |
| `JWT_ACCESS_TOKEN_EXPIRES_IN_SECONDS` | Access token expiration time |
| `CACHE_BACKEND` | `redis` or `dragonfly` |
| `REDIS_HOST` / `REDIS_PORT` / `REDIS_PASSWORD` | Redis connection settings |
| `DRAGONFLY_HOST` / `DRAGONFLY_PORT` / `DRAGONFLY_PASSWORD` | Dragonfly connection settings |
| `REDIS_DB` / `DRAGONFLY_DB` | Redis/Dragonfly DB number (0: development, 1: test) — used to isolate development and test environments |
| `RUST_LOG` | Log level filter per module |

## 6. Docker Compose

```bash
# Run PostgreSQL, Redis, and Dragonfly in the background
docker compose up -d

# Check running status
docker compose ps

# View all logs
docker compose logs -f

# View logs for a specific service
docker compose logs -f postgres
docker compose logs -f redis
docker compose logs -f dragonfly

# Stop the containers
docker compose down
```

### Commands for Connecting to the Redis/Dragonfly Docker Containers

```bash
# Redis
docker compose exec redis redis-cli -a "$REDIS_PASSWORD" -n "$REDIS_DB"

# Dragonfly
docker compose exec dragonfly redis-cli -a "$DRAGONFLY_PASSWORD" -n "$DRAGONFLY_DB"
```

Commonly used commands:

```text
SELECT <DB number>    Select a DB (0: dev DB, 1: test DB)
PING                 Check the connection
SCAN 0               Safely iterate over keys
GET <key>            Retrieve a string value
TTL <key>            Check the remaining time-to-live
DEL <key>            Delete a key
FLUSHDB              Delete all data in the current DB (use with caution)
```

## 7. Testing

Unit tests verify individual pieces of logic such as DTOs, Role, Permission, and JWT, while integration tests send actual HTTP requests to the Axum router to verify domain-level flows.

The PostgreSQL and Redis/Dragonfly environments used for testing are kept separate from the development environment.

PostgreSQL connects to `TEST_DATABASE_URL` from `.env.test`, and the `accounts` and `products` tables are reset when the tests start.

Redis/Dragonfly uses the test DB number (`1`) from `.env.test`, keeping it separate from the development DB (`0`), and its data is reset when the tests start.

```bash
# Run all unit and integration tests
# Run serially to avoid conflicts from shared DB/Redis state
cargo test -- --test-threads=1

# All unit tests inside src
cargo test --lib

# Unit tests for a specific module inside src
cargo test --lib common::auth::jwt
cargo test --lib common::auth::authorization
cargo test --lib common::auth::extractor
cargo test --lib common::auth::permission
cargo test --lib common::auth::role
cargo test --lib common::auth::refresh_token
cargo test --lib common::security::password
cargo test --lib domains::account::dto

# All integration API tests
cargo test --test api -- --test-threads=1

# Per Account API file
cargo test --test api account::create_account -- --test-threads=1
cargo test --test api account::login -- --test-threads=1
cargo test --test api account::get_me -- --test-threads=1
cargo test --test api account::refresh -- --test-threads=1
cargo test --test api account::logout -- --test-threads=1

# Per Product API file
cargo test --test api product::get_product -- --test-threads=1
cargo test --test api product::get_products -- --test-threads=1
cargo test --test api product::create_product -- --test-threads=1
cargo test --test api product::put_product -- --test-threads=1
cargo test --test api product::patch_product -- --test-threads=1
cargo test --test api product::delete_product -- --test-threads=1

# Example of running a single specific test function
cargo test --lib common::auth::jwt::tests::create_and_verify_success

# Show test logs
cargo test --test api account::login -- --test-threads=1 --nocapture
```

By default, `cargo test` runs tests in parallel. Since this project resets PostgreSQL and Redis data for every test, `--test-threads=1` should be used together with the full test suite and the API integration tests.

## 8. Branch and Commit Conventions

The default workflow uses `develop` as the integration branch: changes are made on per-task branches and merged into `develop` via pull request.

```text
develop
└── feature / docs / bugfix, etc. branches
```

Commit messages use the following format.

```text
type: description of the work
```

| Type | Purpose | Example |
|---|---|---|
| `feat` | Add a feature | `feat: add account registration API` |
| `fix` | Fix a bug | `fix: handle invalid JWT token` |
| `docs` | Write/update documentation | `docs: add API documentation` |
| `chore` | Project maintenance, initial setup | `chore: initialize Rust project` |
| `build` | Dependency/build configuration | `build: add project dependencies` |
| `test` | Add/update tests | `test: add account API tests` |
| `refactor` | Structural improvement with no functional change | `refactor: separate repository params` |
| `style` | Formatting, whitespace, semicolons, etc. | `style: format Rust source files` |
| `perf` | Performance improvement | `perf: optimize product query` |

Once your work is done, run `cargo fmt`, `cargo check`, and the relevant tests before committing.
