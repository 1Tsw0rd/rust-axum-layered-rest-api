use axum::http::StatusCode;
use tower::ServiceExt;

use super::super::{authorized_json_request, authorized_request, response_json, seed_test_products, test_app, test_token};
use rust_axum_layered_rest_api::common::auth::role::AccountRole;

#[tokio::test]
async fn each_partial_update_shape_is_supported() {
    // 시나리오 1: name, description, price 단일 필드와 복합 필드 PATCH가 실제로 반영되고,
    // 보내지 않은 필드는 원본 값이 유지된다.
    let (app, pool, _redis) = test_app().await;
    seed_test_products(&pool).await;
    let token = test_token(vec![AccountRole::Admin]);

    // 목록 조회로 테스트 대상 4개(id 포함)를 동적으로 가져옴
    let response = app
        .clone()
        .oneshot(authorized_request("GET", "/products?page=1&size=4", &token))
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);

    let list = response_json(response).await;
    let products = list["data"].as_array().unwrap();

    let patches = [
        serde_json::json!({"name": "이름만 수정"}),
        serde_json::json!({"description": "설명만 수정"}),
        serde_json::json!({"price": 9999}),
        serde_json::json!({"name": "복합 수정", "price": 8888}),
    ];

    for (before, patch_body) in products.iter().zip(patches) {
        let id = before["id"].as_i64().unwrap();

        let response = app
            .clone()
            .oneshot(authorized_json_request("PATCH", &format!("/products/{id}"), &token, patch_body.clone()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "id={id}");

        let response = app
            .clone()
            .oneshot(authorized_request("GET", &format!("/products/{id}"), &token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let after = response_json(response).await;

        // 수정한 필드는 변경되었는지, 수정하지 않은 필드는 원래 값(before) 그대로인지 확인
        for field in ["name", "description", "price"] {
            let expected = patch_body.get(field).cloned().unwrap_or(before[field].clone());
            assert_eq!(after["data"][field], expected, "id={id} field={field}");
        }
    }
}

#[tokio::test]
async fn empty_patch_and_invalid_values_are_rejected() {
    // 시나리오 2: 수정 필드가 없거나 필드 값이 유효하지 않으면 400/422를 반환한다.
    let (app, pool, _redis) = test_app().await;
    seed_test_products(&pool).await;
    let token = test_token(vec![AccountRole::Admin]);

    // 변경 필드 없음 → Service 단에서 400
    let response = app
        .clone()
        .oneshot(authorized_json_request(
            "PATCH",
            "/products/1",
            &token,
            serde_json::json!({}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // name 빈 문자열 → 최소 길이 위반 → 422
    let response = app
        .clone()
        .oneshot(authorized_json_request(
            "PATCH",
            "/products/1",
            &token,
            serde_json::json!({"name": ""}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // name 101자 → 최대 길이 위반 → 422
    let response = app
        .clone()
        .oneshot(authorized_json_request(
            "PATCH",
            "/products/1",
            &token,
            serde_json::json!({"name": "a".repeat(101)}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // description 1001자 → 최대 길이 위반 → 422
    let response = app
        .clone()
        .oneshot(authorized_json_request(
            "PATCH",
            "/products/1",
            &token,
            serde_json::json!({"description": "a".repeat(1001)}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // price 음수 → 최소값 위반 → 422
    let response = app
        .oneshot(authorized_json_request(
            "PATCH",
            "/products/1",
            &token,
            serde_json::json!({"price": -1}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn customer_is_forbidden_and_missing_product_is_not_found() {
    // 시나리오 3: Customer는 수정할 수 없고, 없는 상품은 404를 반환한다.
    let (app, pool, _redis) = test_app().await;
    seed_test_products(&pool).await;
    let customer = test_token(vec![AccountRole::Customer]);
    let response = app
        .clone()
        .oneshot(authorized_json_request(
            "PATCH", "/products/1", &customer, serde_json::json!({"name": "거부"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let admin = test_token(vec![AccountRole::Admin]);
    let response = app
        .oneshot(authorized_json_request(
            "PATCH", "/products/9999", &admin, serde_json::json!({"name": "없음"}),
        ))
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
        .oneshot(authorized_json_request(
            "PATCH", "/products/abc", &token, serde_json::json!({"name": "수정"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = app
        .oneshot(authorized_json_request(
            "PATCH", "/products/0", &token, serde_json::json!({"name": "수정"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
