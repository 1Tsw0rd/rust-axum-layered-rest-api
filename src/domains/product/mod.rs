pub mod controller;
pub mod dto;
pub mod service;
pub mod repository;
pub mod entity;

use axum::Router;
use std::sync::Arc;
use service::ProductService;

use crate::state::AppState;
use crate::common::auth::permission::{
    Permission::Product,
    ProductPermission::*,
};
use crate::secured_route;


// product 도메인 전용 라우터 생성기
pub fn router(state: AppState) -> Router {
    let product_service = Arc::new(ProductService::new(state.postgres.clone()));

    // secured_route! 매크로가 HTTP Method별 MethodRouter를 만들고 merge()로 결합
    // GET    /products        : 제품 목록 조회
    // GET    /products/{id}   : 제품 단건 조회
    // POST   /products        : 제품 생성
    // PUT    /products/{id}   : 제품 전체 수정
    // PATCH  /products/{id}   : 제품 일부 수정
    // DELETE /products/{id}   : 제품 삭제
    Router::new()
        .route(
            "/",
            secured_route!(
            state.jwt.clone();
            get(controller::get_products) => Product(GetProducts),
            post(controller::create_product) => Product(CreateProduct),
        ),
        )
        .route(
            "/{id}",
            secured_route!(
            state.jwt.clone();
            get(controller::get_product) => Product(GetProduct),
            put(controller::put_product) => Product(PutProduct),
            patch(controller::patch_product) => Product(PatchProduct),
            delete(controller::delete_product) => Product(DeleteProduct),
        ),
        )
        .with_state(product_service)

    // 권한 적용 전 예시: MethodRouter 체이닝으로 동일 경로의 HTTP 메서드를 그룹화
    // Router::new()
    //     .route("/", get(controller::get_products).post(controller::create_product))
    //     .route("/{id}", get(controller::get_product).put(controller::put_product).patch(controller::patch_product).delete(controller::delete_product))
    //     .with_state(product_service)
}
