// https://docs.rs/argon2/latest/argon2/
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use crate::common::error::AppError;

// 평문 -> Argon2id 해시 문자열로 변환
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng); // OsRng: 운영체제에서 제공하는 난수 생성기
    let argon2 = Argon2::default(); // Argon2::default() → Argon2id + 기본 파라미터(메모리/시간/병렬성) 사용
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AppError::Internal("비밀번호 해싱에 실패했습니다.".into()))?
        .to_string();

    Ok(password_hash)
}

// 평문 비밀번호와 DB에 저장된 Argon2 해시 비교
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, AppError> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|_| AppError::Internal("비밀번호 해시 형식이 올바르지 않습니다.".into()))?;

    let argon2 = Argon2::default();

    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// cargo test --lib common::security::password
// cargo test --lib security::password -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PASSWORD: &str = "password123!";

    // 1. 시나리오: 정상적인 평문 비밀번호를 해싱하고, 같은 비밀번호로 검증했을 때 성공하는지 확인
    #[test]
    fn hash_and_verify_success() {
        let hash = hash_password(TEST_PASSWORD).expect("비밀번호 해싱 실패");

        assert_ne!(hash, TEST_PASSWORD); // 해시값이 원문과 달라야 함
        assert!(hash.starts_with("$argon2id$")); // 해시가 Argon2id 포맷($argon2id$)으로 시작해야 함

        // 해싱된 값을 verify_password에 넣으면 true가 나와야 함
        let verified = verify_password(TEST_PASSWORD, &hash).expect("비밀번호 검증 실패");
        assert!(verified);
    }

    // 2. 시나리오: 올바른 비밀번호로 해싱한 해시에, 틀린 평문 비밀번호로 검증을 시도했을 때 실패(false)하는지 확인
    #[test]
    fn verify_wrong_password_failed() {
        let hash = hash_password(TEST_PASSWORD).expect("비밀번호 해싱 실패");
        let verified = verify_password("틀린 비번", &hash).expect("비밀번호 검증 실패");
        assert!(!verified);
    }

    // 3. 시나리오: 동일한 평문 비밀번호를 두 번 해싱했을 때, salt가 매번 랜덤 생성되므로 서로 다른 해시값이 나오는지 확인
    // - 레인보우 테이블 공격 방법: 평문-해시 쌍을 미리 계산해두고 역조회하는 공격 → salt로 매번 해시를 다르게 만들어 방어
    #[test]
    fn same_password_generates_different_hash() {
        let hash1 = hash_password(TEST_PASSWORD).expect("비밀번호 해싱 실패");
        let hash2 = hash_password(TEST_PASSWORD).expect("비밀번호 해싱 실패");

        assert_ne!(hash1, hash2); // 같은 값이라도 해시값이 달라야 성공
    }
}
