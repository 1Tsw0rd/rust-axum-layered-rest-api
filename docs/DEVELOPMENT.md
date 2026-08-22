# Development Guide

이 문서는 프로젝트를 처음 실행하고, 의존성을 관리하고, 테스트하는 방법을 정리합니다.

## 1. 사전 요구사항

- [Rust 설치](https://www.rust-lang.org/tools/install)

```bash
# Rust 컴파일러 버전 확인
rustc --version

# Cargo 버전 확인
cargo --version
```

- [Docker 설치 방법](https://docs.docker.com/get-started/get-docker/)

```bash
# Docker 버전 확인
docker --version

# Docker Compose 버전 확인
docker compose version
```

## 2. 프로젝트 초기화

저장소를 이미 클론한 경우 프로젝트 루트에서 `cargo init`을 다시 실행하지 않습니다.

```bash
git clone https://github.com/1Tsw0rd/rust-axum-layered-rest-api.git
cd rust-axum-layered-rest-api
cargo build
```

`Cargo.toml`과 `Cargo.lock`이 저장소에 포함되어 있으므로 별도의 `cargo add` 없이 `cargo build`가 의존성을 내려받습니다.

## 3. Cargo 명령어

```bash
# 개발 프로필로 컴파일
cargo build

# 실행 파일 실행
cargo run

# 의존성·코드의 컴파일 가능 여부만 빠르게 확인
cargo check

# 전체 테스트 실행
cargo test

# 출력 로그까지 확인하며 테스트 실행
cargo test -- --nocapture

# 참고: 이 프로젝트의 통합 테스트는 상태를 공유하므로
# 실제 테스트 실행 시 `cargo test -- --test-threads=1` 사용을 권장

# fmt는 코드의 동작은 바꾸지 않고, 읽기 좋은 모양으로 정리하는 작업
# 포맷 적용
cargo fmt
# 포맷 확인
cargo fmt -- --check

# Clippy 정적 분석
# Clippy는 Rust 코드에서 개선할 수 있는 부분이나 잠재적인 실수를 알려주는 정적 분석 도구
# 기본 정적 분석
cargo clippy
# 전체 대상과 모든 feature를 엄격하게 검사할 때
cargo clippy --all-targets --all-features -- -D warnings
```

## 4. 사용된 의존성 추가 명령어

아래 명령어는 프로젝트를 구성할 때 실제로 사용한 기록입니다.
저장소를 클론한 뒤에는 반복해서 실행하지 않고 `cargo build`만 실행합니다.

프로젝트 루트에서 `cargo add`를 실행하면
기능 플래그는 `Cargo.toml`에 기록되므로 명령어와 함께 관리합니다.

### 애플리케이션 의존성

```bash
# Cargo.toml의 버전 조건 안에서 의존성을 최신 버전으로 갱신
# 특정 패키지만 갱신(cargo update -p axum)
cargo update

# 웹 프레임워크: Tokio 팀이 개발한 비동기 HTTP 웹 프레임워크
cargo add axum --features "macros"

# 비동기 런타임: 네트워크 I/O, 타이머, 태스크, 동시성 등 비동기 실행 환경 제공
cargo add tokio --features "full"

# 데이터 변환: JSON 바이트 데이터를 Rust 구조체로 상호 변환 (직렬화/역직렬화 자동화)
cargo add serde --features "derive"

# 요청 DTO의 필드 규칙을 #[derive(Validate)]와 #[validate(...)]로 유효성 검증하는 라이브러리
cargo add validator --features "derive"

# info!, warn!, error! 로그 매크로
cargo add tracing

# 로그 프린터
# fmt: 터미널에 로그 출력을 담당
# env-filter: 환경변수를 기반으로 모듈별 로그 레벨 필터링 지원
cargo add tracing-subscriber --features "fmt,env-filter"

# HTTP 요청·응답 추적 미들웨어
cargo add tower-http --features "trace"

# 시간 타입 매퍼: DateTime 구조체가 JSON으로 정상 출력되도록 Serde 연동 기능 켬
cargo add chrono --features "serde"

# DB 연동: 비동기 SQL 툴킷
# runtime-tokio: Tokio연동
# tls-rustls: 암호화 통신
# postgres: Postgres 드라이버
# uuid: UUID 타입 매핑
# chrono: 시간 타입 매핑
# macros: 컴파일 시점 쿼리 검증 매크로
cargo add sqlx --features "runtime-tokio,tls-rustls,postgres,uuid,chrono,macros"

# UUID v4 생성과 Serde 직렬화/역직렬화
cargo add uuid --features "v4,serde"

# .env 환경변수 로더
cargo add dotenvy

# Argon2id 비밀번호 해싱·검증
cargo add argon2

# JWT 생성·서명·검증
# rust_crypto: RustCrypto 계열의 암호화 구현 사용
cargo add jsonwebtoken --features "rust_crypto"

# OS 기반 저수준 암호학적 난수(랜덤 바이트) 생성
cargo add getrandom

# SHA-256 해시
cargo add sha2

# Redis 또는 Dragonfly와 비동기로 통신하고 연결을 관리하는 클라이언트
cargo add redis --features "tokio-comp,connection-manager"

# Cookie를 읽고 생성하기 위한 확장 기능
cargo add axum-extra --features "cookie"
```

### 테스트 전용 의존성

```bash
# 서버를 실제로 구동하지 않고 메모리상에서 HTTP 요청을 모의 시뮬레이션(.oneshot)하는 유틸 켬
cargo add tower --dev --features "util"

# HTTP Body 수집·검증 유틸: Axum이 응답한 스트림 바이트 본문(Body) 데이터를 사람이 읽을 수 있는 JSON/텍스트로 수집하고 변환하는 도구
cargo add http-body-util --dev

# 테스트 JSON 요청·응답 생성 및 검증
cargo add serde_json --dev
```

## 5. 환경변수

`.env`는 로컬 실행용이고 `.env.test`는 통합 테스트용입니다.

| 변수 | 설명 |
|---|---|
| `DATABASE_URL` | PostgreSQL URL로, SQLx가 기본적으로 인식하는 환경변수 이름 |
| `TEST_DATABASE_URL` | 통합 테스트용 PostgreSQL 연결 문자열 |
| `DB_USER` / `DB_PASSWORD` / `DB_NAME` | Docker PostgreSQL 초기 설정 |
| `DB_PORT` | 호스트에서 PostgreSQL에 접근할 포트 |
| `JWT_SECRET` | JWT 서명 키 |
| `JWT_ACCESS_TOKEN_EXPIRES_IN_SECONDS` | Access Token 만료 시간 |
| `CACHE_BACKEND` | `redis` 또는 `dragonfly` |
| `REDIS_HOST` / `REDIS_PORT` / `REDIS_PASSWORD` | Redis 연결 설정 |
| `DRAGONFLY_HOST` / `DRAGONFLY_PORT` / `DRAGONFLY_PASSWORD` | Dragonfly 연결 설정 |
| `REDIS_DB` / `DRAGONFLY_DB` | Redis/Dragonfly DB 번호(0: 개발, 1: 테스트) — 개발/테스트 환경 격리용 |
| `RUST_LOG` | 모듈별 로그 레벨 필터 |

공개 저장소에 실제 운영 비밀키를 올리지 않습니다. 학습용 값이라도 운영 환경에서는 반드시 별도 Secret 관리 방식을 사용합니다.

## 6. Docker Compose

```bash
# PostgreSQL, Redis, Dragonfly를 백그라운드로 실행
docker compose up -d

# 실행 상태 확인
docker compose ps

# 전체 로그 확인
docker compose logs -f

# 서비스별 로그 확인
docker compose logs -f postgres
docker compose logs -f redis
docker compose logs -f dragonfly

# 컨테이너 종료
docker compose down
```

### Redis/Dragonfly docker container 접속 명령어

```bash
# Redis
docker compose exec redis redis-cli -a "$REDIS_PASSWORD" -n "$REDIS_DB"

# Dragonfly
docker compose exec dragonfly redis-cli -a "$DRAGONFLY_PASSWORD" -n "$DRAGONFLY_DB"
```

자주 사용하는 명령:

```text
SELECT <DB번호>       DB 선택(0: 개발DB, 1: 테스트DB)
PING                 연결 확인
SCAN 0               키 목록을 안전하게 순회
GET <key>            문자열 값 조회
TTL <key>            남은 만료 시간 확인
DEL <key>            키 삭제
FLUSHDB              현재 DB 전체 삭제(주의)
```

## 7. 테스트

단위 테스트는 DTO·Role·Permission·JWT 같은 개별 로직을 검증하고, 통합 테스트는 Axum Router에 실제 HTTP 요청을 보내 도메인 흐름을 검증합니다.

통합 테스트는 `.env.test`의 `TEST_DATABASE_URL`에 연결하며, 테스트 시작 시 `accounts`, `products` 테이블을 초기화합니다.

Redis/Dragonfly는 `.env.test`에서 테스트용 DB 번호(`1`)를 사용하여 개발용 DB(`0`)와 분리하며, 테스트 시작 시 데이터를 초기화합니다.

테스트용 PostgreSQL 및 Redis/Dragonfly 환경은 개발용 환경과 분리해서 사용됩니다.

```bash
# 전체 단위·통합 테스트
# DB·Redis 공유 상태 충돌을 피하기 위해 직렬 실행
cargo test -- --test-threads=1

# src 내부 단위 테스트 전체
cargo test --lib

# src 내부 특정 모듈 단위 테스트
cargo test --lib common::auth::jwt
cargo test --lib common::auth::authorization
cargo test --lib common::auth::extractor
cargo test --lib common::auth::permission
cargo test --lib common::auth::role
cargo test --lib common::auth::refresh_token
cargo test --lib common::security::password
cargo test --lib domains::account::dto

# 통합 API 테스트 전체
cargo test --test api -- --test-threads=1

# Account API 파일별
cargo test --test api account::create_account -- --test-threads=1
cargo test --test api account::login -- --test-threads=1
cargo test --test api account::get_me -- --test-threads=1
cargo test --test api account::refresh -- --test-threads=1
cargo test --test api account::logout -- --test-threads=1

# Product API 파일별
cargo test --test api product::get_product -- --test-threads=1
cargo test --test api product::get_products -- --test-threads=1
cargo test --test api product::create_product -- --test-threads=1
cargo test --test api product::put_product -- --test-threads=1
cargo test --test api product::patch_product -- --test-threads=1
cargo test --test api product::delete_product -- --test-threads=1

# 특정 테스트 함수 하나만 실행하는 예시
cargo test --lib common::auth::jwt::tests::create_and_verify_success

# 테스트 로그 출력
cargo test --test api account::login -- --test-threads=1 --nocapture
```

`cargo test`는 기본적으로 테스트를 병렬 실행합니다. 이 프로젝트는 테스트마다 PostgreSQL과 Redis 데이터를 초기화하므로, 전체 테스트와 API 통합 테스트는 `--test-threads=1`을 함께 사용합니다.

## 8. 브랜치와 커밋 규칙

기본 흐름은 `develop`을 통합 브랜치로 사용하고, 작업별 브랜치에서 변경한 뒤 Pull Request로 `develop`에 병합하는 방식입니다.

```text
develop
└── feature / docs / bugfix 등 브랜치
```

커밋 메시지는 다음 형식을 사용합니다.

```text
type: 작업 내용
```

| 타입 | 용도 | 예시 |
|---|---|---|
| `feat` | 기능 추가 | `feat: add account registration API` |
| `fix` | 버그 수정 | `fix: handle invalid JWT token` |
| `docs` | 문서 작성·수정 | `docs: add API documentation` |
| `chore` | 프로젝트 관리·초기 설정 | `chore: initialize Rust project` |
| `build` | 의존성·빌드 설정 | `build: add project dependencies` |
| `test` | 테스트 추가·수정 | `test: add account API tests` |
| `refactor` | 기능 변경 없는 구조 개선 | `refactor: separate repository params` |
| `style` | 포맷·공백·세미콜론 등 수정 | `style: format Rust source files` |
| `perf` | 성능 개선 | `perf: optimize product query` |

작업이 끝나면 먼저 `cargo fmt`, `cargo check`, 관련 테스트를 실행한 뒤 커밋합니다.
