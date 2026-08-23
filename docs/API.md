# API Documentation

Frontend에서 API를 한눈에 확인하고, 필요한 Request·Response 상세 내용으로 이동할 수 있도록 정리한 문서입니다.

## Base URL

```text
http://127.0.0.1:8080
```

## API Overview

| 기능 | API Method & URL | 인증 / 요청 조건 | Request | Response | 권한 |
|---|---|---|---|---|---|
| 회원가입 | `POST /accounts` | 인증 불필요 | [Request](#account-create-request) | [Response](#account-create-response) | - |
| 로그인 | `POST /accounts/login` | 인증 불필요 | [Request](#account-login-request) | [Response](#account-login-response) | - |
| 내 계정 조회 | `GET /accounts/me` | Access Token 필요 | [Request](#account-me-request) | [Response](#account-me-response) | [확인](#role-permission-table) |
| Access Token 재발급 | `POST /accounts/refresh` | 만료된 Access Token + Refresh Cookie 필요 | [Request](#account-refresh-request) | [Response](#account-refresh-response) | - |
| 로그아웃 | `POST /accounts/logout` | Access Token + Refresh Cookie 필요 | [Request](#account-logout-request) | [Response](#account-logout-response) | - |
| 상품 목록 조회 | `GET /products?page=1&size=20&keyword=keyboard` | Access Token 필요 | [Request](#product-list-request) | [Response](#product-list-response) | [확인](#role-permission-table) |
| 상품 단건 조회 | `GET /products/{id}` | Access Token 필요 | [Request](#product-get-request) | [Response](#product-get-response) | [확인](#role-permission-table) |
| 상품 생성 | `POST /products` | Access Token 필요 | [Request](#product-create-request) | [Response](#product-create-response) | [확인](#role-permission-table) |
| 상품 전체 수정 | `PUT /products/{id}` | Access Token 필요 | [Request](#product-put-request) | [Response](#product-put-response) | [확인](#role-permission-table) |
| 상품 일부 수정 | `PATCH /products/{id}` | Access Token 필요 | [Request](#product-patch-request) | [Response](#product-patch-response) | [확인](#role-permission-table) |
| 상품 삭제 | `DELETE /products/{id}` | Access Token 필요 | [Request](#product-delete-request) | [Response](#product-delete-response) | [확인](#role-permission-table) |

<a id="role-permission-table"></a>
### Role별 API 권한

`-`는 Role별 Permission 검사를 적용하지 않는 API를 의미합니다.

| API | Admin | Employee | Customer |
|---|---|---|---|
| 회원가입 | - | - | - |
| 로그인 | - | - | - |
| 내 계정 조회 | ✅ | ✅ | ✅ |
| Access Token 재발급 | - | - | - |
| 로그아웃 | - | - | - |
| 상품 조회(단건) | ✅ | ✅ | ✅ |
| 상품 조회(목록) | ✅ | ✅ | ✅ |
| 상품 생성 | ✅ | ✅ | ❌ |
| 상품 전체 수정(PUT) | ✅ | ✅ | ❌ |
| 상품 일부 수정(PATCH) | ✅ | ✅ | ❌ |
| 상품 삭제 | ✅ | ❌ | ❌ |

## 공통 인증

Access Token은 다음 Header로 전달합니다.

```http
Authorization: Bearer <access-token>
```

Refresh Token은 로그인·재발급 응답의 `HttpOnly`, `Secure`, `SameSite=Strict` Cookie로 전달됩니다. Refresh API는 만료된 Access Token과 유효한 Refresh Cookie를 함께 요구합니다.

## 공통 응답

### 성공 - 단일 Data 포함

```json
{
  "success": true,
  "status": 200,
  "data": {}
}
```

### 성공 - 목록 Data와 meta 포함

```json
{
  "success": true,
  "status": 200,
  "data": [],
  "meta": {
    "count": 2,
    "total": 5,
    "page": 1,
    "size": 2,
    "total_pages": 3
  }
}
```

- `count`: 현재 페이지에 포함된 데이터 개수
- `total`: 전체 검색 결과 개수
- `page`: 현재 페이지 번호
- `size`: 페이지당 데이터 개수
- `total_pages`: 전체 페이지 수

### 성공 - Data 없음

```json
{
  "success": true,
  "status": 201
}
```

### 실패

```json
{
  "success": false,
  "status": 401,
  "error": {
    "code": "UNAUTHORIZED",
    "message": "인증이 필요합니다."
  }
}
```

| HTTP 상태 | code | 의미 |
|---:|---|---|
| 400 | `BAD_REQUEST` | JSON·Path·Query 형식 오류 또는 잘못된 요청 |
| 401 | `UNAUTHORIZED` | 인증 누락·토큰 오류·Refresh Session 오류 |
| 401 | `INVALID_CREDENTIALS` | 이메일 또는 비밀번호 불일치 |
| 403 | `FORBIDDEN` | 접근 권한 부족 |
| 404 | `NOT_FOUND` | 대상 또는 라우트가 없음 |
| 422 | `VALIDATION_ERROR` | DTO 검증 실패 |
| 500 | `INTERNAL_SERVER_ERROR` | 서버 내부 오류 |

### 오류 정보 제공 범위

보안을 위해 서버 내부의 상세 오류 원인은 Response에 노출하지 않습니다.
상세 원인과 디버깅 정보는 `request_id`를 기준으로 백엔드 로그에서 확인할 수 있도록 구성했습니다.

## Account API 상세

<a id="account-create-request"></a>
### 회원가입 Request

```http
POST /accounts
Content-Type: application/json
```

```json
{
  "email": "user@example.com",
  "password": "Password1!",
  "name": "홍길동",
  "role": "customer"
}
```

인증이 필요하지 않습니다.

- `email`: 올바른 이메일 형식
- `password`: 공백 없이 8~100자, 영문 대문자·소문자·숫자·기타 문자(특수기호·한글·이모지 등) 중 3종류 이상
- `name`: 1~50자 (공백만으로는 안 되며, 앞뒤 공백은 저장 시 제거됨)
- `role`: `customer` 또는 `employee`
- `admin` Role은 일반 회원가입 Request에서 사용할 수 없음

<a id="account-create-response"></a>
### 회원가입 Response

```json
{
  "success": true,
  "status": 201
}
```

<a id="account-login-request"></a>
### 로그인 Request

```http
POST /accounts/login
Content-Type: application/json
```

```json
{
  "email": "user@example.com",
  "password": "Password1!"
}
```

- `email`: 올바른 이메일 형식
- `password`: 1자 이상 (빈 값 불가)

<a id="account-login-response"></a>
### 로그인 Response

```json
{
  "success": true,
  "status": 200,
  "data": {
    "access_token": "<access-token>"
  }
}
```

Refresh Token은 JSON이 아니라 `refresh_token` Cookie로 함께 반환됩니다. 이메일과 비밀번호가 일치하지 않으면 `INVALID_CREDENTIALS`를 반환합니다.

<a id="account-me-request"></a>
### 내 계정 조회 Request

```http
GET /accounts/me
Authorization: Bearer <access-token>
```

<a id="account-me-response"></a>
### 내 계정 조회 Response

```json
{
  "success": true,
  "status": 200,
  "data": {
    "id": 1,
    "email": "user@example.com",
    "name": "홍길동"
  }
}
```

<a id="account-refresh-request"></a>
### Access Token 재발급 Request

```http
POST /accounts/refresh
Authorization: Bearer <expired-access-token>
Cookie: refresh_token=<refresh-token>
```

만료된 Access Token의 서명과 만료 상태를 확인하고, Refresh Token의 존재·유효기간·계정 일치 여부를 함께 검증합니다. Access Token이 아직 만료되지 않았거나 Cookie가 없으면 실패합니다.

<a id="account-refresh-response"></a>
### Access Token 재발급 Response

```json
{
  "success": true,
  "status": 200,
  "data": {
    "access_token": "<new-access-token>"
  }
}
```

기존 Refresh Token은 Rotation으로 폐기되고 새로운 `refresh_token` Cookie가 발급됩니다.

<a id="account-logout-request"></a>
### 로그아웃 Request

```http
POST /accounts/logout
Authorization: Bearer <access-token>
Cookie: refresh_token=<refresh-token>
```

Route 단위 Permission 검사는 없지만 `AuthenticatedUser`를 통한 Access Token 인증과 Refresh Cookie가 필요합니다.

<a id="account-logout-response"></a>
### 로그아웃 Response

```json
{
  "success": true,
  "status": 200
}
```

Redis·Dragonfly의 Refresh Session과 `refresh_token` Cookie가 삭제됩니다. 이미 발급된 Access Token은 만료 전까지 유효할 수 있습니다.

## Product API 상세

모든 Product API는 Access Token 인증이 필요합니다.


<a id="product-get-request"></a>
### 상품 단건 조회 Request

```http
GET /products/{id}
Authorization: Bearer <access-token>
```

`id`는 1 이상이어야 합니다.

<a id="product-get-response"></a>
### 상품 단건 조회 Response

```json
{
  "success": true,
  "status": 200,
  "data": {
    "id": 1,
    "name": "Keyboard",
    "description": "Mechanical keyboard",
    "price": 10000,
    "created_at": "2026-01-01T00:00:00Z",
    "updated_at": "2026-01-01T00:00:00Z"
  }
}
```

<a id="product-list-request"></a>
### 상품 목록 조회 Request

```http
GET /products?page=1&size=20&keyword=keyboard
Authorization: Bearer <access-token>
```

- `page`: 1 이상
- `size`: 1~100
- `keyword`: 선택값, 최대 20자

<a id="product-list-response"></a>
### 상품 목록 조회 Response

```json
{
  "success": true,
  "status": 200,
  "data": [
    {
      "id": 1,
      "name": "Keyboard",
      "description": "Mechanical keyboard",
      "price": 10000,
      "created_at": "2026-01-01T00:00:00Z",
      "updated_at": "2026-01-01T00:00:00Z"
    }
  ],
  "meta": {
    "count": 1,
    "total": 1,
    "page": 1,
    "size": 20,
    "total_pages": 1
  }
}
```

<a id="product-create-request"></a>
### 상품 생성 Request

```http
POST /products
Authorization: Bearer <access-token>
Content-Type: application/json
```

```json
{
  "name": "Keyboard",
  "description": "Mechanical keyboard",
  "price": 10000
}
```

- `name`: 1~100자 (공백만으로는 안 되며, 앞뒤 공백은 저장 시 제거됨)
- `description`: 최대 1000자
- `price`: 0 이상

<a id="product-create-response"></a>
### 상품 생성 Response

```json
{
  "success": true,
  "status": 201
}
```

<a id="product-put-request"></a>
### 상품 전체 수정 Request

```http
PUT /products/{id}
Authorization: Bearer <access-token>
Content-Type: application/json
```

```json
{
  "name": "Updated Keyboard",
  "description": "Updated description",
  "price": 12000
}
```

세 필드를 모두 전달해야 합니다. 각 필드에는 상품 생성과 같은 검증 규칙이 적용됩니다.

<a id="product-put-response"></a>
### 상품 전체 수정 Response

```json
{
  "success": true,
  "status": 200
}
```

<a id="product-patch-request"></a>
### 상품 일부 수정 Request

```http
PATCH /products/{id}
Authorization: Bearer <access-token>
Content-Type: application/json
```

```json
{
  "name": "Updated Keyboard",
  "price": 12000
}
```

`name`, `description`, `price` 중 하나 이상을 전달해야 합니다. 전달한 필드에만 DTO 검증 규칙이 적용됩니다.

하나도 전달하지 않으면 DTO 검증(422)이 아니라 Service 단 체크로 `400 BAD_REQUEST`가 반환됩니다.

<a id="product-patch-response"></a>
### 상품 일부 수정 Response

```json
{
  "success": true,
  "status": 200
}
```

<a id="product-delete-request"></a>
### 상품 삭제 Request

```http
DELETE /products/{id}
Authorization: Bearer <access-token>
```

`id`는 1 이상이어야 합니다.

<a id="product-delete-response"></a>
### 상품 삭제 Response

```json
{
  "success": true,
  "status": 200
}
```
