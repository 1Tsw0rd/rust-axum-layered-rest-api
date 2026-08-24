use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::common::auth::role::AccountRole;
use crate::common::error::AppError;

// JWT Payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64,                // 사용자 ID
    pub exp: usize,              // 토큰 만료 시간(Unix timestamp)
    pub iat: usize,              // 토큰 발급 시간(Unix timestamp)
    pub roles: Vec<AccountRole>, // 계정 역할
}

// JWT 생성 및 검증에 필요한 설정
// 매번 검증 시 secret키로 인코딩/디코딩할 필요없이 처음에 인코딩키, 디코딩키를 미리 만들어 넣어둠
#[derive(Clone)]
pub struct JwtConfig {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_expires_in_seconds: i64,
}

// Access Token 기본 만료 시간 (15분)
const DEFAULT_ACCESS_TOKEN_EXPIRES: i64 = 900;

impl JwtConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let secret = std::env::var("JWT_SECRET")
            .map_err(|_| AppError::Internal("JWT_SECRET 환경변수가 설정되지 않았습니다.".into()))?;

        if secret.is_empty() {
            return Err(AppError::Internal(
                "JWT_SECRET은 비어 있을 수 없습니다.".into(),
            ));
        }

        let access_token_expires_in_seconds = std::env::var("JWT_ACCESS_TOKEN_EXPIRES_IN_SECONDS")
            .unwrap_or_else(|_| DEFAULT_ACCESS_TOKEN_EXPIRES.to_string())
            .parse::<i64>() // 환경변수 값을 i64 정수로 변환하고, 숫자가 아닌 잘못된 값은 오류 처리
            .map_err(|_| {
                AppError::Internal(
                    "JWT_ACCESS_TOKEN_EXPIRES_IN_SECONDS는 올바른 정수여야 합니다.".into(),
                )
            })?;

        if access_token_expires_in_seconds <= 0 {
            return Err(AppError::Internal(
                "JWT Access Token 만료 시간은 0보다 커야 합니다.".into(),
            ));
        }

        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_token_expires_in_seconds,
        })
    }

    // 테스트에서 JWT 설정을 직접 주입하기 위한 생성자
    pub fn new(secret: impl Into<String>, expires_in_seconds: i64) -> Self {
        let secret = secret.into();

        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_token_expires_in_seconds: expires_in_seconds,
        }
    }

    // Access Token 생성
    pub fn create_access_token(
        &self,
        account_id: i64,
        roles: Vec<AccountRole>,
    ) -> Result<String, AppError> {
        // 1. 현재 시간으로 iat 설정
        let now = Utc::now();
        let iat = now.timestamp() as usize;

        // 2. 만료 시간(exp) 계산
        let duration = Duration::try_seconds(self.access_token_expires_in_seconds)
            .ok_or_else(|| AppError::Internal("JWT 만료 시간 설정 범위를 초과했습니다.".into()))?;

        let exp = now
            .checked_add_signed(duration)
            .ok_or_else(|| AppError::Internal("JWT 만료 시간 계산에 실패했습니다.".into()))?
            .timestamp() as usize;

        // 3. Claims 구성
        let claims = Claims {
            sub: account_id,
            exp,
            iat,
            roles,
        };

        // 4. Header + Claims + EncodingKey로 서명
        encode(
            &Header::default(), // {"typ": "JWT", "alg": "HS256"} 헤더 정보를 런타임에 주입
            &claims,
            &self.encoding_key,
        )
        .map_err(|e| AppError::Internal(format!("JWT 토큰 생성 실패: {}", e)))
    }

    // Access Token 검증
    fn decode_access_token(&self, token: &str, validate_exp: bool) -> Result<Claims, AppError> {
        // 1. exp 검증 설정
        let validation = Validation {
            validate_exp,
            ..Default::default()
        };

        // 2. 토큰 + DecodingKey로 서명 및 Claims 검증: 성공 시 Claims 반환 / 실패 시 401
        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims) // data 안에서 claims 추출
            .map_err(|e| {
                // 만료 / 서명 불일치 / 잘못된 형식 등을 모두 인증 실패(401)로 반환
                tracing::warn!("JWT 검증 실패: {:?}", e.kind());
                AppError::Unauthorized
            })
    }
    // 서명 + exp 검증
    pub fn verify_access_token(&self, token: &str) -> Result<Claims, AppError> {
        self.decode_access_token(token, true)
    }
    // 서명만 검증
    pub fn verify_expired_access_token(&self, token: &str) -> Result<Claims, AppError> {
        self.decode_access_token(token, false)
    }
}

