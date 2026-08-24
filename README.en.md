# Rust Axum REST API

```text
┌──────────────────────────────────────┐
│        RUST · AXUM · REST API        │
│     Layered Architecture Example     │
└──────────────────────────────────────┘
```

A REST API project built on Rust + Axum.

It applies the layered server structure familiar from NestJS / Spring Boot to Rust/Axum, and implements input validation, common response/error handling, Role/Permission-based authorization, JWT authentication, Refresh Token Rotation, and Redis/Dragonfly session management, all built from scratch.

> **Project goal**  
> Beyond a simple CRUD example, the goal is to bring together, in a single project, a basic Rust/Axum server structure and authentication/authorization patterns that can serve as a reference when adding new APIs.

> **Project context**
>
> - Time learning Rust: about 1 month
> - Time learning Axum and developing the project: about 3 weeks
> - Development approach: vibe coding, carried out while continuously discussing design and implementation direction with AI
> - AI tools used (free tiers): ChatGPT, Grok, Claude, Google AI Search engine
> - AI coding tool used: Codex (about 3 days)
> - Nature of the project: a personal learning project in which server fundamentals were learned by directly designing and implementing things while studying Rust/Axum
> - Current level: a project at the learning stage, not a finished product by an experienced Rust/Axum developer
---

## Architecture

```text
HTTP Request
     │
     ▼
┌──────────────┐
│  Controller  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│     DTO      │  ← Body / Path / Query Validation
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Service    │  ← Business Logic
└──────┬───────┘
       │
       ▼
┌──────────────────┐
│ Repository Params│  ← DB Input Model
└──────┬───────────┘
       │
       ▼
┌──────────────┐
│  Repository  │  ← SQLx
└──────┬───────┘  ◀──────────────────┐
       │                              │
       ▼                              │
┌──────────────┐                 ┌───────────┐
│ PostgreSQL   │───────────────▶ │ Entity   │
└──────────────┘                 └───────────┘
```

### Role of Each Layer

- `controller`: Receives HTTP requests and returns responses.
- `dto`: Defines request/response data and input validation rules.
- `service`: Handles the domain's business rules.
- `repository`: Communicates directly with PostgreSQL.
- `params`: The database input value model passed to the Repository.
- `entity`: The model mapped to a database row.
- `common`: Functionality shared across multiple domains, such as authentication, errors, responses, and middleware.

### Authentication Flow

1. On a successful login, the Access Token is returned as JSON.
2. The Refresh Token is stored as an HttpOnly Cookie.
3. Protected routes apply JWT authentication and permission checks via `secured_route!`.
4. The raw Refresh Token is never stored — only its hash is stored in Redis/Dragonfly.

---
## Features

