use sqlx::PgPool;

use crate::common::error::AppError;
use crate::domains::product::dto::{CreateProductDto, ReplaceProductDto, UpdateProductDto, ProductListQuery, ProductResponseDto};
use crate::domains::product::repository::{ProductRepository, CreateProductParams, UpdateProductParams};
use crate::common::dto::PaginatedResponseDto;

// repository 주입
pub struct ProductService {
    product_repository: ProductRepository,
}

impl ProductService {
    // [의존성 주입을 위한 서비스 인스턴스 생성자]
    // 1. 왜 필요한가?: Rust에는 자바나 NestJS처럼 클래스를 자동으로 New 해서 메모리에 올려주는 기본 기능이 없음
    // 2. 작동 원리: 메인 허브인 `main.rs`나 `mod.rs`에서 이 `new()`를 딱 한 번 수동으로 호출하여
    //    `ProductService`라는 로직 주머니(인스턴스)를 실물 메모리에 실체화(Instantiation)시킴
    // 3. 공유의 목적: 이렇게 실체화된 서비스 주머니를 `Arc::new(ProductService::new())`로 감싸서
    //    Axum 라우터의 `.with_state()`에 탑재해주어야만, 수많은 컨트롤러 함수들이 이 로직을 안전하게 공유(DI)해서 호출할 수 있게 됨
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            product_repository: ProductRepository::new(db_pool),
        }
    }

    // GET: 제품 단건 조회
    pub async fn get_product(&self, id: i64) -> Result<ProductResponseDto, AppError> {
        let entity = self.product_repository.find_by_id(id).await?;
        Ok(entity.into()) // Entity -> ProductResponseDto 매퍼 정상 작동
    }

    // GET: 제품 목록 조회
    pub async fn get_products(
        &self,
        query: &ProductListQuery,
    ) -> Result<PaginatedResponseDto<ProductResponseDto>, AppError> {
        let total = self.product_repository.count(query).await?;

        // 0건이면 목록 조회 자체를 안 함
        if total == 0 {
            return Ok(PaginatedResponseDto {
                data: Vec::new(),
                count: 0,
                total: 0,
           });
        }

        // 데이터가 있을 때만 목록 조회
        let entities = self.product_repository.find_all(query).await?;
        let data = entities
            .into_iter()
            .map(|entity| entity.into())
            .collect::<Vec<ProductResponseDto>>();
        let count = data.len();

        Ok(PaginatedResponseDto {
            data,
            count,
            total,
        })
    }


    // POST: 제품 생성
    pub async fn create_product(&self, payload: &CreateProductDto) -> Result<(), AppError> {
        let params: CreateProductParams = payload.into();
        self.product_repository.save(params).await
    }

    // PUT: 제품 전체 정보 수정
    pub async fn replace_product(&self, id: i64, payload: &ReplaceProductDto) -> Result<(), AppError> {
        // 경계 표현을 위해 아래처럼 명시적으로 params 생성하는 방식도 가능
        // let params = ReplaceProductParams::from((id, payload));
        // self.product_repository.replace(params).await

        // Into trait 사용하여 타입 추론을 통해 변환 가능
        // 중간 변수 없이 (id, ReplaceProductDto) 튜플을 ReplaceProductParams로 변환 후 Repository 전달
        self.product_repository.replace((id, payload).into()).await
    }

    // PATCH: 제품 일부 정보 수정
    pub async fn update_product_fields(&self, id: i64, payload: &UpdateProductDto) -> Result<(), AppError> {
        if payload.name.is_none() && payload.description.is_none() && payload.price.is_none() {
            return Err(AppError::BadRequest("변경할 필드가 없습니다".to_string()));
        }
        let params = UpdateProductParams::from((id, payload));
        self.product_repository.update(params).await
    }

    // DELETE: 제품 삭제
    pub async fn delete_product(&self, id: i64) -> Result<(), AppError> {
        self.product_repository.delete(id).await
    }
}
