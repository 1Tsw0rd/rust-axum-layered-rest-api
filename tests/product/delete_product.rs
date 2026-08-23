use axum::http::StatusCode;
use tower::ServiceExt;

use super::super::{authorized_request, response_json, seed_test_products, test_app, test_token};
use rust_axum_layered_rest_api::common::auth::role::AccountRole;

#[tokio::test]
async fn admin_can_delete_and_deleted_product_is_not_found() {
    // 시나리오 1: Admin이 삭제한 상품은 이후 조회 시 404가 된다.
    let (app, pool, _redis) = test_app().await;
    seed_test_products(&pool).await;
    let token = test_token(vec![AccountRole::Admin]);
    let response = app
        .clone()
        .oneshot(authorized_request("DELETE", "/products/1", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(authorized_request("GET", "/products/1", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn employee_and_customer_are_forbidden() {
    // 시나리오 2: Employee와 Customer는 삭제 권한이 없어 403을 반환한다.
    for role in [AccountRole::Employee, AccountRole::Customer] {
        let (app, pool, _redis) = test_app().await;
        seed_test_products(&pool).await;
        let token = test_token(vec![role]);
        let response = app
            .oneshot(authorized_request("DELETE", "/products/1", &token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "role={role:?}");
    }
}

#[tokio::test]
async fn invalid_or_missing_product_is_rejected() {
    // 시나리오 3: ID 형식·범위 오류와 없는 대상 삭제를 검증한다.
    let (app, pool, _redis) = test_app().await;
    seed_test_products(&pool).await;
    let token = test_token(vec![AccountRole::Admin]);
    let response = app
        .clone()
        .oneshot(authorized_request("DELETE", "/products/abc", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .clone()
        .oneshot(authorized_request("DELETE", "/products/0", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let response = app
        .oneshot(authorized_request("DELETE", "/products/9999", &token))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}
