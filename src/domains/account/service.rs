use sqlx::PgPool;

use crate::common::security::password::{
    hash_password,
    verify_password,
};
use crate::common::{
    auth::{
        jwt::JwtConfig,
        role::AccountRole,
        refresh_token::{
            delete_refresh_token,
            find_refresh_token,
            generate_refresh_token,
            rotate_refresh_token,
            save_refresh_token,
        },
    },
    redis::RedisClient,
    error::AppError,
};
use crate::domains::account::{
    dto::{
        AccountResponseDto,
        CreateAccountDto,
        LoginAccountDto,
    },
    repository::{
        AccountRepository,
        CreateAccountParams,
    },
};

// Controller에서 Access Token은 Response Body로,
// Refresh Token은 Cookie로 전달하기 위해 사용
pub struct TokenResult {
    pub access_token: String,
    pub refresh_token: String,
}

pub struct AccountService {
    account_repository: AccountRepository,
    jwt: JwtConfig,
    redis: RedisClient,
}

impl AccountService {
    pub fn new(db_pool: PgPool, jwt: JwtConfig, redis: RedisClient,) -> Self {
        Self {
            account_repository: AccountRepository::new(db_pool),
            jwt,
            redis,
        }
    }

    // AuthenticatedUser Extractor에서 JWT 검증에 사용
    pub fn jwt(&self) -> &JwtConfig {
        &self.jwt
    }

    // 이메일 앞뒤 공백 제거 및 소문자 변환
    fn normalize_email(email: &str) -> String {
        email.trim().to_lowercase()
    }

    // 계정 생성
    pub async fn create_account(&self, payload: &CreateAccountDto) -> Result<(), AppError> {
        // 이메일 중복 검사
        let email = Self::normalize_email(&payload.email);
        if self.account_repository.exists_by_email(&email).await? {
            return Err(AppError::BadRequest(
                "이미 사용 중인 이메일입니다.".into(),
            ));
        }

        // 계정 생성
        let role = AccountRole::try_from(payload.role.as_str())?;
        let password_hash = hash_password(&payload.password)?;
        let name = payload.name.trim().to_owned();
        let params = CreateAccountParams::new(
            email,
            password_hash,
            name,
        );

        let mut tx = self.account_repository.begin().await?;

        let account_id = self
            .account_repository
            .save(&mut tx, params)
            .await?;

        self.account_repository.assign_role(&mut tx, account_id, &role).await?;

        tx.commit().await?;
        Ok(())
    }

    // 로그인
    pub async fn login(&self, payload: &LoginAccountDto) -> Result<TokenResult, AppError> {
        // 1. 이메일 정규화
        let email = Self::normalize_email(&payload.email);

        // 2. 이메일로 계정 조회
        let account = self
            .account_repository
            .find_by_email(&email)
            .await?
            .ok_or(AppError::InvalidCredentials)?;


        // 3. 평문 비밀번호와 DB의 Argon2 해시 비교
        let password_valid = verify_password(
            &payload.password,
            &account.password_hash,
        )?;

        // 4. 비밀번호가 틀리면 인증 실패
        if !password_valid {
            return Err(AppError::InvalidCredentials);
        }

        // 5. 계정에 연결된 모든 Role 조회
        let roles = self
            .account_repository
            .find_roles_by_account_id(account.id)
            .await?;

        if roles.is_empty() {
            return Err(AppError::Internal(format!(
                "로그인 계정에 연결된 Role이 없습니다. account_id={}",
                account.id
            )));
        }

        // 6. Access Token 발급
        let access_token = self
            .jwt
            .create_access_token(
                account.id,
                roles,
            )?;

        // 7. Refresh Token 발급 및 Redis에 갱신
        let refresh_token = generate_refresh_token()?;

        save_refresh_token(
            &self.redis,
            &refresh_token,
            account.id,
        )
        .await?;

        Ok(TokenResult {
            access_token,
            refresh_token,
        })
    }

    // jwt 토큰 안에 id 값으로 계정 조회
    pub async fn get_me(&self, account_id: i64) -> Result<AccountResponseDto, AppError> {
        let entity = self
            .account_repository
            .find_by_id(account_id)
            .await?;
        Ok(entity.into())
    }

