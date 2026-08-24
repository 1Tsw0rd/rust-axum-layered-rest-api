use axum::http::StatusCode;
use tower::ServiceExt;

use super::super::{json_request, register_account, response_json, test_app};

#[tokio::test]
async fn login_returns_access_token_and_refresh_cookie() {
    // 시나리오 1: 유효한 계정 로그인은 Access Token과 Refresh Cookie를 반환한다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "login@example.com", "customer").await;
    let response = app
        .oneshot(json_request(
            "POST",
            "/accounts/login",
            serde_json::json!({"email": "login@example.com", "password": "Password123!"}),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let cookie = response
        .headers()
        .get("set-cookie")
        .expect("Refresh Cookie가 없음")
        .to_str()
        .expect("Set-Cookie 변환 실패")
        .to_owned();
    assert!(cookie.contains("refresh_token="));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("SameSite=Strict"));
    assert!(cookie.contains("Path=/accounts"));

    let json = response_json(response).await;
    assert_eq!(json["success"], true);
    assert!(json["data"]["access_token"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn invalid_credentials_are_not_disclosed() {
    // 시나리오 2: 존재하지 않는 계정과 잘못된 비밀번호는 동일한 401 응답을 반환한다.
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "real@example.com", "customer").await;

    for (email, password) in [
        ("unknown@example.com", "Password123!"),
        ("real@example.com", "Wrong123!"),
    ] {
        let response = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/accounts/login",
                serde_json::json!({"email": email, "password": password}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = response_json(response).await;
        assert_eq!(json["error"]["code"], "INVALID_CREDENTIALS");
    }
}

#[tokio::test]
async fn login_validation_is_enforced() {
    // 시나리오 3: 이메일 형식과 비밀번호 필수 입력을 검증한다.
    let (app, _pool, _redis) = test_app().await;
    for body in [
        serde_json::json!({"email": "not-email", "password": "Password123!"}),
        serde_json::json!({"email": "valid@example.com", "password": ""}),
    ] {
        let response = app
            .clone()
            .oneshot(json_request("POST", "/accounts/login", body))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn login_normalizes_email() {
    // 시나리오 4: 이메일 대소문자를 무시하고 로그인할 수 있다(normalize_email의 to_lowercase() 검증)
    let (app, _pool, _redis) = test_app().await;
    register_account(&app, "login@example.com", "customer").await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/accounts/login",
            serde_json::json!({
                "email": "LOGIN@EXAMPLE.COM",
                "password": "Password123!"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
