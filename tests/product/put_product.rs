use axum::http::StatusCode;
use tower::ServiceExt;

use super::super::{authorized_json_request, authorized_request, response_json, seed_test_products, test_app, test_token};
use rust_axum_layered_rest_api::common::auth::role::AccountRole;

fn body() -> serde_json::Value {
    serde_json::json!({"name": "수정 상품", "description": "전체 수정", "price": 7777})
}

#[tokio::test]
async fn admin_and_employee_can_replace_product() {
    // 시나리오 1: Admin과 Employee는 PUT 전체 수정이 가능하고, 실제 값이 변경된다.
    for role in [AccountRole::Admin, AccountRole::Employee] {
        let (app, pool, _redis) = test_app().await;
        seed_test_products(&pool).await;
        let token = test_token(vec![role]);
        let response = app
            .clone()
            .oneshot(authorized_json_request("PUT", "/products/1", &token, body()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "role={role:?}");

        let response = app
            .oneshot(authorized_request("GET", "/products/1", &token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["data"]["name"], "수정 상품");
        assert_eq!(body["data"]["description"], "전체 수정");
        assert_eq!(body["data"]["price"], 7777);
    }
}

#[tokio::test]
async fn customer_is_forbidden_and_invalid_payload_is_rejected() {
    // 시나리오 2: Customer는 403, 잘못된 필드는 422를 반환한다.
    let (app, pool, _redis) = test_app().await;
    seed_test_products(&pool).await;
    let customer = test_token(vec![AccountRole::Customer]);
    let response = app
        .clone()
        .oneshot(authorized_json_request("PUT", "/products/1", &customer, body()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let admin = test_token(vec![AccountRole::Admin]);
    let invalid = serde_json::json!({"name": "", "description": "", "price": -1});
    let response = app
        .clone()
        .oneshot(authorized_json_request("PUT", "/products/1", &admin, invalid))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // name이 공백만으로 구성되면 다른 필드가 유효해도 단독으로 422여야 함(trim 후 길이 0)
    let whitespace_name = serde_json::json!({"name": "   ", "description": "설명", "price": 1000});
    let response = app
        .oneshot(authorized_json_request("PUT", "/products/1", &admin, whitespace_name))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn missing_product_returns_not_found() {
    // 시나리오 3: 존재하지 않는 상품을 PUT하면 404를 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let token = test_token(vec![AccountRole::Admin]);
    let response = app
        .oneshot(authorized_json_request("PUT", "/products/9999", &token, body()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn invalid_path_is_rejected() {
    // 시나리오 4: ID 타입 오류는 400, 범위 오류는 422를 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let token = test_token(vec![AccountRole::Admin]);

    let response = app
        .clone()
        .oneshot(authorized_json_request("PUT", "/products/abc", &token, body()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .oneshot(authorized_json_request("PUT", "/products/0", &token, body()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
