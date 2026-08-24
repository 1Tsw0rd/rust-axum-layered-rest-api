use sqlx::PgPool;

use crate::common::error::AppError;
use crate::domains::product::dto::{
    CreateProductDto, ProductListQuery, ReplaceProductDto, UpdateProductDto,
};
use crate::domains::product::entity::ProductEntity;

// ===== 레포지토리 전용 입력 파라미터 구조체 =====
#[derive(Debug)]
pub struct CreateProductParams {
    pub name: String,
    pub description: String,
    pub price: i64,
}

impl From<&CreateProductDto> for CreateProductParams {
    // name, description은 공백이 필요없으므로 trim() 처리
    // trim()은 &str 반환하므로 to_owned()
    fn from(dto: &CreateProductDto) -> Self {
        Self {
            name: dto.name.trim().to_owned(),
            description: dto.description.trim().to_owned(),
            price: dto.price,
        }
    }
}

#[derive(Debug)]
pub struct ReplaceProductParams {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub price: i64,
}

// From trait의 from()은 하나의 인자만 받기 때문에
// 여러 값을 전달하기 위해 tuple(id, dto) 형태로 묶어서 전달
impl From<(i64, &ReplaceProductDto)> for ReplaceProductParams {
    fn from((id, dto): (i64, &ReplaceProductDto)) -> Self {
        Self {
            id,
            name: dto.name.trim().to_owned(),
            description: dto.description.trim().to_owned(),
            price: dto.price,
        }
    }
}

#[derive(Debug)]
pub struct UpdateProductParams {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<i64>,
}

impl From<(i64, &UpdateProductDto)> for UpdateProductParams {
    fn from((id, dto): (i64, &UpdateProductDto)) -> Self {
        Self {
            id,
            name: dto.name.as_ref().map(|v| v.trim().to_owned()),
            description: dto.description.as_ref().map(|v| v.trim().to_owned()),
            // i64는 Copy 타입이므로 Option<i64> 값 전체가 복사되어 소유권 이동 없이 전달 가능
            price: dto.price,
        }
    }
}

// ===== 레포지토리 로직 =====
#[derive(Clone)]
pub struct ProductRepository {
    db_pool: PgPool,
}

impl ProductRepository {
    // 레포지토리 생성자
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    // product count()와 find_all() 둘 다 동일한 WHERE 조건을 써야 하므로 여기서 통일
    fn apply_product_filters(
        builder: &mut sqlx::QueryBuilder<sqlx::Postgres>,
        query: &ProductListQuery,
    ) {
        if let Some(keyword) = &query.keyword {
            builder.push(" WHERE name LIKE ");
            builder.push_bind(format!("%{}%", keyword));
        }
    }

