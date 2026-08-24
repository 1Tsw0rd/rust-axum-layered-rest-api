use crate::common::error::AppError;
use serde::{Deserialize, Serialize};

#[rustfmt::skip] // 아래 항목별 설명 주석을 직접 정렬해둔 형태라 cargo fmt가 재배치하지 않도록 고정
#[derive(
    Debug,       // {:?} 매크로로 구조체 내부 값 터미널 로그에 출력하기 위함
    Clone, Copy, // 소유권 이동 없이 값 복사만으로 함수 인자 가볍게 전달
    PartialEq,   // ==, != 연산자를 사용한 값 비교 능력 부여
    Eq,          // HashMap 키나 엄격한 패턴 매칭 검증을 위해 반사성 보장
    Serialize,   // Rust Enum 객체를 외부에 전송 가능한 Json 문자열로 변환
    Deserialize  // 들어온 Json 텍스트를 Rust의 Enum으로 파싱
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
// Serialize / Deserialize 시 enum variant를
// SCREAMING_SNAKE_CASE 형식(대문자와 언더바(_))으로 변환
// 예: Admin → "ADMIN", Employee → "EMPLOYEE"
pub enum AccountRole {
    Admin,
    Customer,
    Employee,
}

// Enum -> 소문자 문자열 변환
impl AccountRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Customer => "customer",
            Self::Employee => "employee",
        }
    }
}

// 소문자 문자열 -> Enum 변환
// From: 변환이 100% 성공할 때 사용
// TryFrom: 변환이 실패할 가능성 있을 때 사용
impl TryFrom<&str> for AccountRole {
    // 실패할 경우 Result Error 타입으로 반환할거라 이렇게 하고
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // 성공할 경우 Result Ok 타입으로 반환
        match value.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "customer" => Ok(Self::Customer),
            "employee" => Ok(Self::Employee),

            _ => {
                tracing::warn!("알 수 없는 계정 역할 값: {}", value);

                Err(AppError::BadRequest(
                    "올바르지 않은 요청 형식입니다.".into(),
                ))
            }
        }
    }
}

// cargo test --lib common::auth::role
// cargo test --lib common::auth::role -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;

    // 시나리오 1: AccountRole의 각 Enum Variant가 정의된 소문자 문자열로 정상 변환되는지 검증
    #[test]
    fn role_to_string() {
        assert_eq!(AccountRole::Admin.as_str(), "admin");
        assert_eq!(AccountRole::Customer.as_str(), "customer");
        assert_eq!(AccountRole::Employee.as_str(), "employee");
    }

    // 시나리오 2: 소문자 및 대소문자가 혼합된 문자열이 올바른 AccountRole Enum으로 정상 변환되는지 검증
    #[test]
    fn string_to_role() {
        assert_eq!(AccountRole::try_from("admin").unwrap(), AccountRole::Admin);

        assert_eq!(
            AccountRole::try_from("CUSTOMER").unwrap(),
            AccountRole::Customer
        );

        assert_eq!(
            AccountRole::try_from("employee").unwrap(),
            AccountRole::Employee
        );
    }

    // 시나리오 3: 정의되지 않은 문자열을 AccountRole로 변환할 때 오류가 발생하는지 검증
    #[test]
    fn invalid_role_fails() {
        assert!(AccountRole::try_from("unknown").is_err());
    }

    // 시나리오 4: AccountRole이 SCREAMING_SNAKE_CASE로 직렬화/역직렬화되는지 검증
    #[test]
    fn serialize_deserialize() {
        let role = AccountRole::Admin;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"ADMIN\"");

        let deserialized: AccountRole = serde_json::from_str("\"EMPLOYEE\"").unwrap();
        assert_eq!(deserialized, AccountRole::Employee);
    }
}
