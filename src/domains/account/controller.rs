use std::sync::Arc;
use axum::{
    extract::State, http::{
        HeaderMap, StatusCode, header::AUTHORIZATION,
    }, response::IntoResponse,
};
use axum_extra::extract::cookie::{
    Cookie,
    CookieJar,
    SameSite,
};
use crate::common::{
    auth::extractor::AuthenticatedUser,
    error::AppError,
    extractors::ValidatedJson,
    response::ApiResponse,
};
use crate::domains::account::{
    dto::{
        CreateAccountDto,
        LoginAccountDto,
        TokenResponseDto,
    },
    service::{
        AccountService,
        TokenResult,
    }
};

// access token은 response body
// refresh token은 cookie에 담아 응답
fn token_response(jar: CookieJar, result: TokenResult) -> impl IntoResponse {
    let refresh_cookie = Cookie::build((
        "refresh_token",
        result.refresh_token,
    ))
        .path("/accounts") // accounts 및 하위 경로에서 Refresh Cookie 전송
        .http_only(true) // JavaScript에서 Cookie 접근 불가
        .secure(true) // HTTPS에서만 Cookie 전송. Nginx 등 앞단 프록시가 TLS를 종단(termination)하는 구조를 전제로 함
        .same_site(SameSite::Strict) // 다른 site의 요청에는 Cookie 전송 제한
        .build();

    let jar = jar.add(refresh_cookie); // Set-Cookie 응답 헤더에 refresh_token 추가

    let response_body = TokenResponseDto {
        access_token: result.access_token,
    };

    (
        jar,
        ApiResponse::success_with_data(
            StatusCode::OK,
            response_body,
        ),
    )
}


// POST: 계정 생성
#[axum::debug_handler]
pub async fn create_account(
    State(account_service): State<Arc<AccountService>>,
    ValidatedJson(payload): ValidatedJson<CreateAccountDto>,
) -> Result<impl IntoResponse, AppError> {
    account_service.create_account(&payload).await?;
    Ok(ApiResponse::success(StatusCode::CREATED))
}

// POST 로그인
// Product는 Entity -> Response DTO가 단순 필드 변환이므로 impl From으로 변환하지만,
// Login은 Entity에서 인증/검증 후 JWT를 생성해야 하므로 단순 변환이 아님
// 따라서 Service에서 Access Token과 Refresh Token을 생성하고,
// Controller에서 Access Token은 JSON Response Body로,
// Refresh Token은 HttpOnly Cookie로 분리하여 전달
#[axum::debug_handler]
pub async fn login(
    State(account_service): State<Arc<AccountService>>,
    jar: CookieJar,
    ValidatedJson(payload): ValidatedJson<LoginAccountDto>,
) -> Result<impl IntoResponse, AppError> {
    let result = account_service.login(&payload).await?;
    Ok(token_response(jar, result))
}

// POST: Access Token 재발급
#[axum::debug_handler]
pub async fn refresh(
    State(account_service): State<Arc<AccountService>>,
    headers: HeaderMap,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let access_token = headers
        .get(AUTHORIZATION) // Authorization Header 조회
        .ok_or_else(|| {
            tracing::warn!(
                "Refresh 실패: Authorization Header가 없습니다."
            );
            AppError::Unauthorized
        })?
        .to_str()  // HeaderValue를 문자열로 변환
        .map_err(|_| {
            tracing::warn!(
                "Refresh 실패: Authorization Header를 문자열로 변환할 수 없습니다."
            );
            AppError::Unauthorized
        })?
        .strip_prefix("Bearer ") // "Bearer " 제거
        .filter(|token| !token.is_empty()) // 토큰이 비어있지 않은지 확인
        .ok_or_else(|| {
            tracing::warn!(
                "Refresh 실패: Authorization Header의 Bearer Token 형식이 올바르지 않습니다."
            );
            AppError::Unauthorized
        })?;

    let refresh_token = jar
        .get("refresh_token") // refresh_token Cookie 조회
        .ok_or_else(|| {
            tracing::warn!("Refresh 실패: refresh_token Cookie가 없습니다.");
            AppError::Unauthorized
        })? // Cookie가 없으면 인증 실패
        .value(); // Cookie의 실제 Refresh Token 값 추출

    let result = account_service.refresh(access_token, refresh_token).await?;

    Ok(token_response(jar, result))
}

// GET: 현재 로그인한 계정 조회
#[axum::debug_handler]
pub async fn get_me(
    State(account_service): State<Arc<AccountService>>,
    AuthenticatedUser { account_id, .. }: AuthenticatedUser,
) -> Result<impl IntoResponse, AppError> {
    // AuthenticatedUser Extractor가 Authorization 헤더의 JWT를 먼저 검증 수행
    // 검증이 성공하면 JWT의 sub(account_id)를 사용할 수 있음
    let account = account_service
        .get_me(account_id)
        .await?;

    Ok(ApiResponse::success_with_data(
        StatusCode::OK,
        account,
    ))
}

// POST: 로그아웃
#[axum::debug_handler]
pub async fn logout(
    State(account_service): State<Arc<AccountService>>,
    AuthenticatedUser { account_id, .. }: AuthenticatedUser,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let refresh_token = jar
        .get("refresh_token") // refresh_token Cookie 조회
        .ok_or_else(|| {
            tracing::warn!(
                account_id,
                "Logout 실패: refresh_token Cookie가 없습니다."
            );

            AppError::Unauthorized
        })?
        .value(); // Cookie의 실제 Refresh Token 값 추출

    account_service.logout(account_id, refresh_token).await?;

    // Refresh Token Cookie 삭제
    let remove_cookie = Cookie::build(("refresh_token", ""))
        .path("/accounts")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .build();

    let jar = jar.remove(remove_cookie);

    Ok((
        jar,
        ApiResponse::success(StatusCode::OK),
    ))
}
