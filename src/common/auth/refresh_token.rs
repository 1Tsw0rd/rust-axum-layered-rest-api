use crate::common::{error::AppError, redis::RedisClient};
use sha2::{
    Digest, // 해시 알고리즘에서 공통으로 사용하는 기능을 제공하는 trait
    Sha256, // SHA-256 해시 알고리즘
};

// Refresh Token에 사용할 랜덤 바이트 길이(32 bytes = 256 bits)
const REFRESH_TOKEN_BYTES: usize = 32;
// Refresh Token 유효기간: 3일
const REFRESH_TOKEN_EXPIRES_IN_SECONDS: u64 = 60 * 60 * 24 * 3;

// Refresh Token hash를 Redis Key 문자열로 생성
fn generate_refresh_key(token_hash: &str) -> String {
    format!("refresh:{}", token_hash)
}

// 계정별 Refresh Token hash를 Redis Key 문자열로 생성
fn generate_account_refresh_key(account_id: i64) -> String {
    format!("account:{}:refresh", account_id)
}

// 랜덤 Refresh Token 생성: OS가 제공하는 난수원에서 32바이트 랜덤 데이터 생성
pub fn generate_refresh_token() -> Result<String, AppError> {
    let mut bytes = [0u8; REFRESH_TOKEN_BYTES]; // 0으로 초기화된 u8 타입의 32바이트 값을 가변 변수에 저장

    // OS 난수원에서 랜덤 바이트 데이터 생성
    getrandom::fill(&mut bytes)
        .map_err(|_| AppError::Internal("Refresh Token 생성에 실패했습니다.".into()))?;

    // println!("랜덤 bytes = {:?}", bytes);
    // println!("bytes 길이 = {}", bytes.len());

    // Client에 전달하기 위해 hex 문자열로 변환
    Ok(bytes.iter().map(|byte| format!("{:02x}", byte)).collect())
}

// Refresh Token 원문을 SHA-256로 해싱하여 Redis Key로 사용
pub fn hash_refresh_token(token: &str) -> String {
    let hash = Sha256::digest(token.as_bytes());
    hash.iter().map(|byte| format!("{:02x}", byte)).collect()
}

// Refresh Token을 Redis에 저장하고 기존 계정 Refresh Token을 교체
// refresh:{token_hash} -> account_id
// account:{account_id}:refresh -> token_hash
pub async fn save_refresh_token(
    redis: &RedisClient,
    refresh_token: &str,
    account_id: i64,
) -> Result<(), AppError> {
    // redis에 계정의 현재 Refresh Token hash를 조회하고 기존 Redis Key 생성
    // 1. account:{id}:refresh GET
    let account_key = generate_account_refresh_key(account_id);
    let old_hash: Option<String> = redis.get(&account_key).await?;
    // 2. old_key 생성
    let old_key = old_hash.as_deref().map(generate_refresh_key);

    // 새 Refresh Token hash를 Redis Key로 생성
    // 3. new_key 생성: refresh:{token_hash}
    let token_hash = hash_refresh_token(refresh_token);
    let new_key = generate_refresh_key(&token_hash);

    // 4. MULTI/EXEC
    // - old refresh key 존재할 경우 삭제
    // - new refresh key → account_id 저장
    // - account key → new hash 저장
    redis
        .atomic_pipeline(|pipeline| {
            // 기존 Refresh Token이 있으면 삭제
            if let Some(old_key) = old_key.as_deref() {
                pipeline.del(old_key).ignore(); // ignore()은 반환값 무시
            }
            // refresh:{token_hash} → account_id
            pipeline
                .set_ex(&new_key, account_id, REFRESH_TOKEN_EXPIRES_IN_SECONDS)
                .ignore();
            // account:{account_id}:refresh → token_hash
            pipeline
                .set_ex(&account_key, &token_hash, REFRESH_TOKEN_EXPIRES_IN_SECONDS)
                .ignore();
        })
        .await?;

    Ok(())
}

