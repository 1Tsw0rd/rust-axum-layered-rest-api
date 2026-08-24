use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::common::response::{ApiResponse, ErrorBody};

#[derive(Debug)]
pub enum AppError {
    Validation(String),
    BadRequest(String),
    NotFound(String),
    Unauthorized,
    InvalidCredentials,
    Forbidden,
    Internal(String),
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::Forbidden => "FORBIDDEN",
            Self::Internal(_) => "INTERNAL_SERVER_ERROR",
        }
    }

    fn client_message(&self) -> String {
        match self {
            Self::Validation(msg) | Self::BadRequest(msg) | Self::NotFound(msg) => msg.clone(),
            Self::Unauthorized => "인증이 필요합니다.".into(),
            Self::InvalidCredentials => "이메일 또는 비밀번호가 올바르지 않습니다.".into(),
            Self::Forbidden => "접근 권한이 없습니다.".into(),
            Self::Internal(_) => "서버 내부 오류가 발생했습니다. 잠시 후 다시 시도해주세요.".into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let Self::Internal(ref msg) = self {
            /*
             %: 사람이 읽기 편한 텍스트만 출력(Display)
             ?: 구조체 타입 및 쌍따옴표까지 포함하여 출력(Debug)

             tracing::error!(error = %msg, "internal server error");
             // 대략 이런 느낌 → error=connection timed out

             tracing::error!(error = ?msg, "internal server error");
             // 대략 이런 느낌 → error=Database("connection timed out")  (구조까지)
            */
            tracing::error!(error = %msg, "internal server error");
        }

        let status = self.status_code();

        let body = ApiResponse::<()> {
            success: false,
            status: status.as_u16(),
            data: None,
            meta: None,
            error: Some(ErrorBody {
                code: self.code(),
                message: self.client_message(),
            }),
        };

        // 여기서 into_response()는 tuple에 대한 into_response
        // into_response는 Axum에서 어떤 값을 실제 HTTP 응답(Response)으로 변환하는 메서드
        (status, Json(body)).into_response()
    }
}

// sqlx Error 전용
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("대상을 찾을 수 없습니다.".into()),
            _ => AppError::Internal(err.to_string()),
        }
    }
}
