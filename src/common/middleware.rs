use axum::{
    body::Body,
    extract::State,
    http::{Request, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing::{info, Instrument};
use uuid::Uuid;

use crate::common::{
    auth::extractor::authenticate,
    auth::jwt::JwtConfig,
    error::AppError,
};

// main.rs에 .layer(axum::middleware::from_fn(request_id_middleware)); 로 적용함
// Request ID Middleware
// - 모든 api 요청마다 request-id를 저장하여 디버깅 용이하도록 함
// 예시:
//
// Client
//   |
//   | GET /products/1
//   |
//   v
// Middleware
//   request_id = "63e63954-dd7d-487f-9268-e24fcfd6a5e2"
//   |
//   v
// Controller / Service / Repository
//   |
//   v
// Logs
//   request_id: 동일한 UUID로 추적 가능
// 2026-08-06T16:22:18.592404Z  INFO my_app::common::middleware: request completed
//     at src\common\middleware.rs:31
//     in my_app::common::middleware::request with request_id: 63e63954-dd7d-487f-9268-e24fcfd6a5e2, method: GET, uri: /products/1
//
//   2026-08-06T16:22:18.592542Z DEBUG tower_http::trace::on_eos: end of stream, stream_duration: 0 ms
//     at C:\products\ohhoj\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\tower-http-0.7.0\src\trace\on_eos.rs:103
//     in tower_http::trace::make_span::request with method: GET, uri: /products/1, version: HTTP/1.1
//     in my_app::common::middleware::request with request_id: 63e63954-dd7d-487f-9268-e24fcfd6a5e2, method: GET, uri: /products/1
//
// Response Header
//   x-request-id: 63e63954-dd7d-487f-9268-e24fcfd6a5e2
pub async fn request_id_middleware(
    req: Request<Body>, // HTTP Request 객체 (method, uri, header, body 포함)
    next: Next, // 다음 middleware 또는 handler 실행 (NestJS의 next()와 유사)
) -> Response {
    // 요청마다 고유한 Request ID 생성
    let request_id = Uuid::new_v4().to_string();

     // 현재 요청을 표현하는 tracing span 생성하여 request_id, method, uri 정보 공유
    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %req.method(),
        uri = %req.uri()
    );

    // response header를 수정하기 위해 mut로 선언
    // next.run()으로 다음 middleware 또는 handler 실행
    // instrument()를 사용하여 요청 처리 Future 전체에 span 적용
    // 같은 request_id로 controller/service/repository 로그 추적 가능
    let mut response = next
        .run(req)
        .instrument(span.clone())
        .await;


    // response 완료 로그 기록
    // parent: &span 지정하면 해당 request 완료 로그가 request 로그 트리에 포함됨
    info!(
        parent: &span,
        "request completed"
    );

    // response header에 request id 추가
    // 장애 분석 시 클라이언트가 받은 request id로 서버 로그 검색 가능
    response.headers_mut()
        .insert(
            "x-request-id",
            HeaderValue::from_str(&request_id).unwrap(),
        );

    // 반환
    response
}

// JWT 인증 Middleware
//
// - Authorization Header에서 JWT를 검증
// - AuthenticatedUser를 생성
// - request.extensions()에 인증 정보를 저장
//
// 공개 API에는 이 Middleware를 적용하지 않고,
// 인증이 필요한 API에만 적용한다.
pub async fn authentication_middleware(
    State(jwt): State<JwtConfig>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // 공통 인증 함수 사용
    let user = authenticate(
        request.headers().get(axum::http::header::AUTHORIZATION),
        &jwt,
    )?;

    // 이후 Authorization Middleware와 Controller Extractor가 재사용
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
