use axum::{Json, http::StatusCode};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Meta {
    pub count: usize,
    pub total: i64,
    pub page: i64,
    pub size: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,

    // HTTP StatusCode 타입은 serde::Serialize를 지원하지 않음
    // API JSON 응답으로 내려주기 위해 숫자 코드(u16)로 변환하여 저장하기 위해 u16으로 지정함
    // 예: StatusCode::OK -> 200
    pub status: u16,

    // Option 값이 None이면 해당 필드를 JSON 응답에서 제외
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

// data가 없는 성공 응답 전용 impl
// data가 없기 때문에 추론할 타입이 없고, Controller에서 호출할 때
// ApiResponse::<()>::success(StatusCode::OK) 처럼 <()>가 항상 붙어 보기에 좋지 않음
// 그래서 data가 없는 성공 응답 전용 impl을 따로 만들어서 Controller에서 호출할 때
// ApiResponse::success(StatusCode::OK) 처럼 <()>가 생략되도록 함
impl ApiResponse<()> {
    // 성공 - data 없이 단순 성공 응답하고, StatusCode를 커스터마이징할 수 있음
    pub fn success(status: StatusCode) -> (StatusCode, Json<Self>) {
        (
            status,
            Json(Self {
                success: true,
                status: status.as_u16(), // StatusCode 타입을 u16으로 수정
                data: None,
                meta: None,
                error: None,
            }),
        )
    }
}

impl<T: Serialize> ApiResponse<T> {
    // 성공 - 단건 data를 포함한 응답
    pub fn success_with_data(status: StatusCode, data: T) -> (StatusCode, Json<Self>) {
        (
            status,
            Json(Self {
                success: true,
                status: status.as_u16(),
                data: Some(data),
                meta: None,
                error: None,
            }),
        )
    }

    // 성공 - 목록 data + 페이징 정보를 포함한 meta 필드 응답
    // 응답할 때 page, size 제거해도 되지만 그냥 포함하는 걸로 함
    pub fn success_with_list(
        status: StatusCode,
        data: T,
        page: i64,
        size: i64,
        count: usize,
        total: i64,
    ) -> (StatusCode, Json<Self>) {
        let total_pages = if total == 0 {
            0
        } else {
            (total + size - 1) / size
        };

        (
            status,
            Json(Self {
                success: true,
                status: status.as_u16(),
                data: Some(data),
                meta: Some(Meta {
                    count,
                    total,
                    page,
                    size,
                    total_pages,
                }),
                error: None,
            }),
        )
    }
}
