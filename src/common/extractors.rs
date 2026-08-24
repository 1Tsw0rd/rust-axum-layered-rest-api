use axum::{
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    extract::{FromRequest, FromRequestParts, Json, Path, Query, Request},
    http::request::Parts,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::common::error::AppError;

// Extractor: Client http 요청에서 데이터를 추출
//
// [설계]
//                 HTTP Request
//                      │
//                      ▼
//               Middleware
//       (로그, CORS, 인증, Panic)
//                      │
//                      ▼
//             Extractor(Path/Query/Json)
//                      │
//       ┌──────────────┴──────────────┐
//       │                             │
//       ▼                             ▼
//   성공(Validated DTO)          Rejection 발생
//       │                             │
//       ▼                             ▼
//  Controller                    Error Handler
//       │                             │
//       ▼                             ▼
//  Service                    ApiError 변환
//       │                             │
//       └──────────────┬──────────────┘
//                      ▼
//              ApiResponse JSON

// FromRequestParts: Client Request의 'Body를 제외한 모든 정보' 추출 (URL 경로, 쿼리 스트링, Header 등)
// FromRequest     : Client Request의 'Body(본문) 데이터' 추출 (JSON, Form 등)

/// ---------------------------
/// Json
/// ---------------------------
pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
// ValidationJson<T>에 FromRequest<S> 능력 부여하는데
where
    S: Send + Sync, // S(서버 상태)는 멀티스레드 환경에서 "안전하게 전달(Send) 및 공유(Sync)" 가능해야 함
    T: DeserializeOwned + Validate, // T(데이터 타입)는 "JSON 역직렬화(패싱)"가 가능하고 "유효성 검증(Validate)" 규칙이 있어야 함
{
    type Rejection = AppError; // 에러 발생(Rejection) 시, Axum 기본 에러 대신 내가 만든 AppError로 가공해서 리턴
    // Rejection는 실패했을 때 최종적으로 어떤 타입의 에러 데이터를 컴파일러에게 리턴할지 결정해줌

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Axum의 기본 Json 추출기를 실행하여 HTTP 요청 바디를 파싱 (실패 시 우리가 만든 JSON 에러로 변환)
        let Json(value) = Json::<T>::from_request(req, state) // HTTP 요청 바디를 역직렬화한 뒤, 구조 분해하여 순수한 데이터만 'value' 변수에 담는 문법
            .await
            .map_err(map_json_error)?;

        // 파싱된 데이터(DTO)의 유효성 검증 규칙을 실행 (실패 시 우리가 만든 Validation 에러로 변환)
        value
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        // 추출과 검증을 모두 통과하면 최종적으로 ValidatedJson을 리턴
        Ok(ValidatedJson(value))
    }
}
// =========================================================================
// 💡 [익스트랙터(Extractor) 단계별 실제 데이터 변환 과정 예시 메모]
// =========================================================================
//
// 0. 실습용 데이터 세팅 (DTO)
// pub struct CreateUser {
//     #[validate(length(min = 2, message = "이름은 최소 2글자 이상이어야 합니다."))]
//     pub name: String,
//     pub email: String,
// }
//
// -------------------------------------------------------------------------
// 1. [시작] HTTP Request (req안에 들어있는 날것의 데이터)
// -------------------------------------------------------------------------
// req (HTTP Request) 내부 상태 (텍스트 문자열 상태)
// Headers: Content-Type: application/json
// Body   : "{\"name\": \"Tom\", \"email\": \"tom@test.com\"}"
//
// -------------------------------------------------------------------------
// 2. [1단계] Json::<T>::from_request 실행 직후 (역직렬화)
// -------------------------------------------------------------------------
// Request Body(FromRequest)로부터 <T>(CreateUser)에 해당하는 문자열 데이터를 읽음
// Json( CreateUser { name: "Tom".to_string(), email: "tom@test.com".to_string() } )
//
// -------------------------------------------------------------------------
// 3. [2단계] let Json(value) = ... 실행 직후 (구조 분해 할당)
// -------------------------------------------------------------------------
// 오른쪽 결과인 Json(CreateUser { ... }) 데이터에서
// 안의 진짜 데이터 구조체(CreateUser {...})만 'value'라는 순수한 변수에 대입
// CreateUser {
//     name: "Tom".to_string(),
//     email: "tom@test.com".to_string()
// }
//
// -------------------------------------------------------------------------
// 4. [3단계] value.validate() 실행 (유효성 검증)
// -------------------------------------------------------------------------
// 변수 value에 접근해서 DTO 위에 적어두었던 검증 규칙을 실행
// * 성공 케이스: "Tom"은 2글자 이상이므로 Ok(())가 반환되어 통과
// * 실패 케이스: 만약 name: "T"로 보내면 map_err이 발동되어
//   AppError::Validation("이름은 최소 2글자 이상이어야 합니다.")를 리턴하고 바로 튕겨 나감
//
// -------------------------------------------------------------------------
// 5. [4단계] Ok(ValidatedJson(value)) (최종 리턴)
// -------------------------------------------------------------------------
// 검증까지 완벽하게 끝난 안전한 데이터를 다시 ValidatedJson에 넣어서 감싸줌
// Ok( ValidatedJson( CreateUser { name: "Tom".to_string(), email: "tom@test.com".to_string() } ) )
//
// -> 이 최종 결과물이 컨트롤러 함수의 매개변수인 ValidatedJson(payload) 자리에 꽂히며,
//    컨트롤러 내부에서는 payload.name이나 payload.email로 안전하게 데이터를 꺼내씀
// =========================================================================

/// ---------------------------
/// Path
/// ---------------------------
pub struct ValidatedPath<T>(pub T);

impl<S, T> FromRequestParts<S> for ValidatedPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate + Send, // FromRequest와 달리 FromRequestParts는 T: Send 제약조건을 추가해야 함
{
    type Rejection = AppError;

    // parts는 HTTP 요청에서 Body(본문 데이터)를 제외한 모든 메타데이터를 담고 있는 가변 참조
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(map_path_error)?;

        value
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        Ok(ValidatedPath(value))
    }
}

/// ---------------------------
/// Query
/// ---------------------------
pub struct ValidatedQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(map_query_error)?;

        value
            .validate()
            .map_err(|e| AppError::Validation(e.to_string()))?;

        Ok(ValidatedQuery(value))
    }
}

// ---------------------------
// Error Mapper
// ---------------------------

fn map_json_error(_: JsonRejection) -> AppError {
    AppError::BadRequest("요청 본문이 올바르지 않습니다.".into())
}

fn map_path_error(_: PathRejection) -> AppError {
    AppError::BadRequest("URL 경로가 올바르지 않습니다.".into())
}

fn map_query_error(_: QueryRejection) -> AppError {
    AppError::BadRequest("Query Parameter가 올바르지 않습니다.".into())
}