    /*
        Refresh API 정책
        - 본인의 만료된 Access Token와 본인의 유효한 Refresh Token으로 Access Token 재발급
        - Refresh Token은 Rotation하여 기존 Refresh Token을 폐기하고 새로운 Refresh Token 발급
        - 새로운 Refresh Token의 TTL은 기존 Refresh Token과 동일하게 갱신
        - Refresh Token 자체가 만료되는 경우엔 다시 로그인해야 함
    */
    pub async fn refresh(&self, access_token: &str, refresh_token: &str) -> Result<TokenResult, AppError> {
        // 1. 만료된 Access Token의 서명과 Claims 검증
        let claims = self
            .jwt
            .verify_expired_access_token(access_token)?;

        // 2. Access Token이 실제로 만료되었는지 확인
        let now = chrono::Utc::now().timestamp() as usize;

        if claims.exp >= now {
            tracing::warn!(
                account_id = claims.sub,
                exp = claims.exp,
                now,
                "Refresh 실패: Access Token이 아직 만료되지 않았습니다."
            );
            return Err(AppError::Unauthorized);
        }

        // 3. Refresh Token 검증 및 Redis에서 account_id 조회
        let refresh_account_id = find_refresh_token(
            &self.redis,
            refresh_token,
        )
        .await?
        .ok_or_else(|| {
            tracing::warn!(
                account_id = claims.sub,
                "Refresh 실패: 유효한 Refresh Token을 찾을 수 없습니다."
            );

            AppError::Unauthorized
        })?;

        // 4. Access Token의 account_id와 Refresh Token의 account_id 일치 여부 확인
        if claims.sub != refresh_account_id {
            tracing::warn!(
                access_account_id = claims.sub,
                refresh_account_id,
                "Refresh 실패: Access Token과 Refresh Token의 계정이 일치하지 않습니다."
            );

            return Err(AppError::Unauthorized);
        }

        // 5. 현재 계정의 Role 조회
        let roles = self
            .account_repository
            .find_roles_by_account_id(refresh_account_id)
            .await?;

        if roles.is_empty() {
            tracing::error!(
                account_id = refresh_account_id,
                "Refresh 실패: 계정에 연결된 Role이 없습니다."
            );
            return Err(AppError::Internal(format!(
                "로그인 계정에 연결된 Role이 없습니다. account_id={}",
                refresh_account_id
            )));
        }

        // 6. 새 Access Token 발급
        let access_token = self
            .jwt
            .create_access_token(refresh_account_id, roles)?;

        // 7. 새로운 Refresh Token 발급
        let new_refresh_token = generate_refresh_token()?;

        // 8. 기존 Refresh Token 폐기 + 새로운 Refresh Token 저장
        rotate_refresh_token(
            &self.redis,
            refresh_token,
            &new_refresh_token,
            refresh_account_id,
        )
        .await?;

        // 9. 새 Access Token과 새로운 Refresh Token 반환
        Ok(TokenResult {
            access_token,
            refresh_token: new_refresh_token,
        })
    }

    // 로그아웃
    pub async fn logout(&self, account_id: i64, refresh_token: &str) -> Result<(), AppError> {
        // 1. Refresh Token 검증 및 Redis에서 account_id 조회
        let refresh_account_id = find_refresh_token(&self.redis, refresh_token)
            .await?
            .ok_or_else(|| {
                tracing::warn!(
                    account_id,
                    "Logout 실패: 유효한 Refresh Token을 찾을 수 없습니다."
                );

                AppError::Unauthorized
            })?;

        // 2. Access Token의 account_id와 Refresh Token의 account_id 일치 여부 확인
        if account_id != refresh_account_id {
            tracing::warn!(
                access_account_id = account_id,
                refresh_account_id,
                "Logout 실패: Access Token과 Refresh Token의 계정이 일치하지 않습니다."
            );

            return Err(AppError::Unauthorized);
        }

        // 3. 현재 Refresh Token Session 삭제
        delete_refresh_token(&self.redis, refresh_token, account_id).await?;

        Ok(())
    }
}