// cargo test --lib common::auth::jwt
// cargo test --lib common::auth::jwt -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> JwtConfig {
        JwtConfig::new("test-secret-key-for-unit-test-only", 3600)
    }

    // 시나리오 1: 정상적인 JWT를 생성하고 동일한 설정으로 검증했을 때 Claims가 정상적으로 복원되는지 검증
    #[test]
    fn create_and_verify_success() {
        let config = test_config();

        let token = config
            .create_access_token(42, vec![AccountRole::Customer, AccountRole::Employee])
            .expect("토큰 생성 실패");

        assert!(!token.is_empty());

        let claims = config.verify_access_token(&token).expect("토큰 검증 실패");

        assert_eq!(claims.sub, 42);
        assert_eq!(
            claims.roles,
            vec![AccountRole::Customer, AccountRole::Employee,]
        );
        assert!(claims.exp > claims.iat);
    }

    // 시나리오 2: 유효하지 않은 JWT를 검증했을 때 인증에 실패하는지 검증
    #[test]
    fn verify_invalid_token_fails() {
        let config = test_config();

        let result = config.verify_access_token("invalid.jwt.token");

        assert!(result.is_err());
    }

    // 시나리오 3: 다른 Secret으로 서명된 JWT를 검증했을 때 인증에 실패하는지 검증
    #[test]
    fn verify_wrong_secret_fails() {
        let config = test_config();

        let token = config
            .create_access_token(42, vec![AccountRole::Customer])
            .unwrap();

        let wrong_config = JwtConfig::new("completely-different-secret", 3600);

        let result = wrong_config.verify_access_token(&token);

        assert!(result.is_err());
    }

    // 시나리오 4: 서로 다른 account_id로 생성한 JWT가 서로 다른 Token을 생성하는지 검증
    #[test]
    fn different_account_ids_produce_different_tokens() {
        let config = test_config();

        let token1 = config
            .create_access_token(1, vec![AccountRole::Customer])
            .unwrap();

        let token2 = config
            .create_access_token(2, vec![AccountRole::Customer])
            .unwrap();

        assert_ne!(token1, token2);
    }

    // 시나리오 5: 만료된 토큰은
    // verify_access_token(exp 검증 O)에서는 실패하고,
    // verify_expired_access_token(exp 검증 X)에서는 서명이 유효하면 성공
    #[test]
    fn expired_token_rejected_by_verify_but_accepted_by_verify_expired() {
        // jsonwebtoken의 Validation::default()는 기본 leeway(허용 오차)가 60초라
        // -1초처럼 짧게 만료된 토큰은 허용 오차 범위 내에서 유효하게 판단 될 수 있음
        // leeway를 확실히 넘기기 위해 60초를 훨씬 넘기는 -120(2분 전 만료)으로 설정
        let config = JwtConfig::new("test-secret-key-for-unit-test-only", -120);
        let token = config
            .create_access_token(1, vec![AccountRole::Customer])
            .expect("토큰 생성 실패");

        assert!(config.verify_access_token(&token).is_err());

        let claims = config
            .verify_expired_access_token(&token)
            .expect("만료 토큰 검증(exp 무시) 실패");
        assert_eq!(claims.sub, 1);
    }
}
