use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

use super::super::{login_account, register_account, response_json, test_app};

// Access Token(Authorization Header)과 Refresh Token(Cookie)을 선택적으로 포함하는 로그아웃 요청 생성
// 각각 None이면 해당 헤더를 생략함 (Access Token / Refresh Cookie 누락 케이스 테스트에 사용)
fn logout_request(token: Option<&str>, cookie: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method("POST").uri("/accounts/logout");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    builder.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn logout_removes_refresh_session() {
    // 시나리오 1: 유효한 Access Token과 Cookie로 로그아웃하고 삭제 Cookie 응답을 받는다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "logout@example.com", "customer").await;
    let (token, cookie) = login_account(&app, "logout@example.com").await;
    let response = app
        .clone()
        .oneshot(logout_request(Some(&token), Some(&cookie)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let set_cookie = response
        .headers()
        .get("set-cookie")
        .expect("Refresh Cookie 삭제 응답이 없음")
        .to_str()
        .unwrap()
        .to_owned();
    assert!(set_cookie.contains("refresh_token="));
    assert!(set_cookie.contains("Path=/accounts"));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("Secure"));
    assert!(set_cookie.contains("SameSite=Strict"));

    let json = response_json(response).await;
    assert_eq!(json["success"], true);

    // 같은 Refresh Cookie는 로그아웃 후 사용할 수 없어야 한다.
    let response = app
        .oneshot(logout_request(Some(&token), Some(&cookie)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_requires_access_token_and_refresh_cookie() {
    // 시나리오 2: Access Token 없음, Refresh Cookie 없음, 잘못된 Access Token은 각각 401이다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "logout-invalid@example.com", "customer").await;
    let (token, cookie) = login_account(&app, "logout-invalid@example.com").await;

    // Access Token 없음
    let response = app
        .clone()
        .oneshot(logout_request(None, Some(&cookie)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Refresh Cookie 없음
    let response = app
        .clone()
        .oneshot(logout_request(Some(&token), None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 잘못된 Access Token
    let response = app
        .oneshot(logout_request(
            Some("old.invalid.token"),
            Some("refresh_token=old"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_rejects_mismatched_account() {
    // 시나리오 3: Access Token과 Refresh Cookie의 계정이 다르면 401을 반환한다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "user-aaa@example.com", "customer").await;
    register_account(&app, "user-bbb@example.com", "customer").await;

    let (token_a, _cookie_a) = login_account(&app, "user-aaa@example.com").await;
    let (_token_b, cookie_b) = login_account(&app, "user-bbb@example.com").await;

    let response = app
        .oneshot(logout_request(Some(&token_a), Some(&cookie_b)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
