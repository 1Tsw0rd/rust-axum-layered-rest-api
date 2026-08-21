use axum::{body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;

use super::super::{login_account, register_account, response_json, test_app};
use rust_axum_layered_rest_api::common::auth::{jwt::JwtConfig, role::AccountRole};

fn refresh_request(token: &str, cookie: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/accounts/refresh")
        .header("authorization", format!("Bearer {token}"))
        .header("cookie", cookie)
        .body(Body::empty())
        .unwrap()
}

// Set-Cookie 응답 헤더에서 refresh_token 쿠키 값만 추출 (다음 요청에 "cookie" 헤더로 그대로 사용)
fn extract_refresh_cookie(response: &axum::http::Response<Body>) -> String {
    response
        .headers()
        .get("set-cookie")
        .expect("Refresh Cookie가 없음")
        .to_str()
        .expect("Set-Cookie 변환 실패")
        .split(';')
        .next()
        .expect("Refresh Cookie 파싱 실패")
        .to_owned()
}

#[tokio::test]
async fn expired_access_token_rotates_refresh_session() {
    // 시나리오 1: 만료된 Access Token과 유효한 Cookie로 새 토큰을 발급하고 Rotation한다.
    // 새로 발급된 Refresh Cookie로 다시 refresh가 성공하는 것까지 확인한다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "refresh@example.com", "customer").await;
    let (_, refresh_cookie) = login_account(&app, "refresh@example.com").await;
    let expired = JwtConfig::new("test-secret-key-for-test", -120)
        .create_access_token(1, vec![AccountRole::Customer])
        .unwrap();

    let response = app
        .clone()
        .oneshot(refresh_request(&expired, &refresh_cookie))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let new_refresh_cookie = extract_refresh_cookie(&response);
    let json = response_json(response).await;
    assert!(json["data"]["access_token"].is_string());

    // 이전 Refresh Cookie는 Rotation 후 재사용할 수 없어야 한다.
    let response = app
        .clone()
        .oneshot(refresh_request(&expired, &refresh_cookie))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 새로 발급된 Refresh Cookie로는 다시 refresh가 성공해야 한다.
    let response = app
        .oneshot(refresh_request(&expired, &new_refresh_cookie))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn refresh_requires_expired_access_token_and_cookie() {
    // 시나리오 2: Header/Cookie 누락과 아직 만료되지 않은 토큰은 401이다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "refresh-invalid@example.com", "customer").await;
    let (access_token, refresh_cookie) = login_account(&app, "refresh-invalid@example.com").await;

    // 아직 만료되지 않은 Access Token
    let response = app
        .clone()
        .oneshot(refresh_request(&access_token, &refresh_cookie))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Refresh Cookie 없음
    let expired = JwtConfig::new("test-secret-key-for-test", -120)
        .create_access_token(1, vec![AccountRole::Customer])
        .unwrap();
    let request = Request::builder()
        .method("POST")
        .uri("/accounts/refresh")
        .header("authorization", format!("Bearer {expired}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Authorization Header 없음
    let request = Request::builder()
        .method("POST")
        .uri("/accounts/refresh")
        .header("cookie", &refresh_cookie)
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_rejects_unknown_refresh_token() {
    // 시나리오 3: 만료된 Access Token과 존재하지 않는 Refresh Token은 401이다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "refresh-unknown@example.com", "customer").await;
    login_account(&app, "refresh-unknown@example.com").await;

    let expired = JwtConfig::new("test-secret-key-for-test", -120)
        .create_access_token(1, vec![AccountRole::Customer])
        .unwrap();

    let response = app
        .oneshot(refresh_request(&expired, "refresh_token=totally-unknown-value"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_rejects_mismatched_account() {
    // 시나리오 4: Access Token과 Refresh Cookie의 계정이 다르면 401을 반환한다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "refresh-aaa@example.com", "customer").await;
    register_account(&app, "refresh-bbb@example.com", "customer").await;

    login_account(&app, "refresh-aaa@example.com").await;
    let (_, cookie_b) = login_account(&app, "refresh-bbb@example.com").await;

    // account_id=1(refresh-aaa)의 만료 토큰 + account_id=2(refresh-bbb)의 Refresh Cookie
    let expired_a = JwtConfig::new("test-secret-key-for-test", -120)
        .create_access_token(1, vec![AccountRole::Customer])
        .unwrap();

    let response = app
        .oneshot(refresh_request(&expired_a, &cookie_b))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_rejects_malformed_authorization_header() {
    // 시나리오 5: Bearer 형식이 아니거나 토큰이 비어있는 Authorization Header는 401이다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "refresh-bad-header@example.com", "customer").await;
    let (_, refresh_cookie) = login_account(&app, "refresh-bad-header@example.com").await;

    for auth_value in ["Basic abc", "Bearer", "Bearer "] {
        let request = Request::builder()
            .method("POST")
            .uri("/accounts/refresh")
            .header("authorization", auth_value)
            .header("cookie", &refresh_cookie)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "auth_value={auth_value:?}");
    }
}
