use redis::{
    aio::ConnectionManager,
    Client,
};

use crate::common::error::AppError;

#[derive(Clone)]
pub struct RedisClient {
    connection: ConnectionManager,
}

/*
ConnectionManager가 Redis 연결을 관리하며,
clone()은 새로운 TCP 연결을 만드는 것이 아니라
기존 연결을 공유하여 여러 비동기 요청에서 사용할 핸들을 복제

Redis 서버
   │
   │ TCP 연결 관리
   ▼
ConnectionManager
   │
   ├── 요청 A → "거 연결 좀 같이 씁시다" → GET
   ├── 요청 B → "저도 같이 쓸게요"     → SET
   └── 요청 C → "저도요"               → DEL

Redis Client (연결 정보만 보유, "어디 연결할지 알고 있음")

ConnectionManager (실제 연결 수립 + 재연결 관리, "연결을 실제로 관리함")
→ "연결 관리하고 있습니다."
→ "여러 요청이 이 연결을 사용할 수 있습니다."
→ "연결이 끊어지면 재연결도 관리합니다."

clone() (ConnectionManager의 핸들을 복제하여 Redis 작업에 사용)
→ "저도 그 연결을 사용할게요."
*/
impl RedisClient {
    pub async fn new(client: Client) -> Result<Self, AppError> {
        let connection = client
            .get_connection_manager()
            .await
            .map_err(|err| {
                AppError::Internal(format!(
                    "Redis/Dragonfly 연결에 실패했습니다: {}",
                    err
                ))
            })?;

        Ok(Self { connection })
    }

    pub async fn ping(&self) -> Result<(), AppError> {
        let mut connection = self.connection.clone();

        let _: String = redis::cmd("PING")
            .query_async(&mut connection)
            .await
            .map_err(|err| {
                AppError::Internal(format!(
                    "Redis PING에 실패했습니다: {}",
                    err
                ))
            })?;

        Ok(())
    }

    // Key-Value 데이터를 저장하고 TTL을 초 단위로 설정
    // redis-cli SET key value EX ttl
    // EX는 만료시간을 초 단위로 설정
    pub async fn set(
        &self,
        key: &str,
        value: i64,
        ttl_seconds: u64,
    ) -> Result<(), AppError> {
        let mut connection = self.connection.clone();

        redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(ttl_seconds)
            .query_async::<()>(&mut connection)
            .await
            .map_err(|err| {
                AppError::Internal(format!(
                    "Redis 데이터 저장에 실패했습니다: {}",
                    err
                ))
            })?;
        // 잘 성공했다고 Ok 반환
        Ok(())
    }

    // Key에 저장된 값을 조회
    // redis-cli GET key
    pub async fn get(
        &self,
        key: &str,
    ) -> Result<Option<String>, AppError> {
        let mut connection = self.connection.clone();

        // Some(value)는 key 존재, None는 Key 없음 또는 TTL 만료
        // key가 있다면 value를 반환
        redis::cmd("GET")
            .arg(key)
            .query_async(&mut connection)
            .await
            .map_err(|err| {
                AppError::Internal(format!(
                    "Redis 데이터 조회에 실패했습니다: {}",
                    err
                ))
            })
    }

    // Key에 저장된 데이터를 삭제
    // redis-cli DEL key
    pub async fn delete(
        &self,
        key: &str,
    ) -> Result<(), AppError> {
        let mut connection = self.connection.clone();

        redis::cmd("DEL")
            .arg(key)
            .query_async::<()>(&mut connection)
            .await
            .map_err(|err| {
                AppError::Internal(format!(
                    "Redis 데이터 삭제에 실패했습니다: {}",
                    err
                ))
            })?;
        // 잘 성공했다고 Ok 반환
        Ok(())
    }

    // 여러 Redis 명령을 하나의 원자적 pipeline(MULTI/EXEC)으로 실행
    // redis-cli에서
    // MULTI
    // <여러 Redis 명령 실행>
    // EXEC
    pub async fn atomic_pipeline(
        &self,
        build: impl FnOnce(&mut redis::Pipeline),
    ) -> Result<(), AppError> {
        let mut connection = self.connection.clone();

        let mut pipeline = redis::pipe();
        pipeline.atomic();

        build(&mut pipeline);

        /*
            참고사항
            Cmd
            └── query_async()

            Pipeline
            ├── query_async() → 결과를 받고 싶을 때
            └── exec_async()  → 결과가 필요 없을 때
        */
        pipeline
            .exec_async(&mut connection)
            .await
            .map_err(|err| {
                AppError::Internal(format!(
                    "Redis 원자적 파이프라인 실행에 실패했습니다: {}",
                    err
                ))
            })?;

        Ok(())
    }
}
