use axum::http::StatusCode;
use tower::ServiceExt;

use super::super::{
    authorized_request, empty_request, response_json, seed_test_products, test_app, test_token,
};
use rust_axum_layered_rest_api::common::auth::role::AccountRole;

#[tokio::test]
async fn all_roles_with_read_permission_can_get_product() {
    // 시나리오 1: Admin, Employee, Customer 모두 상품 단건 조회가 가능하다.
    for role in [
        AccountRole::Admin,
        AccountRole::Employee,
        AccountRole::Customer,
    ] {
        let (app, pool, _redis) = test_app().await;
        seed_test_products(&pool).await;
        let token = test_token(vec![role]);
        let response = app
            .oneshot(authorized_request("GET", "/products/2", &token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "role={role:?}");
        let body = response_json(response).await;
        assert_eq!(body["data"]["id"], 2);
        assert!(body["data"]["name"].is_string());
    }
}

#[tokio::test]
async fn missing_product_returns_not_found() {
    // 시나리오 2: 존재하지 않는 상품 ID는 404를 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let token = test_token(vec![AccountRole::Customer]);
    let response = app
        .oneshot(authorized_request("GET", "/products/9999", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn invalid_path_is_rejected() {
    // 시나리오 3: ID 타입 오류는 400, 범위 오류는 422를 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let token = test_token(vec![AccountRole::Customer]);
    let response = app
        .clone()
        .oneshot(authorized_request("GET", "/products/abc", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .oneshot(authorized_request("GET", "/products/0", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn authentication_is_required() {
    // 시나리오 4: Authorization Header가 없으면 401을 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let response = app
        .oneshot(empty_request("GET", "/products/2"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
