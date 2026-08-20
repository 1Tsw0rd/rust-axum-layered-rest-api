use sqlx::{
    PgPool,
    Postgres,
    Transaction,
};

use crate::common::{
    error::AppError,
    auth::role::AccountRole
};
use crate::domains::account::{
    entity::AccountEntity,
};

// ===== 레포지토리 전용 입력 파라미터 구조체 =====
#[derive(Debug)]
pub struct CreateAccountParams {
    pub email: String,
    pub password_hash: String,
    pub name: String,
}

impl CreateAccountParams {
    pub fn new(email: String, password_hash: String, name: String) -> Self {
        Self {
            email,
            password_hash,
            name,
        }
    }
}

// ===== 레포지토리 로직 =====
#[derive(Clone)]
pub struct AccountRepository {
    db_pool: PgPool,
}

impl AccountRepository {
    // 레포지토리 생성자
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    // DB 트랜잭션 시작 함수
    pub async fn begin(
        &self,
    ) -> Result<Transaction<'_, Postgres>, AppError> {
        self.db_pool.begin().await.map_err(Into::into)
    }

    // 이메일 중복 검사
    pub async fn exists_by_email(&self, email: &str) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM accounts WHERE email = $1
            )
            "#,
            email
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(exists.unwrap_or(false))
    }

    // 계정 생성
    // - 생성된 account_id는 Role 연결 등 Service 내부 후속 처리에만 사용하고, Controller를 통해 Client에게는 반환하지 않음
    // - Service의 exists_by_email()은 사전 중복 검사이며, 동시 요청에서는 여러 요청이 동시에 "중복 없음"으로 판단할 수 있음
    // 따라서 DB의 UNIQUE 제약조건(accounts_email_key)을 최종 방어선으로 사용함
    pub async fn save(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        params: CreateAccountParams,
    ) -> Result<i64, AppError> {
        let account_id = sqlx::query_scalar!(
            r#"
            INSERT INTO accounts (
                email,
                password_hash,
                name
            )
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
            params.email,
            params.password_hash,
            params.name
        )
        .fetch_one(&mut **tx) // &mut **tx: tx를 두 번 역참조하여 Transaction 내부의 PgConnection을 가변 참조로 전달
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err)
                if db_err.constraint() == Some("accounts_email_key") => {
                    AppError::BadRequest(
                        "이미 사용 중인 이메일입니다.".into(),
                    )
                }
            _ => e.into(),
        })?;
        // db_err.code()       // PostgreSQL 오류 코드
        // db_err.constraint() // 위반한 제약조건 이름
        // db_err.table()      // 관련 테이블
        // db_err.column()     // 관련 컬럼
        // db_err.message()    // DB 오류 메시지

         Ok(account_id)
    }

    pub async fn assign_role(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account_id: i64,
        role: &AccountRole,
    ) -> Result<(), AppError> {
        let result = sqlx::query!(
            r#"
            INSERT INTO account_roles (
                account_id,
                role_id
            )
            SELECT
                $1,
                id
            FROM roles
            WHERE name = $2
            "#,
            account_id,
            role.as_str()
        )
        .execute(&mut **tx)
        .await?;

        if result.rows_affected() == 0 {
            tracing::error!(
                "Role 연결 실패: 존재하지 않는 role={}",
                role.as_str()
            );

            return Err(AppError::Internal(
                "계정 역할 처리 중 오류가 발생했습니다.".into(),
            ));
        }

        Ok(())
    }

    // 계정에 연결된 모든 Role 조회
    pub async fn find_roles_by_account_id(
        &self,
        account_id: i64,
    ) -> Result<Vec<AccountRole>, AppError> {
        let role_names = sqlx::query_scalar!(
            r#"
            SELECT
                roles.name
            FROM account_roles
            INNER JOIN roles
                ON roles.id = account_roles.role_id
            WHERE account_roles.account_id = $1
            ORDER BY roles.id
            "#,
            account_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        role_names
            .into_iter()
            .map(|name| {
                AccountRole::try_from(name.as_str()).map_err(|_| {
                    tracing::error!(
                        "DB에 존재하지 않는 AccountRole 값: {}",
                        name
                    );

                    AppError::Internal(
                        "계정 역할 처리 중 오류가 발생했습니다.".into(),
                    )
                })
            })
            .collect()
    }

    // 이메일로 계정 조회
    pub async fn find_by_email(&self, email: &str) -> Result<Option<AccountEntity>, AppError> {
        // fetch_optional() 사용한 이유는 Some(AccountEntity) 또는 None 반환 시켜
        // 이메일이 없는 경우 401 Unauthorized 반환하기 위함
        let account = sqlx::query_as!(
            AccountEntity,
            r#"
            SELECT
                id,
                email,
                password_hash,
                name
            FROM accounts
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(account)
    }

    // ID로 계정 조회
    pub async fn find_by_id(&self, id: i64) -> Result<AccountEntity, AppError> {
        let account = sqlx::query_as!(
            AccountEntity,
            r#"
            SELECT
                id,
                email,
                password_hash,
                name
            FROM accounts
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(account)
    }
}
