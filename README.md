# Rust Axum REST API

```text
┌──────────────────────────────────────┐
│        RUST · AXUM · REST API        │
│     Layered Architecture Example     │
└──────────────────────────────────────┘
```

Rust + Axum 기반 REST API 프로젝트입니다.

NestJS / Spring Boot에서 익숙한 계층형 서버 구조를 Rust/Axum에 적용하고, 입력 검증, 공통 응답/에러 처리, Role/Permission 기반 인가, JWT 인증, Refresh Token Rotation, Redis/Dragonfly 세션 관리까지 직접 구성했습니다.

> **Project goal**  
> 단순 CRUD 예제를 넘어, 새로운 API를 추가할 때 참고할 수 있는 Rust/Axum 서버의 기본 구조와 인증/인가 패턴을 하나의 프로젝트에서 정리하는 것을 목표로 합니다.

> **Project context**
>
> - Rust 언어 학습: 약 1개월
> - Axum 학습 및 프로젝트 개발: 약 3주
> - 개발 방식: AI와 지속적으로 설계·구현 방향을 논의하며 진행한 바이브 코딩
> - 사용 AI 도구(무료 버전): ChatGPT, Grok, Claude, Google AI Search 엔진
> - 사용 AI 코딩 도구: Codex (약 3일)
> - 프로젝트 성격: Rust/Axum 학습 과정에서 직접 설계·구현하며 서버 기본기를 익힌 개인 학습 프로젝트
> - 현재 수준: Rust/Axum 숙련 개발자의 완성품이 아닌 학습 단계의 프로젝트
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

### 계층 역할

- `controller`: HTTP 요청을 받고 응답을 반환합니다.
- `dto`: 요청·응답 데이터와 입력값 검증 규칙을 정의합니다.
- `service`: 도메인의 비즈니스 규칙을 처리합니다.
- `repository`: PostgreSQL과 직접 통신합니다.
- `params`: Repository에 전달하는 데이터베이스 입력값 모델입니다.
- `entity`: 데이터베이스 행과 매핑되는 모델입니다.
- `common`: 인증, 오류, 응답, 미들웨어처럼 여러 도메인이 공유하는 기능입니다.

### 인증 흐름

1. 로그인 성공 시 Access Token을 JSON으로 반환합니다.
2. Refresh Token은 HttpOnly Cookie로 저장합니다.
3. 보호된 라우트는 `secured_route!`를 통해 JWT 인증과 권한 검사를 적용합니다.
4. Refresh Token 원문은 저장하지 않고 해시하여 Redis/Dragonfly에 저장합니다.

---
## 특징