    // 단건 조회
    pub async fn find_by_id(&self, id: i64) -> Result<ProductEntity, AppError> {
        // SELECT * 를 안 쓴 이유는 컬럼 추가 이슈가 있는 경우 불필요한 Data를 받을 수 있기에 현재처럼 작성
        // query_as!()는 컴파일시 DB와 rust 타입 검사해주지만, 동적쿼리 구현하지는 못함
        // 타입 검사하는 건 얘가 어떤 쿼리 쓸지 정적쿼리이기에 아니까 해주는거지 동적쿼리는 쿼리가 어떻게 만들어질지 모름
        // 동적쿼리 구현하고 싶으면 1. query_as 또는 2. query_as!로 분기해서 써야함
        // 동적쿼리 예시는 find_all() 참고
        let entity = sqlx::query_as!(
            ProductEntity,
            r#"
            SELECT 
                id,
                name,
                description,
                price,
                created_at,
                updated_at
            FROM products 
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&self.db_pool) // 1건만 조회
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                AppError::NotFound(format!("ID {}번 제품을 찾을 수 없습니다.", id))
            } // 여기서만 따로 사용
            _ => e.into(), // From<sqlx::Error> 사용
        })?;

        Ok(entity)
    }

    // 조회 조건에 맞는 products row 개수 전체 조회
    pub async fn count(&self, query: &ProductListQuery) -> Result<i64, AppError> {
        let mut builder = sqlx::QueryBuilder::new(
            r#"
            SELECT count(*)
            FROM products
            "#,
        );

        Self::apply_product_filters(&mut builder, query);

        // build_query_scalar: 컬럼 하나 가져올 때 사용
        let count = builder
            .build_query_scalar::<i64>()
            .fetch_one(&self.db_pool)
            .await?;

        Ok(count)
    }

    // 목록 조회
    pub async fn find_all(&self, query: &ProductListQuery) -> Result<Vec<ProductEntity>, AppError> {
        /*
        =====================================================
        방법 1. query_as!() 분기 방식
        =====================================================

        query_as!()는 컴파일 타임 SQL 검사 장점이 있음

        장점:
        - SQL 문법 오류를 컴파일 단계에서 확인 가능
        - DB 컬럼과 Rust 타입 매핑 검증 가능
        - 런타임 SQL 오류 감소

        단점:
        - SQL 문자열이 고정되어 있어야 함
        - 동적 조건이 많아질수록 if 분기 코드가 증가함

        검색 조건이 적고 SQL 구조가 단순한 경우에는 좋은 선택
        */

        /*
        if let Some(keyword) = &query.keyword {

            let products = sqlx::query_as!(
                ProductEntity,
                r#"
                SELECT
                    id,
                    name,
                    description,
                    price,
                    created_at,
                    updated_at
                FROM products
                WHERE name LIKE $1
                ORDER BY id DESC
                LIMIT $2
                OFFSET $3
                "#,
                format!("%{}%", keyword),
                query.size,
                ((query.page - 1) * query.size)
            )
            .fetch_all(&self.db_pool)
            .await?;

            Ok(products)

        } else {

            let products = sqlx::query_as!(
                ProductEntity,
                r#"
                SELECT
                    id,
                    name,
                    description,
                    price,
                    created_at,
                    updated_at
                FROM products
                ORDER BY id DESC
                LIMIT $1
                OFFSET $2
                "#,
                query.size,
                ((query.page - 1) * query.size)
            )
            .fetch_all(&self.db_pool)
            .await?;

            Ok(products)
        }
        */

        /*
        =====================================================
        방법 2. QueryBuilder 방식 (현재 사용)
        =====================================================

        검색 조건, 정렬, 페이징 등 런타임에 SQL 조건이 변경되는
        목록 조회에서 사용

        QueryBuilder를 사용하면 필요한 조건만 SQL에 추가할 수 있음

        장점:
        - 동적 조건 조합 가능
        - push_bind()를 통한 parameter binding으로 SQL injection 방지
        - build_query_as() 사용 시 Entity 매핑 유지 가능
        */

        let mut builder = sqlx::QueryBuilder::new(
            r#"
            SELECT
                id,
                name,
                description,
                price,
                created_at,
                updated_at
            FROM products
            "#,
        );

        // Product 목록 조회 공통 필터 적용
        Self::apply_product_filters(&mut builder, query);

        // TODO 정렬 개선 가능
        builder.push(" ORDER BY id DESC");

        // 페이징
        builder.push(" LIMIT ");
        builder.push_bind(query.size);

        builder.push(" OFFSET ");
        builder.push_bind((query.page - 1) * query.size);

        let products = builder
            .build_query_as::<ProductEntity>()
            .fetch_all(&self.db_pool)
            .await?;

        Ok(products)

        /*
        =====================================================
        사용하지 않는 방식
        =====================================================

        하나의 고정 SQL에서 모든 조건을 처리하는 방식

        예:
        WHERE
            ($1 IS NULL OR name LIKE $1)
        AND
            ($2 IS NULL OR price >= $2)
        AND
            ($3 IS NULL OR created_at >= $3)

        장점:
        - query_as!() 사용 가능
        - SQL 코드가 하나로 유지됨

        단점:
        - 조건이 증가할수록 SQL 복잡도가 증가
        - 불필요한 조건까지 옵티마이저가 고려해야 함
        - 검색 조건, 정렬, 페이징 확장 시 유지보수가 어려움

        목록 조회처럼 조건 조합이 많은 경우에는
        필요한 조건만 추가하는 QueryBuilder 방식이 적합
        */

        /*
        // Option<String> → Option<&str> → Option<String>
        // 원본 소유권은 그대로 두고, &str로 빌려 읽은 뒤 LIKE용 "%keyword%" String을 새로 생성
        // String(소유권이 있는 문자열)은 데이터를 실제로 가지고 있어 메모리 할당/해제 책임이 있음
        // &str(빌려서 읽는 문자열)은 잠시 참조만 하기에 소유권 없음
        // Option<String> → as_ref() 사용하여 Option<String> → Option<String> 이렇게 구현 안함
        let search_keyword = query.keyword.as_deref()
            .map(|k| format!("%{}%", k));

        let entities = sqlx::query_as!(
            ProductEntity,
            r#"
            SELECT
                id,
                name,
                description,
                price,
                created_at,
                updated_at
            FROM products
            WHERE ($1::TEXT IS NULL OR name LIKE $1)
            ORDER BY id DESC
            LIMIT $2
            OFFSET $3
            "#,
            search_keyword,
            query.size,
            (query.page - 1) * query.size
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(entities)
        */
    }

    // 제품 생성
    pub async fn save(&self, params: CreateProductParams) -> Result<(), AppError> {
        // query!()는 쿼리 결과를 Entity로 매핑하지 않을 때 사용(컴파일 타임 SQL 검사 + 파라미터 타입 검사)
        sqlx::query!(
            r#"
            INSERT INTO products (
                name,
                description,
                price
            )
            VALUES ($1, $2, $3)
            "#,
            params.name,
            params.description,
            params.price
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    // 제품 수정(put)
    pub async fn replace(&self, params: ReplaceProductParams) -> Result<(), AppError> {
        let result = sqlx::query!(
            r#"
            UPDATE products
            SET
                name = $1,
                description = $2,
                price = $3
            WHERE id = $4
            "#,
            params.name,
            params.description,
            params.price,
            params.id
        )
        .execute(&self.db_pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("대상을 찾을 수 없습니다.".into()));
        }

        Ok(())
    }

    // 제품 수정(patch)
    pub async fn update(&self, params: UpdateProductParams) -> Result<(), AppError> {
        let mut builder = sqlx::QueryBuilder::new(r#"UPDATE products SET "#);
        let mut separated = builder.separated(", ");
        let mut has_set = false;

        if let Some(name) = params.name {
            // separated는 push마다 콤마(,)를 추가하므로
            // value를 push_bind()로 추가하면 "name = , $1" 형태가 됨
            // "name = $1" 형태로 만들기 위해 push_bind_unseparated() 사용
            separated.push("name = ");
            separated.push_bind_unseparated(name);
            has_set = true
        }

        if let Some(description) = params.description {
            separated.push("description = ");
            separated.push_bind_unseparated(description);
            has_set = true
        }

        if let Some(price) = params.price {
            separated.push("price = ");
            separated.push_bind_unseparated(price);
            has_set = true
        }

        // 서비스에서 검사하고 있긴 하지만, 방어코드로 남김
        if !has_set {
            return Err(AppError::BadRequest("변경할 필드가 없습니다.".into()));
        }

        builder.push(" WHERE id = ");
        builder.push_bind(params.id);

        // 쿼리 실행
        let result = builder.build().execute(&self.db_pool).await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("대상을 찾을 수 없습니다.".into()));
        }

        Ok(())
    }

    // 제품 삭제
    pub async fn delete(&self, id: i64) -> Result<(), AppError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM products WHERE id = $1
            "#,
            id
        )
        .execute(&self.db_pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("삭제 대상을 찾을 수 없습니다.".into()));
        }

        Ok(())
    }
}
