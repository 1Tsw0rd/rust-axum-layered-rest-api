use axum::http::StatusCode;
use tower::ServiceExt;

use super::super::{json_request, response_json, test_app};

fn body(email: &str, role: &str) -> serde_json::Value {
    serde_json::json!({
        "email": email,
        "password": "Password123!",
        "name": "홍길동",
        "role": role
    })
}

#[tokio::test]
async fn customer_and_employee_can_register() {
    // 시나리오 1: customer와 employee 역할로 회원가입할 수 있다.
    for (email, role) in [("customer@example.com", "customer"), ("employee@example.com", "employee")] {
        let (app, _pool, _redis) = test_app().await;
        let response = app
            .oneshot(json_request("POST", "/accounts", body(email, role)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED, "role={role}");
        let json = response_json(response).await;
        assert_eq!(json["success"], true);
        assert_eq!(json["status"], 201);
    }
}

#[tokio::test]
async fn admin_role_is_rejected() {
    // 시나리오 2: 일반 회원가입에서 admin 역할은 허용하지 않는다.
    let (app, _pool, _redis) = test_app().await;
    let response = app
        .oneshot(json_request("POST", "/accounts", body("admin-signup@example.com", "admin")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json = response_json(response).await;
    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn validation_rules_are_enforced() {
    // 시나리오 3: 이메일·비밀번호·이름·role 검증 실패는 422를 반환한다.
    for invalid in [
        serde_json::json!({"email": "not-email", "password": "Password123!", "name": "홍길동", "role": "customer"}),
        serde_json::json!({"email": "valid@example.com", "password": "password", "name": "홍길동", "role": "customer"}),
        serde_json::json!({"email": "valid2@example.com", "password": "Password123!", "name": "", "role": "customer"}),
        serde_json::json!({"email": "valid3@example.com", "password": "Password123!", "name": "a".repeat(51), "role": "customer"}),
        serde_json::json!({"email": "valid4@example.com", "password": "Password123!", "name": "홍길동", "role": "unknown"}),
    ] {
        let (app, _pool, _redis) = test_app().await;
        let response = app.oneshot(json_request("POST", "/accounts", invalid)).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn missing_field_and_invalid_json_are_rejected() {
    // 시나리오 4: 필수 필드 누락과 JSON 형식 오류는 400을 반환한다.
    let (app, _pool, _redis) = test_app().await;
    let response = app
        .clone()
        .oneshot(json_request(
            "POST", "/accounts", serde_json::json!({"email": "missing@example.com"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/accounts")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(r#"{"email":"broken""#))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn duplicate_email_is_case_insensitive() {
    // 시나리오 5: 이메일 대소문자를 무시하고 중복 가입을 차단한다.
    let (app, _pool, _redis) = test_app().await;
    let response = app
        .clone()
        .oneshot(json_request("POST", "/accounts", body("Case@example.com", "customer")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(json_request("POST", "/accounts", body("case@EXAMPLE.COM", "customer")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = response_json(response).await;
    assert_eq!(json["error"]["code"], "BAD_REQUEST");
}
