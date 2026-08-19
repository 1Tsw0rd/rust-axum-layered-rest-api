// DB, JWT, Redis, Kafka 등 애플리케이션 전역 인프라 자원을 통합 관리
use crate::common::auth::jwt::JwtConfig;
use crate::common::redis::RedisClient;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub postgres: PgPool,
    pub jwt: JwtConfig,
    pub redis: RedisClient,
}
