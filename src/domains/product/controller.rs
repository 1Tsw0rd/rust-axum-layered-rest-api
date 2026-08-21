use std::sync::Arc;
use axum::{extract::State, response::IntoResponse, http::StatusCode};
use crate::domains::product::{
    dto::{ProductPathId, CreateProductDto, ReplaceProductDto, UpdateProductDto, ProductListQuery},
    service::ProductService,
};
use crate::common::response::ApiResponse;
use crate::common::error::AppError;
use crate::common::extractors::{
    ValidatedJson,
    ValidatedPath,
    ValidatedQuery,
};

// State(product_service): Spring의 @Autowired나 NestJS 생성자 주입과 같은 의존성 주입(DI) 장치
// State<Arc<ProductService>>: State가 보관 중인 Arc<ProductService>를 패턴 매칭으로 꺼내 product_service 변수에 바인딩하는 문법
// impl IntoResponse: 반환값을 Axum의 HTTP Response로 변환할 수 있도록 하는 trait
// ApiResponse, Json, StatusCode 등 다양한 응답 타입을 HTTP Response로 변환할 때 사용
// GET: 제품 단건 조회
#[axum::debug_handler] // 개발 중 Handler 관련 에러를 자세히 보기 위한 디버깅용 Attribute, 운영 시 보통 제거
pub async fn get_product(
    State(product_service): State<Arc<ProductService>>,
    ValidatedPath(params): ValidatedPath<ProductPathId>,
) -> Result<impl IntoResponse, AppError> {
    let product = product_service.get_product(params.id).await?;
    Ok(ApiResponse::success_with_data(StatusCode::OK, product))
}

// POST: 제품 생성
#[axum::debug_handler]
pub async fn create_product(
    State(product_service): State<Arc<ProductService>>,
    ValidatedJson(payload): ValidatedJson<CreateProductDto>,
) -> Result<impl IntoResponse, AppError> { 
    product_service.create_product(&payload).await?;
    Ok(ApiResponse::success(StatusCode::CREATED))
}

// PUT: 제품 정보 전체 교체
#[axum::debug_handler]
pub async fn put_product(
    State(product_service): State<Arc<ProductService>>,
    ValidatedPath(params): ValidatedPath<ProductPathId>,
    ValidatedJson(payload): ValidatedJson<ReplaceProductDto>,
) -> Result<impl IntoResponse, AppError> {
    product_service.replace_product(params.id, &payload).await?;
    Ok(ApiResponse::success(StatusCode::OK))
}

// PATCH: 제품 정보 일부 수정
#[axum::debug_handler]
pub async fn patch_product(
    State(product_service): State<Arc<ProductService>>,
    ValidatedPath(params): ValidatedPath<ProductPathId>,
    ValidatedJson(payload): ValidatedJson<UpdateProductDto>,
) -> Result<impl IntoResponse, AppError> {
    product_service.update_product_fields(params.id, &payload).await?;
    Ok(ApiResponse::success(StatusCode::OK))
}

// DELETE: 제품 삭제
#[axum::debug_handler]
pub async fn delete_product(
    State(product_service): State<Arc<ProductService>>,
    ValidatedPath(params): ValidatedPath<ProductPathId>,
) -> Result<impl IntoResponse, AppError> {
    product_service.delete_product(params.id).await?;
    Ok(ApiResponse::success(StatusCode::OK))
}

// GET: 제품 목록 조회
#[axum::debug_handler]
pub async fn get_products(
    State(product_service): State<Arc<ProductService>>,
    ValidatedQuery(query): ValidatedQuery<ProductListQuery>,
) -> Result<impl IntoResponse, AppError> {
    let result = product_service.get_products(&query).await?;
    Ok(ApiResponse::success_with_list(
        StatusCode::OK,
        result.data,
        query.page, // page
        query.size, // size
        result.count,   // count
        result.total, // total
    ))
}
