pub mod common; // 외부 통합 테스트 파일에 전역 fallback/error 등을 노출
pub mod domains; // 외부 통합 테스트 파일(tests/)에 내 도메인들을 안전하게 노출
pub mod state; // AppState(DB, JWT, Redis 등 공유 자원) 외부 노출, 테스트에서 AppState 구성 시 필요
