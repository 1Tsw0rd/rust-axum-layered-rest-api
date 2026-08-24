use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use crate::domains::product::entity::ProductEntity;

// =========================================================================
// [아키텍처 결정 레코드 (ADR) 및 타입 설계 철학 명세]
//
// 1. [타입 불일치와 양보]:
//    최초 설계 시 양수만 안전하게 제어하기 위해 Rust 코드 내 u64/u32 타입을 지향했으나,
//    현재 PostgreSQL 엔진은 부호 없는 정수(Unsigned)를 지원하지 않음
//    인프라 레이어의 물리적 스펙을 존중하고 일관성을 확보하기 위해 전 계층 타입을 i64/i32로 동기화함
//
// 2. [침묵의 실패(Silent Failure)와 중복 검증의 모순 해결]:
//    u64에서 i64로 단순 강제 캐스팅(as) 적용 시, u64::MAX 같은 값이 에러 없이
//    비트는 그대로 유지된 채 부호만 반전되어 해석됨(예: u64::MAX(0xFFFF...FFFF) → i64로는 -1)
//    이로 인해 '침묵의 데이터 오염'이 발생할 수 있음
//    이를 막기 위해 서비스/레포지토리 단마다 try_into() 검증 코드를 추가하려 했으나, 
//    필드가 늘어날 때마다 검증 소음이 비대해지는 아키텍처적 모순이 발생함
//
// 3. [최종 결론 - Validator 다중 방어선 확립]:
//    DB에 적재된 정보는 언제나 오염되지 않은 '가장 정확한 진실의 데이터'로 간주함
//    따라서 내부 물줄기는 인프라 타입(i64/i32)에 그대로 맞춰 형변환 노이즈를 0%로 격리하되,
//    음수 유입 방어선은 최외곽 진입점 DTO에서 #[validate(range(min = 1))] 매크로가 단 한 번 전담 통제함
//
// 4. [향후 확장성 및 인프라 매핑 가이드 (ClickHouse 배포 시 주의점)]:
//    추후 대용량 로그/시계열 통계 처리를 위해 ClickHouse 등 Unsigned(UInt64/UInt32)를 
//    공식 지원하는 가상 인프라가 추가 탑재될 경우, 해당 DB의 물리 스펙(u64/u32)을 존중하여 처리함이 올바르다고 판단
//    단, ClickHouse는 컬럼별로 파일을 저장·관리하는 대용량 압축 적재(OLAP)에 특화된 구조로 설계되어 있어,
//    전형적인 웹 서비스의 '단건 행 수정(UPDATE)' 및 '단건 삭제(DELETE)' 연산 시 
//    전체 데이터 블록을 다시 굽는 엄청난 디스크 I/O 병목(성능 저하)이 발생함
//    따라서 잦은 단건 수정이 일어나는 '메인 제품 도메인'은 본 PostgreSQL(RDBMS)를 유지하고,
//    ClickHouse는 수정/삭제가 일어나지 않는 순수 거대 Append-Only(로그, 지표 수집) 영역에만 격리 구현하고자 함
// =========================================================================

// 공백만으로 min 길이를 채우는 것을 막기 위해 trim한 길이로 검사
// (실제 저장 시 repository.rs에서 trim()하므로, 검증 기준도 trim 후 길이에 맞춤)
fn validate_name(name: &str) -> Result<(), ValidationError> {
    let len = name.trim().chars().count();

    if !(1..=100).contains(&len) {
        return Err(ValidationError::new("name_length"));
    }

    Ok(())
}

// ===== [요청 DTO] =====
#[derive(Deserialize, Validate)]
pub struct ProductPathId {
    #[validate(range(min = 1, message = "id는 1 이상이어야 합니다."))]
    pub id: i64,
}

// 1. [Request-GET] 제품 목록 조회 Query
#[derive(Debug, Deserialize, Validate)]
pub struct ProductListQuery {
    #[validate(range(min = 1, message = "page는 1 이상이어야 합니다."))]
    pub page: i64, // page와 size의 postgres에서 i64 타입

    #[validate(range(min = 1, max = 100, message = "size는 1~100 사이여야 합니다."))]
    pub size: i64,

    #[validate(length(max = 20, message = "검색어는 최대 20자까지 입력할 수 있습니다."))]
    pub keyword: Option<String>,
}

// 2. [Request-POST] 제품 생성 DTO
#[derive(Deserialize, Validate)]
pub struct CreateProductDto {
    #[validate(custom(function = "validate_name", message = "제품명은 1자 이상, 100자 이하여야 합니다."))]
    pub name: String,

    #[validate(length(max = 1000, message = "제품 설명은 최대 1000자입니다."))]
    pub description: String,

    #[validate(range(min = 0, message = "가격은 0 이상이어야 합니다."))]
    pub price: i64,
}

// 3. [Request-PUT] 제품 전체 수정 DTO
#[derive(Debug, Deserialize, Validate)]
pub struct ReplaceProductDto {
    #[validate(custom(function = "validate_name", message = "제품명은 1자 이상, 100자 이하여야 합니다."))]
    pub name: String,

    #[validate(length(max = 1000, message = "제품 설명은 최대 1000자입니다."))]
    pub description: String,

    #[validate(range(min = 0, message = "가격은 0 이상이어야 합니다."))]
    pub price: i64,
}

// 4. [Request-PATCH] 제품 일부 수정 DTO
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductDto {
    #[validate(custom(function = "validate_name", message = "제품명은 1자 이상, 100자 이하여야 합니다."))]
    pub name: Option<String>,

    #[validate(length(max = 1000, message = "제품 설명은 최대 1000자입니다."))]
    pub description: Option<String>,

    #[validate(range(min = 0, message = "가격은 0 이상이어야 합니다."))]
    pub price: Option<i64>,
}

// ===== [응답 DTO] =====
// Serialize: Rust 메모리에 있는 구조체 -> 브라우저가 읽을 수 있는 JSON 문자열로 변환(Deserialize의 반대 과정)
#[derive(Debug, Serialize)]
pub struct ProductResponseDto {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub price: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Entity -> Response DTO 변환
// Rust 표준 라이브러리가 제공하는 "안전한 타입 변환 규격(인터페이스)"인 From 트레이트 구현
// 컴파일러에게 ProductEntity 데이터를 줄테니, ProductResponseDto 객체로 필드를 복사해서 조립하라고 Self {} 규칙 알려줌
// 이렇게 딱 한번 정의하면 컴파일러가 반대 방향인 .into() 메서드를 자동 생성
// 서비스 레이어에서는 let dto: ProductResponseDto = entity.into(); 한 줄로 변환 가능
impl From<ProductEntity> for ProductResponseDto {
    fn from(entity: ProductEntity) -> Self {
        Self {
            id: entity.id,
            name: entity.name,
            description: entity.description,
            price: entity.price,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        }
    }
}

// 참고:
// 응답 형태를 다르게 주고 싶은 경우 (예: 다른 필드 포함, 필드 일부 제외 등)
// 새로운 Response DTO를 만들고 From을 따로 구현
