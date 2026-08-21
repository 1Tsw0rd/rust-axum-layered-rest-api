use chrono::{DateTime, Utc};
use sqlx::FromRow; // SQLx 쿼리 결과를 ProductEntity로 매핑

#[derive(Debug, Clone, FromRow)]
pub struct ProductEntity {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub price: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
