use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

use crate::common::auth::role::AccountRole;
use crate::domains::account::entity::AccountEntity;

// 비밀번호는 3가지 조합 + 8자 이상
fn validate_password(password: &str) -> Result<(), ValidationError> {
    // 공백 허용하지 않음
    if password.chars().any(|c| c.is_whitespace()) {
        return Err(ValidationError::new("password_whitespace"));
    }

    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    // ASCII 영숫자(A-Z, a-z, 0-9)가 아닌 문자를 특수문자 조건으로 인정
    // 따라서 일반 기호뿐 아니라 한글·이모지 같은 Unicode 문자도 이 조건을 충족할 수 있음
    let has_special = password.chars().any(|c| !c.is_ascii_alphanumeric());

    let combination_count = [
        has_uppercase,
        has_lowercase,
        has_digit,
        has_special
    ]
    .into_iter()
    .filter(|&value| value)
    .count();

    if combination_count < 3 {
        return Err(ValidationError::new("password_combination"));
    }

    Ok(())
}

// 공백만으로 min 길이를 채우는 것을 막기 위해 trim한 길이로 검사
// (실제 저장 시 service.rs에서 trim()하므로, 검증 기준도 trim 후 길이에 맞춤)
fn validate_name(name: &str) -> Result<(), ValidationError> {
    let len = name.trim().chars().count();

    if !(1..=50).contains(&len) {
        return Err(ValidationError::new("name_length"));
    }

    Ok(())
}

// admin은 request로 생성하지 못하도록 함
fn validate_role(role: &str) -> Result<(), ValidationError> {
    let role = AccountRole::try_from(role)
        .map_err(|_| ValidationError::new("invalid_role"))?;

    if role == AccountRole::Admin {
        tracing::warn!("회원가입에서는 Admin을 허용하지 않음");
        return Err(ValidationError::new("invalid_role"));
    }

    Ok(())
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateAccountDto {
    #[validate(email(message = "올바른 이메일 형식이어야 합니다."))]
    pub email: String,

    #[validate(
        length(min = 8,max = 100, message = "비밀번호는 8자 이상, 100자 이하여야 합니다."),
        custom(function = "validate_password", message = "비밀번호는 공백 없이 영문 대문자, 영문 소문자, 숫자, 특수문자 중 3가지 이상을 포함해야 합니다.")
    )]
    pub password: String,

    #[validate(custom(function = "validate_name", message = "이름은 1자 이상, 50자 이하여야 합니다."))]
    pub name: String,

    #[validate(custom(function = "validate_role", message = "요청이 올바르지 않습니다."))]
    pub role: String,
}

// 로그인 요청 DTO
#[derive(Debug, Deserialize, Validate)]
pub struct LoginAccountDto {
    #[validate(email(message = "올바른 이메일 형식이 아닙니다."))]
    pub email: String,

    #[validate(length(min = 1, message = "비밀번호를 입력해주세요."))]
    pub password: String,
}

// 로그인 응답 DTO
// Access Token만 JSON Response Body로 반환하고,
// Refresh Token은 Controller에서 HttpOnly Cookie로 전달
#[derive(Debug, Serialize)]
pub struct TokenResponseDto {
    pub access_token: String,
}

#[derive(Debug, Serialize)]
pub struct AccountResponseDto {
    pub id: i64,
    pub email: String,
    pub name: String,
}

impl From<AccountEntity> for AccountResponseDto {
    fn from(entity: AccountEntity) -> Self {
        Self {
            id: entity.id,
            email: entity.email,
            name: entity.name,
        }
    }
}

