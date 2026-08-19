use std::sync::Arc;

use axum::{
    extract::FromRef, // FromRef: Router State에서 필요한 타입(여기선 Arc<AccountService>)을 꺼내기 위한 trait
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, HeaderValue},
};
use crate::common::auth::{
    jwt::JwtConfig,
    role::AccountRole,
};
use crate::common::error::AppError;
use crate::domains::account::service::AccountService;

// Jwt 인증 완료된 사용자 정보
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub account_id: i64,
    pub roles: Vec<AccountRole>,
}

// Authorization Header + JWT를 검증하여 AuthenticatedUser 생성
// 인증에 필요한 실제 로직은 이 함수 하나만 사용
// Extractor와 Authentication Middleware에서 공통으로 호출
pub fn authenticate(
    auth_header: Option<&HeaderValue>,
    jwt: &JwtConfig,
) -> Result<AuthenticatedUser, AppError> {
    // 1. Authorization 헤더 추출
    let auth_header = auth_header
        .ok_or_else(|| {
            tracing::warn!(
                "인증 실패: Authorization Header가 없습니다."
            );
            AppError::Unauthorized
        })?
        .to_str()
        .map_err(|_| {
            tracing::warn!(
                "인증 실패: Authorization Header를 문자열로 변환할 수 없습니다."
            );
            AppError::Unauthorized
        })?;

    // tracing::debug!("Authorization Header: {}", auth_header);

    // 2. "Bearer <token>" 형식 확인: Bearer 접두사가 없거나 Token이 비어있는 경우 인증 실패
    let token = auth_header
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .ok_or_else(|| {
            tracing::warn!(
                "인증 실패: Authorization Header의 Bearer Token 형식이 올바르지 않습니다."
            );
            AppError::Unauthorized
        })?;


    // 3. JWT 검증
    let claims = jwt.verify_access_token(token)?;
    // tracing::debug!(account_id = claims.sub, roles = ?claims.roles, "JWT 인증 성공");

    // 4. 인증된 사용자 정보 생성
    Ok(AuthenticatedUser {
        account_id: claims.sub,
        roles: claims.roles,
    })
}

// 헤더를 추출하고 JWT 검증하여 인증된 사용자 정보 생성
// S: 현재 Router가 사용하는 State 타입을 의미
// 예: .with_state(Arc<AccountService>) → S = Arc<AccountService>
// 어떤 State 타입 S가 들어오더라도, 조건만 만족하면 AuthenticatedUser를 Extractor로 사용할 수 있게 함
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    Arc<AccountService>: FromRef<S>,
{
    // Extractor 실패 시 반환할 에러 타입
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts, // HTTP 요청의 Header, URI 등 Body를 제외한 요청 정보
        state: &S, // Router에 .with_state(...)로 등록해둔 값(예: Arc<AccountService>)의 참조
    ) -> Result<Self, Self::Rejection> {
        // Middleware에서 이미 JWT 인증을 완료한 경우
        // request.extensions() 전역 보관함에 저장된 AuthenticatedUser를 꺼내서 사용
        if let Some(user) = parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
        {
            return Ok(user);
        }

        // Router State에서 AccountService 추출
        let account_service = Arc::<AccountService>::from_ref(state);

        // 공통 인증 함수 사용
        authenticate(
            parts.headers.get(AUTHORIZATION),
            account_service.jwt(),
        )
    }
}

// cargo test --lib common::auth::extractor
// cargo test --lib common::auth::extractor -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::auth::role::AccountRole;

    fn test_jwt() -> JwtConfig {
        JwtConfig::new("test-secret-key-for-unit-test-only", 3600)
    }

    // 시나리오 1: Authorization 헤더가 없으면 인증 실패
    #[test]
    fn missing_header_fails() {
        let result = authenticate(None, &test_jwt());
        assert!(result.is_err());
    }

    // 시나리오 2: Bearer 접두사가 없으면 인증 실패
    #[test]
    fn missing_bearer_prefix_fails() {
        // 테스트용 Authorization Header Value 생성
        // Authorization: sometoken
        let header = HeaderValue::from_static("sometoken");
        let result = authenticate(Some(&header), &test_jwt());
        assert!(result.is_err());
    }

    // 시나리오 3: Bearer 뒤 토큰이 비어있으면 인증 실패
    #[test]
    fn empty_bearer_token_fails() {
        // Authorization: Bearer
        let header = HeaderValue::from_static("Bearer ");
        let result = authenticate(Some(&header), &test_jwt());
        assert!(result.is_err());
    }

    // 시나리오 4: 유효하지 않은 JWT면 인증 실패
    #[test]
    fn invalid_jwt_fails() {
        // Authorization: Bearer invalid.jwt.token
        let header = HeaderValue::from_static("Bearer invalid.jwt.token");
        let result = authenticate(Some(&header), &test_jwt());
        assert!(result.is_err());
    }

    // 시나리오 5: 유효한 JWT면 AuthenticatedUser가 정상 생성됨
    #[test]
    fn valid_jwt_succeeds() {
        let jwt = test_jwt();
        let token = jwt.create_access_token(7, vec![AccountRole::Customer]).unwrap();

        // Authorization: Bearer 올바른 토큰값
        let header_value = format!("Bearer {token}");
        let header = HeaderValue::from_str(&header_value).unwrap();

        let user = authenticate(Some(&header), &jwt).expect("인증 성공해야 함");
        assert_eq!(user.account_id, 7);
        assert_eq!(user.roles, vec![AccountRole::Customer]);
    }

    // 시나리오 6: Employee 역할로 발급된 토큰도 정상적으로 인증되어 roles가 그대로 보존된다
    #[test]
    fn valid_jwt_with_employee_role_succeeds() {
        let jwt = test_jwt();
        let token = jwt.create_access_token(100, vec![AccountRole::Employee]).unwrap();
        let header_value = format!("Bearer {token}");
        let header = HeaderValue::from_str(&header_value).unwrap();

        let user = authenticate(Some(&header), &jwt).expect("인증 성공해야 함");
        assert_eq!(user.account_id, 100);
        assert_eq!(user.roles, vec![AccountRole::Employee]);
    }

    // 시나리오 7: Admin 역할로 발급된 토큰도 정상적으로 인증되어 roles가 그대로 보존된다
    #[test]
    fn valid_jwt_with_admin_role_succeeds() {
        let jwt = test_jwt();
        let token = jwt.create_access_token(99, vec![AccountRole::Admin]).unwrap();
        let header_value = format!("Bearer {token}");
        let header = HeaderValue::from_str(&header_value).unwrap();

        let user = authenticate(Some(&header), &jwt).expect("인증 성공해야 함");
        assert_eq!(user.account_id, 99);
        assert_eq!(user.roles, vec![AccountRole::Admin]);
    }
}
