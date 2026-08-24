use axum::http::StatusCode;
use tower::ServiceExt;

use super::super::{
    authorized_request, empty_request, register_account, response_json, test_app, test_token,
};
use rust_axum_layered_rest_api::common::auth::role::AccountRole;

#[tokio::test]
async fn authenticated_user_can_get_own_account() {
    // 시나리오 1: 인증된 사용자는 자신의 계정을 조회할 수 있다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "me@example.com", "customer").await;
    let token = test_token(vec![AccountRole::Customer]);
    let response = app
        .oneshot(authorized_request("GET", "/accounts/me", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["data"]["id"], 1);
    assert_eq!(json["data"]["email"], "me@example.com");
}

#[tokio::test]
async fn missing_or_invalid_token_is_unauthorized() {
    // 시나리오 2: Authorization Header 누락·잘못된 토큰은 401을 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let response = app
        .clone()
        .oneshot(empty_request("GET", "/accounts/me"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .oneshot(authorized_request(
            "GET",
            "/accounts/me",
            "old.invalid.token",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn nonexistent_account_returns_not_found() {
    // 시나리오 3: 토큰의 account_id가 DB에 존재하지 않으면 404를 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let token = test_token(vec![AccountRole::Customer]); // account_id=1이지만 register_account를 안 했으니 DB에 없음
    let response = app
        .oneshot(authorized_request("GET", "/accounts/me", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
