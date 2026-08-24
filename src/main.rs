// common, domains, state는 lib.rs에서 이미 컴파일되므로
// main.rs에서 다시 mod로 선언하지 않고 lib 크레이트에서 그대로 가져다 사용함
// tests/에서도 lib 크레이트를 사용하므로 lib.rs는 필요함
use rust_axum_layered_rest_api::{common, domains, state};

use axum::Router;
use common::fallback::not_found;
use common::middleware::request_id_middleware;
use tower_http::trace::TraceLayer;

use common::auth::jwt::JwtConfig;
use common::redis::RedisClient;
use sqlx::postgres::PgPoolOptions;
use state::AppState;

const RUST_LOG_DEFAULT: &str = "my_app=debug,tower_http=debug,sqlx=warn";

#[tokio::main]
async fn main() {
    // .env 로드
    dotenvy::dotenv().expect(".env 파일 로드에 실패");
    // dotenvy::from_filename(".env.production").expect(".env.development 파일 로드에 실패");

    // 로깅 설정
    if std::env::var("RUST_LOG").is_err() {
        // RUST_LOG 환경변수가 세팅되어 있지 않다면 수동 설정
        unsafe {
            // Rust 컴파일러가 안전성을 보장할 수 없는 환경변수 변경 작업이므로 unsafe 필요
            // 기본 로그 설정값을 RUST_LOG 환경변수에 주입
            std::env::set_var("RUST_LOG", RUST_LOG_DEFAULT);
        } // 로그 레벨 error -> warn -> info -> debug -> trace
    }
    // 로그 시스템 초기화
    tracing_subscriber::fmt()
        .pretty() // 가독성 있게 해줌
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env()) // 주입된 RUST_LOG 환경변수 값을 실제로 읽어서 모듈별 로그 레벨을 적용, 설정하지 않으면 tracing 기본 필터(ERROR 레벨)만 출력
        .with_file(true) // 로그에 파일명 출력 활성화
        .with_line_number(true) // 로그에 에러 발생 위치 출력 활성화
        .init();

    // DB 설정
    let db_url = std::env::var("DATABASE_URL").expect(".env 내 DATABASE_URL 추출 실패");

    tracing::info!("[DB CONNECTING] PostgreSQL 서버에 연결 시도합니다...");
    let db_pool = PgPoolOptions::new()
        .max_connections(5) // 허용 최대 커넥션 수
        .connect(&db_url)
        .await
        .expect("PostgreSQL 서버 연결에 실패했습니다, 상태를 점검해보세요.");
    tracing::info!("[DB CONNECTED] PostgreSQL 서버 연결 성공");

    // JWT 설정
    let jwt = JwtConfig::from_env().expect("JWT 설정 초기화 실패");
    tracing::info!("[JWT] 설정 초기화 성공");

    // Redis / Dragonfly 설정: 여기서 Redis와 Dragonfly 중에 운영할 대상 선택함
    let cache_backend = std::env::var("CACHE_BACKEND").expect("CACHE_BACKEND 환경변수 추출 실패");

    let redis_url = match cache_backend.as_str() {
        "redis" => {
            let host = std::env::var("REDIS_HOST").expect("REDIS_HOST 환경변수 추출 실패");
            let port = std::env::var("REDIS_PORT").expect("REDIS_PORT 환경변수 추출 실패");
            let password =
                std::env::var("REDIS_PASSWORD").expect("REDIS_PASSWORD 환경변수 추출 실패");
            let db = std::env::var("REDIS_DB").unwrap_or_else(|_| "0".to_string()); // 기본값 0 (미설정 시 기존 동작 유지)
            format!("redis://:{}@{}:{}/{}", password, host, port, db)
        }
        "dragonfly" => {
            let host = std::env::var("DRAGONFLY_HOST").expect("DRAGONFLY_HOST 환경변수 추출 실패");
            let port = std::env::var("DRAGONFLY_PORT").expect("DRAGONFLY_PORT 환경변수 추출 실패");
            let password =
                std::env::var("DRAGONFLY_PASSWORD").expect("DRAGONFLY_PASSWORD 환경변수 추출 실패");
            let db = std::env::var("DRAGONFLY_DB").unwrap_or_else(|_| "0".to_string());
            format!("redis://:{}@{}:{}/{}", password, host, port, db)
        }
        _ => panic!("지원하지 않는 CACHE_BACKEND입니다: {}", cache_backend),
    };

    let redis = redis::Client::open(redis_url).expect("Redis/Dragonfly Client 초기화 실패");

    let redis_client = RedisClient::new(redis)
        .await
        .expect("Redis/Dragonfly 연결 초기화 실패");

    redis_client
        .ping()
        .await
        .expect("Redis/Dragonfly 연결 확인 실패");

    tracing::info!("[CACHE CONNECTED] {} 연결 성공", cache_backend);

    // AppState
    let state = AppState {
        postgres: db_pool,
        jwt,
        redis: redis_client,
    };

    let app = Router::new()
        .nest("/products", domains::product::router(state.clone()))
        .nest("/accounts", domains::account::router(state.clone()))
        .fallback(not_found)
        .layer(TraceLayer::new_for_http()) // Tower Middleware에서 모든 HTTP 요청/응답을 감시하며 로그를 남김
        .layer(axum::middleware::from_fn(request_id_middleware)); // 커스텀 미들웨어 적용

    /* ----------------------------
        필요 시 활성화
        cargo add tower-http --features "trace,cors,timeout,limit,compression-gzip"

        // 1. 응답 압축(gzip 등) -> 네트워크 트래픽 감소
        .layer(CompressionLayer::new())

        // 2. CORS 설정
        // 모든 Origin / Method / Header 허용
        .layer(CorsLayer::permissive())

        // 또는 아래처럼 작성
        .layer(
            CorsLayer::new()
                .allow_origin("https://axum-test.com".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([AUTHORIZATION, CONTENT_TYPE, ACCEPT])
        )

        // 3. request 타임 아웃
        .layer(TimeoutLayer::new(Duration::from_secs(30)))

        // 4. request Body 최대 크기 제한(예: JSON 업로드 1MB)
        // 대용량 요청으로 인한 메모리 사용 및 DoS 공격 방어
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
    */

    let server_address = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(server_address).await.unwrap();
    tracing::info!("[AXUM SERVER RUNNING] http://{}", server_address);

    axum::serve(listener, app).await.unwrap();
}