// 기존 Refresh Token은 폐기하고 새로운 Refresh Token으로 교체
// - 동일한 Refresh Token의 재사용을 방지
pub async fn rotate_refresh_token(
    redis: &RedisClient,
    old_refresh_token: &str,
    new_refresh_token: &str,
    account_id: i64,
) -> Result<(), AppError> {
    // 1. 기존 Refresh Token hash 생성
    let old_token_hash = hash_refresh_token(old_refresh_token);

    // 2. 새로운 Refresh Token hash 생성
    let new_token_hash = hash_refresh_token(new_refresh_token);

    // 3. Redis Key 생성
    let old_refresh_key = generate_refresh_key(&old_token_hash);
    let new_refresh_key = generate_refresh_key(&new_token_hash);
    let account_refresh_key = generate_account_refresh_key(account_id);

    // 4. 현재 계정에 저장된 Refresh Token hash 조회
    let current_token_hash: Option<String> = redis.get(&account_refresh_key).await?;

    if current_token_hash.as_deref() != Some(old_token_hash.as_str()) {
        tracing::warn!(
            account_id,
            "Refresh Token Rotation 실패: 현재 계정의 Refresh Token과 요청 Refresh Token이 일치하지 않습니다."
        );

        return Err(AppError::Unauthorized);
    }

    // 5. redis에 refresh token 갱신
    redis
        .atomic_pipeline(|pipeline| {
            // 기존 Refresh Token 삭제
            pipeline.del(&old_refresh_key).ignore();
            // 새로운 Refresh Token -> account_id
            pipeline
                .set_ex(
                    &new_refresh_key,
                    account_id,
                    REFRESH_TOKEN_EXPIRES_IN_SECONDS,
                )
                .ignore();
            // account:{account_id}:refresh -> 새로운 Refresh Token hash
            pipeline
                .set_ex(
                    &account_refresh_key,
                    &new_token_hash,
                    REFRESH_TOKEN_EXPIRES_IN_SECONDS,
                )
                .ignore();
        })
        .await?;

    Ok(())
}

// Refresh Token을 검증하여 연결된 account_id 조회
pub async fn find_refresh_token(
    redis: &RedisClient,
    refresh_token: &str,
) -> Result<Option<i64>, AppError> {
    // 1. Refresh Token hash 생성
    let token_hash = hash_refresh_token(refresh_token);
    // 2. refresh:{token_hash} Key 생성
    let key = generate_refresh_key(&token_hash);
    // 3. Redis에서 account_id 조회
    let account_id = redis.get(&key).await?;
    // 4. 값이 있으면 i64로 변환
    match account_id {
        Some(value) => {
            let account_id = value.parse::<i64>().map_err(|_| {
                AppError::Internal("Redis에 저장된 account_id 형식이 올바르지 않습니다.".into())
            })?;

            Ok(Some(account_id))
        }
        None => Ok(None),
    }
}

// 현재 계정의 Refresh Token Session 삭제
pub async fn delete_refresh_token(
    redis: &RedisClient,
    refresh_token: &str,
    account_id: i64,
) -> Result<(), AppError> {
    // 1. Refresh Token hash 생성
    let token_hash = hash_refresh_token(refresh_token);

    // 2. Redis Key 생성
    let refresh_key = generate_refresh_key(&token_hash);
    let account_refresh_key = generate_account_refresh_key(account_id);

    // 3. Redis에서 관리중인 해당 계정의 Refresh Token 정보 삭제
    redis
        .atomic_pipeline(|pipeline| {
            pipeline.del(&refresh_key).ignore();
            pipeline.del(&account_refresh_key).ignore();
        })
        .await?;

    Ok(())
}

// cargo test --lib common::auth::refresh_token
// cargo test --lib common::auth::refresh_token -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;

    // 시나리오 1: Refresh Token이 정상적으로 생성되고, 32바이트를 hex로 변환한 64글자 문자열인지 검증
    #[test]
    fn generated_refresh_token_is_not_empty() {
        let token = generate_refresh_token().expect("Refresh Token 생성 실패");

        // println!("Refresh Token(hex) = {}", token);
        // println!("hex 문자열 길이 = {}", token.len());
        // println!("원래 바이트 길이 = {}", token.len() / 2);

        assert!(!token.is_empty());
        assert_eq!(token.len(), REFRESH_TOKEN_BYTES * 2);
    }

    // 시나리오 2: Refresh Token을 연속으로 생성했을 때 서로 다른 랜덤 값이 생성되는지 검증
    #[test]
    fn generated_refresh_tokens_are_different() {
        let token1 = generate_refresh_token().expect("Refresh Token 생성 실패");

        let token2 = generate_refresh_token().expect("Refresh Token 생성 실패");

        assert_ne!(token1, token2);
    }

    // 시나리오 3: 동일한 Refresh Token을 해싱하면 항상 동일한 SHA-256 hash가 생성되는지 검증
    #[test]
    fn same_token_produces_same_hash() {
        let token = "test-refresh-token";

        let hash1 = hash_refresh_token(token);
        let hash2 = hash_refresh_token(token);

        assert_eq!(hash1, hash2);
    }

    // 시나리오 4: 서로 다른 Refresh Token을 해싱하면 서로 다른 SHA-256 hash가 생성되는지 검증
    #[test]
    fn different_tokens_produce_different_hashes() {
        let hash1 = hash_refresh_token("token-a");
        let hash2 = hash_refresh_token("token-b");

        assert_ne!(hash1, hash2);
    }
}
