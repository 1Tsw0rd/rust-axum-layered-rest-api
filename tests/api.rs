use axum::{body::Body, http::{Request, Response}, Router};
use http_body_util::BodyExt;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower::ServiceExt;

// 외부 통합 테스트 파일이므로 crate:: 대신 내 프로젝트 패키지 이름으로 시작
use rust_axum_layered_rest_api::domains::{
    account::router as account_router,
    product::router as product_router,
};

use rust_axum_layered_rest_api::common::auth::jwt::JwtConfig;
use rust_axum_layered_rest_api::common::auth::role::AccountRole;
use rust_axum_layered_rest_api::common::fallback::not_found;
use rust_axum_layered_rest_api::common::redis::RedisClient;
use rust_axum_layered_rest_api::state::AppState;

pub mod product;
pub mod account;

async fn cleanup_test_db(pool: &PgPool) {
    sqlx::query!(
        r#"
        TRUNCATE TABLE accounts, products
        RESTART IDENTITY CASCADE
        "#
    )
        .execute(pool)
        .await
        .expect("테스트 DB 초기화 실패");
}

async fn cleanup_test_redis(redis: &RedisClient) {
    redis
        .atomic_pipeline(|pipeline| {
            pipeline.cmd("FLUSHDB").ignore();
        })
        .await
        .expect("테스트 Redis 초기화 실패");
}

// 앱 조립만 담당: DB pool 연결 + 항상 clean DB 보장 + Redis 연결 + 항상 clean Redis 보장
// 테스트 데이터(seed_test_products)만 각 테스트가 필요할 때 명시적으로 호출한다
pub async fn test_app() -> (Router, PgPool, RedisClient) {
    dotenvy::from_filename(".env.test").ok();

    // PostgreSQL 연결 설정
    let test_db_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL 추출 실패");

    let test_db_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&test_db_url)
        .await
        .expect("연결 실패");

    // 테스트 시작 전에 항상 DB 초기화
    cleanup_test_db(&test_db_pool).await;

    // Redis / Dragonfly 설정
    let cache_backend = std::env::var("CACHE_BACKEND")
        .expect("CACHE_BACKEND 환경변수 추출 실패");

    let redis_url = match cache_backend.as_str() {
        "redis" => {
            let host = std::env::var("REDIS_HOST")
                .expect("REDIS_HOST 환경변수 추출 실패");
            let port = std::env::var("REDIS_PORT")
                .expect("REDIS_PORT 환경변수 추출 실패");
            let password = std::env::var("REDIS_PASSWORD")
                .expect("REDIS_PASSWORD 환경변수 추출 실패");
            let db = std::env::var("REDIS_DB")
                .unwrap_or_else(|_| "0".to_string());

            format!("redis://:{}@{}:{}/{}", password, host, port, db)
        }
        "dragonfly" => {
            let host = std::env::var("DRAGONFLY_HOST")
                .expect("DRAGONFLY_HOST 환경변수 추출 실패");
            let port = std::env::var("DRAGONFLY_PORT")
                .expect("DRAGONFLY_PORT 환경변수 추출 실패");
            let password = std::env::var("DRAGONFLY_PASSWORD")
                .expect("DRAGONFLY_PASSWORD 환경변수 추출 실패");
            let db = std::env::var("DRAGONFLY_DB")
                .unwrap_or_else(|_| "0".to_string());

            format!("redis://:{}@{}:{}/{}", password, host, port, db)
        }
        _ => panic!("지원하지 않는 CACHE_BACKEND입니다: {}", cache_backend),
    };

    let redis = redis::Client::open(redis_url)
        .expect("테스트 Redis Client 초기화 실패");

    let redis_client = RedisClient::new(redis)
        .await
        .expect("테스트 Redis 연결 초기화 실패");

    // 테스트용 Redis/Dragonfly DB를 통째로 비움
    cleanup_test_redis(&redis_client).await;

    // 테스트용 JWT 설정 생성
    let jwt = JwtConfig::new("test-secret-key-for-test", 3600);

    // 테스트용 AppState 구성
    let state = AppState {
        postgres: test_db_pool.clone(),
        jwt,
        redis: redis_client.clone(),
    };

    // 테스트용 Router 구성
    let app = Router::new()
        .nest("/products", product_router(state.clone()))
        .nest("/accounts", account_router(state))
        .fallback(not_found);

    (app, test_db_pool, redis_client)
}

pub async fn seed_test_products(pool: &PgPool) {
    for (name, description, price) in [
        ("Product Keyboard A", "테스트용 상품 설명", 1000_i64),
        ("Product Keyboard B", "테스트용 상품 설명", 2000_i64),
        ("Product Keyboard C", "테스트용 상품 설명", 3000_i64),
        ("Radio Device", "테스트용 상품 설명", 4000_i64),
        ("Mouse Pad", "테스트용 상품 설명", 5000_i64),
    ] {
        sqlx::query("INSERT INTO products (name, description, price) VALUES ($1, $2, $3)")
            .bind(name)
            .bind(description)
            .bind(price)
            .execute(pool)
            .await
            .expect("테스트 상품 생성 실패");
    }
}

pub async fn response_json(response: Response<Body>) -> serde_json::Value {
    let body = response
        .into_body()
        .collect()
        .await
        .expect("응답 Body 수집 실패")
        .to_bytes();
    serde_json::from_slice(&body).expect("응답 JSON 파싱 실패")
}

pub fn json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("테스트 요청 생성 실패")
}

pub fn empty_request(method: &str, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .expect("테스트 요청 생성 실패")
}

pub fn authorized_request(
    method: &str,
    uri: &str,
    token: &str,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("인증 요청 생성 실패")
}

pub fn authorized_json_request(
    method: &str,
    uri: &str,
    token: &str,
    body: serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("인증 JSON 요청 생성 실패")
}

pub fn test_token(roles: Vec<AccountRole>) -> String {
    JwtConfig::new("test-secret-key-for-test", 3600)
        .create_access_token(1, roles)
        .expect("테스트 JWT 생성 실패")
}

pub async fn register_account(app: &Router, email: &str, role: &str) {
    let request = json_request(
        "POST",
        "/accounts",
        serde_json::json!({
            "email": email,
            "password": "Password123!",
            "name": "테스트 계정",
            "role": role
        }),
    );
    let response = app.clone().oneshot(request).await.expect("회원가입 요청 실패");
    assert_eq!(response.status(), axum::http::StatusCode::CREATED);
}

pub async fn login_account(app: &Router, email: &str) -> (String, String) {
    let request = json_request(
        "POST",
        "/accounts/login",
        serde_json::json!({
            "email": email,
            "password": "Password123!"
        }),
    );
    let response = app.clone().oneshot(request).await.expect("로그인 요청 실패");
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let cookie = response
        .headers()
        .get("set-cookie")
        .expect("로그인 응답에 Refresh Cookie가 없음")
        .to_str()
        .expect("Set-Cookie 변환 실패")
        .split(';')
        .next()
        .expect("Refresh Cookie 파싱 실패")
        .to_owned();
    let body = response_json(response).await;
    let token = body["data"]["access_token"]
        .as_str()
        .expect("로그인 응답에 Access Token이 없음")
        .to_owned();

    (token, cookie)
}
