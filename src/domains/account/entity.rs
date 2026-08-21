// use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct AccountEntity {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub name: String,

    // DB account 테이블에는 아래 Column이 존재하지만 현재 조회/응답에서는 사용하지 않음
    // pub created_at: DateTime<Utc>,
    // pub updated_at: DateTime<Utc>,
}
