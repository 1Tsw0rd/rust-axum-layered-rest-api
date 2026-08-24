use axum::http::StatusCode;
use tower::ServiceExt;

use super::super::{
    authorized_request, empty_request, response_json, seed_test_products, test_app, test_token,
};
use rust_axum_layered_rest_api::common::auth::role::AccountRole;

#[tokio::test]
async fn success_with_pagination_meta() {
    // 시나리오 1: Customer가 첫 페이지 상품 목록과 meta를 조회한다.
    let (app, pool, _redis) = test_app().await;
    seed_test_products(&pool).await;
    let token = test_token(vec![AccountRole::Customer]);
    let response = app
        .oneshot(authorized_request("GET", "/products?page=1&size=2", &token))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["meta"]["count"], 2);
    assert_eq!(body["meta"]["total"], 5);
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["size"], 2);
    assert_eq!(body["meta"]["total_pages"], 3);
}

#[tokio::test]
async fn success_with_keyword_and_second_page() {
    // 시나리오 2: keyword 검색(Keyboard, 3건)과 두 번째 페이지 요청이 정상 처리된다.
    let (app, pool, _redis) = test_app().await;
    seed_test_products(&pool).await;
    let token = test_token(vec![AccountRole::Customer]);
    let response = app
        .oneshot(authorized_request(
            "GET",
            "/products?page=2&size=2&keyword=Keyboard",
            &token,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["meta"]["page"], 2);
    assert_eq!(body["meta"]["size"], 2);
    assert_eq!(body["meta"]["total"], 3);
    assert_eq!(body["meta"]["count"], 1);
}

#[tokio::test]
async fn validation_rejects_invalid_query_values() {
    // 시나리오 3: page=0, size=0, size=101, keyword 초과는 422를 반환한다.
    for query in [
        "page=0&size=2",
        "page=1&size=0",
        "page=1&size=101",
        "page=1&size=2&keyword=123456789012345678901",
    ] {
        let (app, _pool, _redis) = test_app().await;
        let token = test_token(vec![AccountRole::Customer]);
        let response = app
            .oneshot(authorized_request(
                "GET",
                &format!("/products?{query}"),
                &token,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = response_json(response).await;
        assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    }
}

#[tokio::test]
async fn malformed_query_returns_bad_request() {
    // 시나리오 4: Query 타입이 숫자가 아니면 400을 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let token = test_token(vec![AccountRole::Customer]);
    let response = app
        .oneshot(authorized_request(
            "GET",
            "/products?page=abc&size=2",
            &token,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], "BAD_REQUEST");
}

#[tokio::test]
async fn authentication_and_permission_are_enforced() {
    // 시나리오 5: 인증 누락은 401이며, 세 역할 모두 조회 권한은 허용된다.
    let (app, _pool, _redis) = test_app().await;
    let response = app
        .clone()
        .oneshot(empty_request("GET", "/products?page=1&size=2"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    for role in [
        AccountRole::Admin,
        AccountRole::Employee,
        AccountRole::Customer,
    ] {
        let (app, _pool, _redis) = test_app().await;
        let token = test_token(vec![role]);
        let response = app
            .oneshot(authorized_request("GET", "/products?page=1&size=2", &token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "role={role:?}");
    }
}

#[tokio::test]
async fn empty_search_result_returns_empty_list() {
    // 시나리오 6: 검색 결과가 없으면 빈 목록과 함께 200을 반환한다.
    let (app, pool, _redis) = test_app().await;
    seed_test_products(&pool).await;
    let token = test_token(vec![AccountRole::Customer]);
    let response = app
        .oneshot(authorized_request(
            "GET",
            "/products?page=1&size=10&keyword=NotExistXYZ",
            &token,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response_json(response).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["meta"]["count"], 0);
    assert_eq!(body["meta"]["total"], 0);
    assert_eq!(body["meta"]["total_pages"], 0);
}