// cargo test --lib domains::account::dto
// cargo test --lib domains::account::dto -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;

    // 시나리오 1: 대문자·소문자·숫자·특수문자 중 3가지 이상 조합을 포함하면 비밀번호 검증 통과
    #[test]
    fn password_with_3_or_more_combinations_passes() {
        // 대문자 + 소문자 + 숫자
        assert!(validate_password("Abcdefg1").is_ok());

        // 대문자 + 소문자 + 특수문자
        assert!(validate_password("ABCDefg!").is_ok());

        // 대문자 + 숫자 + 특수문자
        assert!(validate_password("ABCDEFG1!").is_ok());

        // 소문자 + 숫자 + 특수문자
        assert!(validate_password("abcdefg1!").is_ok());

        // 대문자 + 소문자 + 숫자 + 특수문자
        assert!(validate_password("Abcdefg1!").is_ok());
    }

    // 시나리오 2: 비ASCII Unicode 문자도 특수문자 조건을 충족할 수 있다.
    #[test]
    fn password_with_non_ascii_as_special_passes() {
        // 대문자 + 숫자 + 한글
        assert!(validate_password("ABCDEFG1가").is_ok());

        // 소문자 + 숫자 + 한글
        assert!(validate_password("abcdefg1가").is_ok());

        // 대문자 + 소문자 + 한글
        assert!(validate_password("ABCDefg가").is_ok());

        // 소문자 + 숫자 + 이모지
        assert!(validate_password("abcdefg1🙂").is_ok());
    }

    // 시나리오 3: 1가지 조합만 포함하면 실패
    #[test]
    fn password_with_1_combination_fails() {
        assert!(validate_password("abcdefgh").is_err()); // 소문자만
        assert!(validate_password("ABCDEFGH").is_err()); // 대문자만
        assert!(validate_password("12345678").is_err()); // 숫자만
        assert!(validate_password("!!!!!!!!").is_err()); // 특수문자만
    }

    // 시나리오 4: 2가지 조합만 포함하면 실패
    #[test]
    fn password_with_2_combinations_fails() {
        assert!(validate_password("abcdefg1").is_err());   // 소문자 + 숫자
        assert!(validate_password("ABCDEFG1").is_err());   // 대문자 + 숫자
        assert!(validate_password("ABCDefgh").is_err());   // 대문자 + 소문자
        assert!(validate_password("abcdefg!").is_err());   // 소문자 + 특수문자
        assert!(validate_password("ABCDEFG!").is_err());   // 대문자 + 특수문자
        assert!(validate_password("1234567!").is_err());   // 숫자 + 특수문자
    }

    // 시나리오 5: 공백이 포함되면 실패해야 함
    #[test]
    fn password_with_whitespace_fails() {
        assert!(validate_password("Abcdefg 1").is_err()); // 중간에 공백
        assert!(validate_password("Abcdefg1 ").is_err()); // 끝에 공백
        assert!(validate_password(" Abcdefg1").is_err()); // 앞에 공백
    }

    // 시나리오 6: 이메일/비밀번호/이름/Role이 모두 유효하면 DTO 전체 검증 통과
    #[test]
    fn create_account_dto_valid() {
        let dto = CreateAccountDto {
            email: "test@example.com".into(),
            password: "Password1!".into(),
            name: "홍길동".into(),
            role: "customer".into(),
        };
        assert!(dto.validate().is_ok()); // Validate trait validate() 메서드로 검사
    }

    // 시나리오 7: 이메일 형식이 올바르지 않으면 DTO 전체 검증이 실패하고, 실패 필드가 email인지 확인
    #[test]
    fn create_account_dto_invalid_email_fails() {
        let dto = CreateAccountDto {
            email: "not-an-email".into(),
            password: "Password1!".into(),
            name: "홍길동".into(),
            role: "customer".into(),
        };
        let result = dto.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().field_errors().contains_key("email"));
    }

    // 시나리오 8: 비밀번호가 조합 조건을 못 채우면 DTO 전체 검증이 실패해야 함
    #[test]
    fn create_account_dto_invalid_password() {
        let dto = CreateAccountDto {
            email: "test@example.com".into(),
            password: "password".into(), // 소문자만, 조합 미달
            name: "홍길동".into(),
            role: "customer".into(),
        };
        assert!(dto.validate().is_err());
    }

   // 시나리오 9: 비밀번호가 8자 미만이면 조합을 충족해도 DTO 전체 검증이 실패해야 함
    #[test]
    fn create_account_dto_password_too_short() {
        let dto = CreateAccountDto {
            email: "test@example.com".into(),
            password: "Aa1!".into(), // 4가지 조합은 만족하지만 4자뿐
            name: "홍길동".into(),
            role: "customer".into(),
        };
        let result = dto.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().field_errors().contains_key("password"));
    }

    // 시나리오 10: 비밀번호가 100자를 초과하면 DTO 전체 검증이 실패해야 함
    #[test]
    fn create_account_dto_password_too_long() {
        let long_password = format!("Aa1!{}", "a".repeat(100)); // 조합은 만족하지만 100자 초과
        let dto = CreateAccountDto {
            email: "test@example.com".into(),
            password: long_password,
            name: "홍길동".into(),
            role: "customer".into(),
        };
        assert!(dto.validate().is_err());
    }

    // 시나리오 11: customer Role은 가입 가능
    #[test]
    fn create_account_customer_role_passes() {
        let dto = CreateAccountDto {
            email: "test@example.com".into(),
            password: "Password1!".into(),
            name: "홍길동".into(),
            role: "customer".into(),
        };

        assert!(dto.validate().is_ok());
    }

    // 시나리오 12: employee Role은 가입 가능
    #[test]
    fn create_account_employee_role_passes() {
        let dto = CreateAccountDto {
            email: "test@example.com".into(),
            password: "Password1!".into(),
            name: "홍길동".into(),
            role: "employee".into(),
        };

        assert!(dto.validate().is_ok());
    }

    // 시나리오 13: 일반 회원가입에서 admin Role은 선택할 수 없음
    #[test]
    fn create_account_admin_role_fails() {
        let dto = CreateAccountDto {
            email: "test@example.com".into(),
            password: "Password1!".into(),
            name: "홍길동".into(),
            role: "admin".into(),
        };

        let result = dto.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().field_errors().contains_key("role"));
    }

    // 시나리오 14: 존재하지 않는 Role은 실패
    #[test]
    fn create_account_unknown_role_fails() {
        let dto = CreateAccountDto {
            email: "test@example.com".into(),
            password: "Password1!".into(),
            name: "홍길동".into(),
            role: "unknown".into(),
        };

        let result = dto.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().field_errors().contains_key("role"));
    }

    // 시나리오 15: 이메일/비밀번호가 모두 유효하면 LoginAccountDto 검증을 통과
    #[test]
    fn login_account_dto_valid() {
        let dto = LoginAccountDto {
            email: "test@example.com".into(),
            password: "anything".into(),
        };
        assert!(dto.validate().is_ok());
    }

    // 시나리오 16: 이메일 형식이 올바르지 않으면 LoginAccountDto 검증이 실패
    #[test]
    fn login_account_dto_invalid_email_fails() {
        let dto = LoginAccountDto {
            email: "not-an-email".into(),
            password: "anything".into(),
        };
        assert!(dto.validate().is_err());
    }

    // 시나리오 17: 비밀번호가 비어있으면 LoginAccountDto 검증이 실패
    #[test]
    fn login_account_dto_empty_password_fails() {
        let dto = LoginAccountDto {
            email: "test@example.com".into(),
            password: "".into(),
        };
        assert!(dto.validate().is_err());
    }
}
