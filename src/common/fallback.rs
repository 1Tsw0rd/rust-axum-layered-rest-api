use crate::common::error::AppError;

pub async fn not_found() -> AppError {
    AppError::NotFound("존재하지 않는 API입니다.".into())
}