- [1. Simple Conversion Between Layers](#1-simple-conversion-between-layers)
- [2. Request Validation](#2-request-validation)
- [3. SQLx Data Access](#3-sqlx-data-access)
- [4. Common API Response / Error](#4-common-api-response--error)
- [5. Request ID & Logging](#5-request-id--logging)
- [6. Redis / Dragonfly Backend Switching](#6-redis--dragonfly-backend-switching)
- [7. Role + Permission](#7-role--permission)
- [8. Authentication: JWT + Refresh Token + Logout](#8-authentication-jwt--refresh-token--logout)

## 1. Simple Conversion Between Layers
For simple conversions, such as between an Entity and a Response DTO, `From` / `Into` are used.

```rust
#[derive(Debug, Serialize)]
pub struct AccountResponseDto {
    pub id: i64,
    pub email: String,
    pub name: String,
}

impl From<AccountEntity> for AccountResponseDto {
    fn from(entity: AccountEntity) -> Self {
        Self {
            id: entity.id,
            email: entity.email,
            name: entity.name,
        }
    }
}
```

```rust
let entity = self.account_repository.find_by_id(account_id).await?;

Ok(entity.into())
```

The Repository does not directly connect the Service and the DB model; instead, it has Repository-specific Params structs such as `CreateAccountParams` and `UpdateProductParams`.

The Request DTO and the Repository input model are kept separate:
the Repository takes a `Params` as input when saving to the DB,
and returns query results from the DB as an `Entity`.

The Service does not return the Entity as-is to the outside world;
it converts only the fields that are needed into a Response DTO before passing it to the Controller.

 ```rust
#[derive(Debug)]
pub struct CreateAccountParams {
    pub email: String,
    pub password_hash: String,
    pub name: String,
}

impl CreateAccountParams {
    pub fn new(email: String, password_hash: String, name: String) -> Self {
        Self {
            email,
            password_hash,
            name,
        }
    }
}
```

```rust
// Example Service code
let params = CreateAccountParams::new(
    email,
    password_hash,
    name,
);
```


---

## 2. Request Validation

Not only the JSON Body, but also **Path / Query input values are validated through the same validation flow**.

```text
HTTP Request
 ├── Body  ──→ ValidatedJson
 ├── Path  ──→ ValidatedPath
 └── Query ──→ ValidatedQuery
                    │
                    ▼
               DTO Validation
                    │
                    ▼
                 Service
```
### Request Body Example

```http
POST /products
Content-Type: application/json
```

```json
{
  "name": "Keyboard",
  "description": "Mechanical keyboard",
  "price": 10000
}
```

### Path Parameter Example

```http
GET /products/10
```

```text
id = 10
```

### Query Parameter Example

```http
GET /products?page=1&size=20&keyword=keyboard
```

```text
page    = 1
size    = 20
keyword = keyboard
```

DTO validation is based on `validator`, and invalid input is returned as a Korean-language error message through the common `AppError` flow.

---

## 3. SQLx Data Access

Instead of an ORM, SQLx is used to **directly specify the columns and SQL that are needed**.

### `query_as!`
Maps query results onto a specific Rust struct.
The types of the SQL and the result columns are validated at compile time.

```rust
let account = sqlx::query_as!(
    AccountEntity,
    r#"
    SELECT
        id,
        email,
        password_hash,
        name
    FROM accounts
    WHERE id = $1
    "#,
    id
)
.fetch_one(&pool)
.await?;
```

### `query!`
Queries multiple columns into an anonymous result type generated by SQLx.
The types of the SQL and the result columns are validated at compile time.

```rust
let row = sqlx::query!(
    r#"
    SELECT id, name
    FROM products
    WHERE id = $1
    "#,
    id
)
.fetch_one(&pool)
.await?;
```

### `query_scalar!`
Used when the query result returns a **single value**, such as the first column of a result.
It is well suited to queries like `EXISTS`, `COUNT`, or `id`, and applies compile-time validation.

```rust
let exists = sqlx::query_scalar!(
    "SELECT EXISTS(SELECT 1 FROM accounts WHERE email = $1)",
    email
)
.fetch_one(&pool)
.await?;
```
SQLx's Query Macros check the database's SQL results against Rust types
at compile time. This makes it possible to catch an incorrect column name or a
type mismatch in a query before it is ever executed.

---

## 4. Common API Response / Error

`AppError` and `ApiResponse` unify the success/failure response format of the API.

### Success - With Data

```rust
Ok(ApiResponse::success_with_data(
    StatusCode::OK,
    response_body,
))
```

```json
{
  "success": true,
  "status": 200,
  "data": {
    "access_token": "..."
  }
}
```

### Success - With List Data

```rust
Ok(ApiResponse::success_with_list(
    StatusCode::OK,
    products,
    page,
    size,
    count,
    total,
))
```

```json
{
  "success": true,
  "status": 200,
  "data": [],
  "meta": {
    "count": 2,
    "total": 5,
    "page": 1,
    "size": 2,
    "total_pages": 3
  }
}
```

- `count`: The number of items included in the current page
- `total`: The total number of matching results
- `page`: The current page number
- `size`: The number of items per page
- `total_pages`: The total number of pages

### Success - No Data

```rust
Ok(ApiResponse::success(StatusCode::CREATED))
```

```json
{
  "success": true,
  "status": 201
}
```

### Failure

```json
{
  "success": false,
  "status": 401,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "인증이 필요합니다."
  }
}
```

Rather than having each Controller build its own response, responses are unified under a common Response / Error format.

---

## 5. Request ID & Logging

Every request is assigned a unique Request ID so that **the logs produced by a single request can be traced as one continuous flow**.

```text
Client
  │
  ▼
Request ID Middleware
  │
  ├── Controller
  ├── Service
  └── Repository
        │
        ▼
     Terminal Logs
```
Example of an actual log:
- request_id: 907af9f7-c47b-4dd1-a228-1354a50c1650

```text
  2026-08-18T20:27:32.317821Z DEBUG tower_http::trace::on_request: started processing request
    at C:\Users\ohhoj\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\tower-http-0.7.0\src\trace\on_request.rs:80
    in tower_http::trace::make_span::request with method: POST, uri: /accounts/logout, version: HTTP/1.1
    in my_app::common::middleware::request with request_id: 907af9f7-c47b-4dd1-a228-1354a50c1650, method: POST, uri: /accounts/logout

  2026-08-18T20:27:32.318225Z  WARN my_app::domains::account::controller: Logout failed: no refresh_token Cookie present., account_id: 2
    at src\domains\account\controller.rs:157
    in tower_http::trace::make_span::request with method: POST, uri: /accounts/logout, version: HTTP/1.1
    in my_app::common::middleware::request with request_id: 907af9f7-c47b-4dd1-a228-1354a50c1650, method: POST, uri: /accounts/logout

  2026-08-18T20:27:32.318350Z DEBUG tower_http::trace::on_response: finished processing request, latency: 0 ms, status: 401
    at C:\Users\ohhoj\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\tower-http-0.7.0\src\trace\on_response.rs:114
    in tower_http::trace::make_span::request with method: POST, uri: /accounts/logout, version: HTTP/1.1
    in my_app::common::middleware::request with request_id: 907af9f7-c47b-4dd1-a228-1354a50c1650, method: POST, uri: /accounts/logout

  2026-08-18T20:27:32.318478Z  INFO my_app::common::middleware: request completed
    at src\common\middleware.rs:76
    in my_app::common::middleware::request with request_id: 907af9f7-c47b-4dd1-a228-1354a50c1650, method: POST, uri: /accounts/logout

  2026-08-18T20:27:32.318666Z DEBUG tower_http::trace::on_eos: end of stream, stream_duration: 0 ms
    at C:\Users\ohhoj\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\tower-http-0.7.0\src\trace\on_eos.rs:103
    in tower_http::trace::make_span::request with method: POST, uri: /accounts/logout, version: HTTP/1.1
    in my_app::common::middleware::request with request_id: 907af9f7-c47b-4dd1-a228-1354a50c1650, method: POST, uri: /accounts/logout
```

Using the Request ID, you can distinguish and review the logs related to a request from its start to its completion.

In addition, every response includes an `x-request-id` header, and this value can be used to search the server logs for the full processing flow of that request.

```text
Response Header
  x-request-id: 907af9f7-c47b-4dd1-a228-1354a50c1650
```

---

## 6. Redis / Dragonfly Backend Switching

The cache backend can be selected via an environment variable.

```env
CACHE_BACKEND=redis
```

or

```env
CACHE_BACKEND=dragonfly
```

```text
              Application
                   │
                   ▼
              RedisClient
              │         │
              ▼         ▼
           Redis     Dragonfly
```

Because a common `RedisClient` is used, the Domain / Service code does not distinguish directly between Redis and Dragonfly.

In other words, **when switching between Redis and Dragonfly, the application logic can remain unchanged and only the backend configuration needs to be switched**.

In the current development/test environment, persistence for Redis and Dragonfly is turned off and `tmpfs` is used, so that stored authentication sessions are reset whenever the container restarts.

---

## 7. Role + Permission

Role and Permission are separated, and `secured_route!` is used so that **the permissions required by a route can be declared right where the route is defined**.

```text
                       Route
                         │
                ┌────────┴────────┐
                ▼                 ▼
         secured_route!       regular route
                │                 │
         Permission check      no check
                │                 │
           allow / deny         Handler
```
### APIs That Require Permission
- `GET /products` requires the Product(GetProducts) permission
- `POST /products` requires the Product(CreateProduct) permission
```rust
.route(
    "/",
    secured_route!(
        state.jwt.clone();
        get(controller::get_products) => Product(GetProducts),
        post(controller::create_product) => Product(CreateProduct),
    ),
)
```

### APIs That Do Not Require Permission
- `POST /accounts/logout` does not use a route-level Permission check.
- However, Access Token authentication is still performed via `AuthenticatedUser`.
```rust
.route("/logout", post(controller::logout))
```

The structure is set up so that you can tell whether an API uses a permission check just by looking at the route definition.

### Permission List

| Domain | Permission | Description |
|---|---|---|
| Account | `account:get_me` | Retrieve the currently logged-in account |
| Product | `product:create_product` | Create a product |
| Product | `product:get_product` | Retrieve a single product |
| Product | `product:get_products` | Retrieve a list of products |
| Product | `product:put_product` | Fully update a product |
| Product | `product:patch_product` | Partially update a product |
| Product | `product:delete_product` | Delete a product |

---

## 8. Authentication: JWT + Refresh Token + Logout

A dual-token structure is used, separating the Access Token from the Refresh Token.

```text
                         Login
                           │
               ┌───────────┴───────────┐
               ▼                       ▼
         Access Token            Refresh Token
             JWT                    Random Token
               │                       │
               ▼                       ▼
        JSON Response          HttpOnly Cookie
                                       │
                                       ▼
                                SHA-256 Hash
                                       │
                                       ▼
                                Redis / Dragonfly
```

### Access Token

- JWT-based stateless authentication
- Delivered in the JSON response body
- Uses a short expiration time (e.g., 15 minutes)

### Refresh Token

- An `HttpOnly + Secure + SameSite=Strict` cookie
- The raw Refresh Token is never stored in Redis
- Stored in Redis as a SHA-256 hash for session management
- Uses a long expiration time (e.g., 3 days)

### Why Reissuing a Token Requires Both the Expired Access Token and a Valid Refresh Token

The Access Token is not reissued using the Refresh Token alone;
the signature and expiration state of the existing Access Token are also checked.

In addition, it is verified that the Access Token's `sub` matches the Refresh Token's `account_id`,
confirming that both tokens belong to the same account.

### Refresh Token Rotation

When a refresh succeeds, the existing Refresh Token is discarded and replaced with a new Refresh Token.

This prevents the same Refresh Token from being reused repeatedly to keep reissuing Access Tokens.

```text
Expired Access Token + R1
        │
        ▼
       200
        │
        ▼
New Access Token + R2

R1 reused
   │
   ▼
  401
```

Rotation is a structure designed to **prevent repeated reissue requests using the same Refresh Token, and to block reuse of a Refresh Token that has already been used**.

### Logout

On logout, the current Refresh Session is deleted from Redis and the Refresh cookie is removed.

> Because the Access Token is a stateless JWT, an already-issued Access Token can remain valid until it expires, even immediately after logout. Logout works by cutting off the Refresh Session, which blocks any further reissuing of Access Tokens.

---

## Domain Examples

To serve as a reference when adding a new Domain, two Domains with different purposes have been implemented.

### Product Domain

```text
Product
 ├── CRUD
 ├── Pagination
 └── Validation
```

This can be used as a **baseline example of a typical REST API structure**.

### Account Domain

```text
Account
 ├── Sign-up
 ├── Login
 ├── JWT
 ├── Refresh
 └── Logout
```

This can be used as a **baseline example of an API structure that requires authentication/authorization**.

So, when adding a new API:

```text
Regular REST API
    ↓
See Product

API requiring authentication
    ↓
See Account
```

---

## Tech Stack

| Category | Technology |
|---|---|
| Language | Rust |
| Web Framework | Axum |
| Async Runtime | Tokio |
| Serialization | Serde |
| Validation | validator |
| Database | PostgreSQL |
| DB Toolkit | SQLx |
| Authentication | JWT / jsonwebtoken |
| Password Hashing | Argon2 |
| Cache / Session Store | Redis / Dragonfly |
| Token Hashing | SHA-256 |
| Logging | tracing / tracing-subscriber |
| HTTP Tracing | tower-http |
| Cookie | axum-extra |
| Environment | dotenvy |

---

## Project Structure

```text
.
├── .github/
│   └── workflows/
│       └── ci.yml               ## GitHub Actions CI workflow file
│
├── docker/
│   ├── postgres/
│   │   └── init/
│   │       ├── 001_init.sql
│   │       ├── 002_seed.sql
│   │       └── 003_test_init.sql
│   ├── redis/
│   └── dragonfly/
│
├── docs/
│   ├── API.md                  ## API specification
│   └── DEVELOPMENT.md          ## Development environment / setup guide
│
├── src/
│   ├── main.rs        # Application entry point and server initialization
│   ├── lib.rs         # Library module composition (used in tests)
│   ├── state.rs       # Definition of the application's shared state
│   ├── common/               # Functionality shared across multiple domains
│   │   ├── mod.rs
│   │   ├── dto.rs            ## Common DTO definitions
│   │   ├── error.rs          ## Common application error definitions and HTTP response conversion
│   │   ├── extractors.rs     ## Extractors for validating request Body / Path / Query
│   │   ├── fallback.rs       ## Fallback handling for routes that do not exist
│   │   ├── macros.rs         ## Common macro definitions
│   │   ├── middleware.rs     ## Request ID and common middleware
│   │   ├── redis.rs          ## Common Redis / Dragonfly client
│   │   ├── response.rs       ## Common API response handling
│   │   ├── auth/                # Authentication, authorization, and JWT-related functionality
│   │   │   ├── mod.rs
│   │   │   ├── authorization.rs ## Permission-based authorization handling
│   │   │   ├── extractor.rs     ## JWT authentication and the AuthenticatedUser extractor
│   │   │   ├── jwt.rs           ## JWT generation and verification
│   │   │   ├── permission.rs    ## API permission definitions
│   │   │   ├── refresh_token.rs ## Refresh Token generation / storage / rotation
│   │   │   └── role.rs          ## Account role definitions
│   │   │
│   │   └── security/            # Security functionality such as passwords
│   │       ├── mod.rs
│   │       └── password.rs      ## Argon2 password hashing / verification
│   │
│   └── domains/                 # Per-domain functionality modules
│       ├── mod.rs
│       ├── account/
│       │   ├── mod.rs            ## Account router configuration
│       │   ├── controller.rs     ## Account HTTP handlers
│       │   ├── dto.rs            ## Account request / response DTOs
│       │   ├── entity.rs         ## Account DB query model
│       │   ├── repository.rs     ## Account DB access
│       │   └── service.rs        ## Account business logic
│       │
│       └── product/
│           ├── mod.rs
│           ├── controller.rs
│           ├── dto.rs
│           ├── entity.rs
│           ├── repository.rs
│           └── service.rs
│
├── tests/                      # Integration tests
│   ├── api.rs                  ## Test app / DB initialization
│   ├── account/
│   │   ├── mod.rs
│   │   ├── create_account.rs
│   │   ├── get_me.rs
│   │   ├── login.rs
│   │   ├── logout.rs
│   │   └── refresh.rs
│   │
│   └── product/
│       ├── mod.rs
│       ├── create_product.rs
│       ├── delete_product.rs
│       ├── get_product.rs
│       ├── get_products.rs
│       ├── patch_product.rs
│       └── put_product.rs
│
├── Cargo.toml
├── Cargo.lock
├── docker-compose.yml
├── rustfmt.toml
├── LICENSE
├── .env
├── .env.test
├── .gitignore               ## List of files/folders excluded from git tracking
└── .gitattributes           ## Configuration file defining how git handles each file (line endings, merge strategy, etc.)
```

---

## Possible Extensions

Building on the current project's basic structure, it can be extended with the following features.

- **OpenAPI / Swagger API Documentation**
  - Document endpoints via an API schema and Swagger UI

- **Making the Refresh Token Expiration Time Configurable via Environment Variable**
  - Manage `REFRESH_TOKEN_EXPIRES_IN_SECONDS` from `.env`
  - Set it up so the Refresh Token expiration time can be changed per environment

- **In-Memory Rate Limiting in Rust**
  - Limit request counts using process memory instead of Redis
  - Control request frequency per IP or per API
  - An example implementation of rate limiting that works in a single-server environment

- **Improving Concurrency in Refresh Token Rotation**
  - The current rotation process separates `GET → verify → update` into distinct steps,
    so a race condition can occur if requests with the same Refresh Token arrive at the same time
  - In the future, a Redis Lua script could be used to perform Refresh Token verification and rotation as a single atomic operation

```text
Request A                  Request B
    │                          │
    ├─ GET old_hash            │
    │                          ├─ GET old_hash
    │                          │
    ├─ verification succeeds   ├─ verification succeeds
    │                          │
    ├─ save new_token_A        ├─ save new_token_B
    │                          │
    └──────── Race Condition ──┘

[Solution]
    Rust application
    │
    ├─ src/common/redis/rotate_refresh_token.lua
    │
    │  at application startup
    ▼
Redis SCRIPT LOAD
    │
    └─ returns SHA
          ↓
     cache the script SHA
          ↓
       EVALSHA
```