- [1. 계층 간 단순 변환](#1-계층-간-단순-변환)
- [2. Request Validation](#2-request-validation)
- [3. SQLx Data Access](#3-sqlx-data-access)
- [4. Common API Response / Error](#4-common-api-response--error)
- [5. Request ID & Logging](#5-request-id--logging)
- [6. Redis / Dragonfly Backend Switching](#6-redis--dragonfly-backend-switching)
- [7. Role + Permission](#7-role--permission)
- [8. Authentication: JWT + Refresh Token + Logout](#8-authentication-jwt--refresh-token--logout)

## 1. 계층 간 단순 변환
Entity와 Response DTO처럼 단순한 변환은 `From` / `Into`를 사용합니다.

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

Repository에는 Service와 DB 모델을 직접 연결하지 않고 `CreateAccountParams`, `UpdateProductParams` 같은 Repository 전용 Params 구조체를 둡니다.

Request DTO와 Repository 입력 모델을 분리하여
Repository는 DB에 저장할 때 `Params`를 입력으로 받고,
DB에서 조회한 결과는 `Entity`로 반환합니다.

Service는 Entity를 그대로 외부에 반환하지 않고,
필요한 필드만 Response DTO로 변환하여 Controller에 전달합니다.

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
// Service 코드 예시
let params = CreateAccountParams::new(
    email,
    password_hash,
    name,
);
```


---

## 2. Request Validation

JSON Body뿐 아니라 **Path / Query 입력값까지 동일한 Validation 흐름으로 검증**합니다.

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
### Request Body 예시

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

### Path Parameter 예시

```http
GET /products/10
```

```text
id = 10
```

### Query Parameter 예시

```http
GET /products?page=1&size=20&keyword=keyboard
```

```text
page    = 1
size    = 20
keyword = keyboard
```

`validator` 기반 DTO 검증을 사용하고, 잘못된 입력은 공통 `AppError` 흐름을 통해 한글 오류 메시지로 반환합니다.

---

## 3. SQLx Data Access

ORM 대신 SQLx를 사용하여 **필요한 컬럼과 SQL을 직접 지정**합니다.

### `query_as!`
조회 결과를 특정 Rust 구조체에 매핑합니다.
컴파일 시점에 SQL과 결과 컬럼의 타입을 검증합니다.

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
여러 컬럼을 SQLx가 생성한 익명 결과 타입으로 조회합니다.
컴파일 시점에 SQL과 결과 컬럼의 타입을 검증합니다.

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
조회 결과의 첫 번째 컬럼처럼 **단일 값**을 반환할 때 사용합니다.
`EXISTS`, `COUNT`, `id` 등의 조회에 적합하며 컴파일 시점 검증을 적용합니다.

```rust
let exists = sqlx::query_scalar!(
    "SELECT EXISTS(SELECT 1 FROM accounts WHERE email = $1)",
    email
)
.fetch_one(&pool)
.await?;
```
SQLx의 Query Macro는 데이터베이스의 SQL 결과와 Rust 타입을
컴파일 시점에 확인합니다. 따라서 잘못된 컬럼명이나 타입이 맞지 않는
쿼리를 실행 전에 발견할 수 있습니다.

---

## 4. Common API Response / Error

`AppError`와 `ApiResponse`를 통해 API의 성공/실패 응답 형식을 통일했습니다.

### 성공 - Data 포함

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

### 성공 - 목록 Data 포함

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

- `count`: 현재 페이지에 포함된 데이터 개수
- `total`: 전체 검색 결과 개수
- `page`: 현재 페이지 번호
- `size`: 페이지당 데이터 개수
- `total_pages`: 전체 페이지 수

### 성공 - Data 없음

```rust
Ok(ApiResponse::success(StatusCode::CREATED))
```

```json
{
  "success": true,
  "status": 201
}
```

### 실패

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

Controller마다 제각각 응답을 만들지 않고 공통 Response / Error 규격으로 통일합니다.

---

## 5. Request ID & Logging

모든 요청에 고유한 Request ID를 부여해 **동일 요청에서 발생한 로그를 하나의 흐름으로 추적**할 수 있도록 구성했습니다.

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
실제 로그 예시:
- request_id: 907af9f7-c47b-4dd1-a228-1354a50c1650

```text
  2026-08-18T20:27:32.317821Z DEBUG tower_http::trace::on_request: started processing request
    at C:\Users\ohhoj\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\tower-http-0.7.0\src\trace\on_request.rs:80
    in tower_http::trace::make_span::request with method: POST, uri: /accounts/logout, version: HTTP/1.1
    in my_app::common::middleware::request with request_id: 907af9f7-c47b-4dd1-a228-1354a50c1650, method: POST, uri: /accounts/logout

  2026-08-18T20:27:32.318225Z  WARN my_app::domains::account::controller: Logout 실패: refresh_token Cookie가 없습니다., account_id: 2
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

Request ID를 기준으로 요청의 시작부터 종료까지 관련 로그를 구분해서 확인할 수 있습니다.

또한, 모든 응답에는 `x-request-id` 헤더가 포함되며, 이 값으로 서버 로그에서 해당 요청의 전체 처리 흐름을 검색할 수 있습니다.

```text
Response Header
  x-request-id: 907af9f7-c47b-4dd1-a228-1354a50c1650
```

---

## 6. Redis / Dragonfly Backend Switching

Cache Backend는 환경변수로 선택할 수 있습니다.

```env
CACHE_BACKEND=redis
```

또는

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

공통 `RedisClient`를 사용하기 때문에 Domain / Service 코드는 Redis와 Dragonfly를 직접 구분하지 않습니다.

즉 **Redis ↔ Dragonfly 변경 시 애플리케이션 로직은 그대로 두고 Backend 구성만 전환**할 수 있습니다.

현재 개발/테스트 환경에서는 Redis와 Dragonfly의 persistence를 끄고 `tmpfs`를 사용하여 컨테이너 재시작 시 저장된 인증 세션이 초기화되도록 구성했습니다.

---

## 7. Role + Permission

Role과 Permission을 분리하고, `secured_route!`를 통해 **Route 정의에서 필요한 권한을 선언**할 수 있도록 구성했습니다.

```text
                       Route
                         │
                ┌────────┴────────┐
                ▼                 ▼
         secured_route!       일반 route
                │                 │
         Permission 검사        검사 없음
                │                 │
           허용 / 거부          Handler
```
### 권한이 필요한 API
- `GET /products`는 Product(GetProducts) 권한 필요
- `POST /products`는 Product(CreateProduct) 권한 필요
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

### 권한이 필요하지 않은 API
- `POST /accounts/logout`은 Route 단위 Permission 검사를 사용하지 않습니다.
- 단, `AuthenticatedUser`를 통해 Access Token 인증은 수행합니다.
```rust
.route("/logout", post(controller::logout))
```

Route 정의만 보더라도 해당 API가 권한 검사를 사용하는지 확인할 수 있도록 구성했습니다.

### Permission 목록

| Domain | Permission | 설명 |
|---|---|---|
| Account | `account:get_me` | 현재 로그인한 계정 조회 |
| Product | `product:create_product` | 상품 생성 |
| Product | `product:get_product` | 상품 단건 조회 |
| Product | `product:get_products` | 상품 목록 조회 |
| Product | `product:put_product` | 상품 전체 수정 |
| Product | `product:patch_product` | 상품 일부 수정 |
| Product | `product:delete_product` | 상품 삭제 |

---

## 8. Authentication: JWT + Refresh Token + Logout

Access Token과 Refresh Token을 분리한 이중 토큰 구조를 사용합니다.

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

- JWT 기반 Stateless 인증
- JSON Response Body로 전달
- 짧은 만료시간 사용(예: 15분)

### Refresh Token

- `HttpOnly + Secure + SameSite=Strict` Cookie
- Refresh Token 원문을 Redis에 저장하지 않음
- SHA-256 hash 기반으로 Redis에 저장하여 Session 관리
- 긴 만료시간 사용(예: 3일)

### 토큰 재발급 시 만료된 Access Token + 유효한 Refresh Toekn을 함께 요구하는 이유

Refresh Token만으로 Access Token을 재발급하지 않고,
기존 Access Token의 서명과 만료 상태를 확인합니다.

또한 Access Token의 `sub`와 Refresh Token의 `account_id`가 일치하는지 검증하여
두 토큰이 동일한 계정에 속하는지 확인합니다.

### Refresh Token Rotation

Refresh가 성공하면 기존 Refresh Token을 폐기하고 새로운 Refresh Token으로 교체합니다.

이를 통해 동일한 Refresh Token을 반복 재사용하여 Access Token을 계속 재발급하는 것을 차단합니다.

```text
만료된 Access Token + R1
        │
        ▼
       200
        │
        ▼
새 Access Token + R2

R1 재사용
   │
   ▼
  401
```

Rotation은 **동일 Refresh Token으로 반복적인 재발급 요청을 계속하는 것을 방지하고, 이미 사용된 Refresh Token의 재사용을 차단**하기 위한 구조입니다.

### Logout

Logout 시 현재 Refresh Session을 Redis에서 삭제하고 Refresh Cookie를 제거합니다.

> Access Token은 Stateless JWT이므로 Logout 직후에도 이미 발급된 Access Token이 만료되기 전까지는 유효할 수 있습니다. Logout에서는 Refresh Session을 끊어 이후의 Access Token 재발급을 차단합니다.

---

## Domain Examples

새로운 Domain을 추가할 때 참고할 수 있도록 서로 다른 목적의 두 Domain을 구현했습니다.

### Product Domain

```text
Product
 ├── CRUD
 ├── Pagination
 └── Validation
```

**일반적인 REST API 구조의 기준 예시**로 사용할 수 있습니다.

### Account Domain

```text
Account
 ├── 회원가입
 ├── Login
 ├── JWT
 ├── Refresh
 └── Logout
```

**인증 / 인가가 필요한 API 구조의 기준 예시**로 사용할 수 있습니다.

따라서 새로운 API를 추가할 때:

```text
일반 REST API
    ↓
Product 참고

인증이 필요한 API
    ↓
Account 참고
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
│       └── ci.yml               ## GitHub Actions CI 실행 파일
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
│   ├── API.md                  ## API 명세
│   └── DEVELOPMENT.md          ## 개발 환경 / 실행 가이드
│
├── src/
│   ├── main.rs        # Application 시작 및 Server 초기화
│   ├── lib.rs         # Library Module 구성(테스트에서 사용)
│   ├── state.rs       # Application 공유 State 정의
│   ├── common/               # 여러 도메인에서 공통으로 사용하는 기능
│   │   ├── mod.rs
│   │   ├── dto.rs            ## 공통 DTO 정의
│   │   ├── error.rs          ## 애플리케이션 공통 에러 정의 및 HTTP 응답 변환
│   │   ├── extractors.rs     ## 요청 Body / Path / Query 검증 Extractor
│   │   ├── fallback.rs       ## 존재하지 않는 Route fallback 처리
│   │   ├── macros.rs         ## 공통 Macro 정의
│   │   ├── middleware.rs     ## Request ID 및 공통 Middleware
│   │   ├── redis.rs          ## Redis / Dragonfly 공통 Client
│   │   ├── response.rs       ## 공통 API Response 처리
│   │   ├── auth/                # 인증·인가·JWT 관련 기능
│   │   │   ├── mod.rs
│   │   │   ├── authorization.rs ## Permission 기반 인가 처리
│   │   │   ├── extractor.rs     ## JWT 인증 및 AuthenticatedUser Extractor
│   │   │   ├── jwt.rs           ## JWT 생성 및 검증
│   │   │   ├── permission.rs    ## API Permission 정의
│   │   │   ├── refresh_token.rs ## Refresh Token 생성 / 저장 / Rotation
│   │   │   └── role.rs          ## Account Role 정의
│   │   │
│   │   └── security/            # 비밀번호 등 보안 기능
│   │       ├── mod.rs
│   │       └── password.rs      ## Argon2 비밀번호 해싱 / 검증
│   │
│   └── domains/                 # 도메인별 기능 모듈
│       ├── mod.rs
│       ├── account/
│       │   ├── mod.rs            ## Account 라우터 구성
│       │   ├── controller.rs     ## Account HTTP Handler
│       │   ├── dto.rs            ## Account Request / Response DTO
│       │   ├── entity.rs         ## Account DB 조회 모델
│       │   ├── repository.rs     ## Account DB 접근
│       │   └── service.rs        ## Account 비즈니스 로직
│       │
│       └── product/
│           ├── mod.rs
│           ├── controller.rs
│           ├── dto.rs
│           ├── entity.rs
│           ├── repository.rs
│           └── service.rs
│
├── tests/                      # 통합 테스트
│   ├── api.rs                  ## 테스트용 앱·DB 초기화
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
└── .gitignore
```

---

## 확장 가능한 예제

현재 프로젝트의 기본 구조를 기반으로 다음과 같은 기능을 추가해 확장할 수 있습니다.

- **OpenAPI / Swagger API 문서**
  - API Schema 및 Swagger UI를 통해 Endpoint 문서화

- **Refresh Token 만료시간 환경변수화**
  - `REFRESH_TOKEN_EXPIRES_IN_SECONDS`를 `.env`에서 관리
  - 환경별 Refresh Token 만료시간을 변경할 수 있도록 구성

- **Rust 메모리 기반 Rate Limit**
  - Redis 없이 프로세스 메모리를 이용한 요청 횟수 제한
  - IP 또는 API 단위로 요청 빈도를 제어
  - 단일 서버 환경에서 동작하는 Rate Limit 구현 예제

- **Refresh Token Rotation 동시성 개선**
  - 현재 Rotation은 `GET → 검증 → 변경` 과정이 분리되어 있어
    동시에 동일한 Refresh Token으로 요청이 들어올 경우 Race Condition이 발생할 수 있음
  - 향후 Redis Lua Script를 사용하여 Refresh Token 검증과 Rotation을 하나의 원자적 작업으로 처리 가능

```text
Request A                  Request B
    │                          │
    ├─ GET old_hash            │
    │                          ├─ GET old_hash
    │                          │
    ├─ 검증 성공                ├─ 검증 성공
    │                          │
    ├─ new_token_A 저장         ├─ new_token_B 저장
    │                          │
    └──────── Race Condition ──┘

[Solution]
    Rust 애플리케이션
    │
    ├─ src/common/redis/rotate_refresh_token.lua
    │
    │  애플리케이션 초기화 시
    ▼
Redis SCRIPT LOAD
    │
    └─ SHA 반환
          ↓
     Script SHA 캐싱
          ↓
       EVALSHA
```
