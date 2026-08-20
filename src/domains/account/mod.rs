pub mod controller;
pub mod dto;
pub mod entity;
pub mod repository;
pub mod service;

use std::sync::Arc;

use axum::{
    routing::post,
    Router,
};

use crate::state::AppState;
use crate::common::auth::permission::{
    Permission::Account,
    AccountPermission::*,
};
use crate::secured_route;

use service::AccountService;

pub fn router(state: AppState) -> Router {
    let account_service = Arc::new(
        AccountService::new(
            state.postgres.clone(),
            state.jwt.clone(),
            state.redis.clone()
        )
    );

    // POST /accounts          : 계정 생성
    // POST /accounts/login    : 로그인
    // GET  /accounts/me       : jwt 토큰 내 id값으로 계정 조회
    // POST /accounts/refresh  : Access Token 재발급
    // POST /accounts/logout   : 로그아웃
    Router::new()
        .route("/", post(controller::create_account))
        .route("/login", post(controller::login))
        .route("/me", 
            secured_route!(state.jwt.clone(); get(controller::get_me) => Account(GetMe),),
        )
        .route("/refresh", post(controller::refresh))
        .route("/logout", post(controller::logout))
        .with_state(account_service)
}
