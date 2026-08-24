use axum::http::StatusCode;
use tower::ServiceExt;

use super::super::{authorized_json_request, response_json, test_app, test_token};
use rust_axum_layered_rest_api::common::auth::role::AccountRole;

fn body() -> serde_json::Value {
    serde_json::json!({
        "name": "TestProductXYZ",
        "description": "테스트 상품 설명",
        "price": 12000
    })
}

#[tokio::test]
async fn admin_and_employee_can_create() {
    // 시나리오 1: Admin과 Employee는 상품 생성 권한을 가진다.
    for role in [AccountRole::Admin, AccountRole::Employee] {
        let (app, _pool, _redis) = test_app().await;
        let token = test_token(vec![role]);
        let response = app
            .oneshot(authorized_json_request("POST", "/products", &token, body()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "role={role:?}");
        let body = response_json(response).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["status"], 201);
    }
}

#[tokio::test]
async fn customer_is_forbidden() {
    // 시나리오 2: Customer는 상품 생성 시 403을 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let token = test_token(vec![AccountRole::Customer]);
    let response = app
        .oneshot(authorized_json_request("POST", "/products", &token, body()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn validation_and_json_errors_are_rejected() {
    // 시나리오 3: 필드 규칙 위반은 422, JSON/타입 오류는 400을 반환한다.
    let invalid = serde_json::json!({ "name": "", "description": "", "price": -1 });
    let (app, _pool, _redis) = test_app().await;
    let token = test_token(vec![AccountRole::Admin]);
    let response = app
        .oneshot(authorized_json_request(
            "POST",
            "/products",
            &token,
            invalid,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // name이 공백만으로 구성되면 다른 필드가 유효해도 단독으로 422여야 함(trim 후 길이 0)
    let whitespace_name =
        serde_json::json!({ "name": "   ", "description": "설명", "price": 1000 });
    let (app, _pool, _redis) = test_app().await;
    let response = app
        .oneshot(authorized_json_request(
            "POST",
            "/products",
            &token,
            whitespace_name,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let (app, _pool, _redis) = test_app().await;
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/products")
        .header(
            "authorization",
            format!("Bearer {}", test_token(vec![AccountRole::Admin])),
        )
        .header("content-type", "application/json")
        .body(axum::body::Body::from(r#"{"name": 1}"#))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn created_product_is_returned_by_list() {
    // 시나리오 4: 상품 생성 후 목록 조회에서 생성된 상품을 keyword로 찾을 수 있다.
    let (app, _pool, _redis) = test_app().await;
    let token = test_token(vec![AccountRole::Admin]);

    let response = app
        .clone()
        .oneshot(authorized_json_request("POST", "/products", &token, body()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(authorized_json_request(
            "GET",
            "/products?page=1&size=100&keyword=TestProductXYZ",
            &token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["meta"]["count"], 1);
    assert_eq!(body["meta"]["total"], 1);
    assert_eq!(body["data"][0]["name"], "TestProductXYZ");
    assert_eq!(body["data"][0]["description"], "테스트 상품 설명");
    assert_eq!(body["data"][0]["price"], 12000);
}
