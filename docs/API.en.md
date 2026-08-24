# API Documentation

This document is organized so you can see the API at a glance from the frontend and jump directly to the detailed Request/Response content you need.

## Base URL

```text
http://127.0.0.1:8080
```

## API Overview

| Feature | API Method & URL | Auth / Request Conditions | Request | Response | Permission |
|---|---|---|---|---|---|
| Sign up | `POST /accounts` | No auth required | [Request](#account-create-request) | [Response](#account-create-response) | - |
| Log in | `POST /accounts/login` | No auth required | [Request](#account-login-request) | [Response](#account-login-response) | - |
| Get my account | `GET /accounts/me` | Access Token required | [Request](#account-me-request) | [Response](#account-me-response) | [See](#role-permission-table) |
| Reissue Access Token | `POST /accounts/refresh` | Expired Access Token + Refresh Cookie required | [Request](#account-refresh-request) | [Response](#account-refresh-response) | - |
| Log out | `POST /accounts/logout` | Access Token + Refresh Cookie required | [Request](#account-logout-request) | [Response](#account-logout-response) | - |
| List products | `GET /products?page=1&size=20&keyword=keyboard` | Access Token required | [Request](#product-list-request) | [Response](#product-list-response) | [See](#role-permission-table) |
| Get a single product | `GET /products/{id}` | Access Token required | [Request](#product-get-request) | [Response](#product-get-response) | [See](#role-permission-table) |
| Create a product | `POST /products` | Access Token required | [Request](#product-create-request) | [Response](#product-create-response) | [See](#role-permission-table) |
| Fully update a product | `PUT /products/{id}` | Access Token required | [Request](#product-put-request) | [Response](#product-put-response) | [See](#role-permission-table) |
| Partially update a product | `PATCH /products/{id}` | Access Token required | [Request](#product-patch-request) | [Response](#product-patch-response) | [See](#role-permission-table) |
| Delete a product | `DELETE /products/{id}` | Access Token required | [Request](#product-delete-request) | [Response](#product-delete-response) | [See](#role-permission-table) |

<a id="role-permission-table"></a>
### API Permissions by Role

`-` indicates an API that does not apply per-role Permission checks.

| API | Admin | Employee | Customer |
|---|---|---|---|
| Sign up | - | - | - |
| Log in | - | - | - |
| Get my account | ✅ | ✅ | ✅ |
| Reissue Access Token | - | - | - |
| Log out | - | - | - |
| Get product (single) | ✅ | ✅ | ✅ |
| Get product (list) | ✅ | ✅ | ✅ |
| Create product | ✅ | ✅ | ❌ |
| Fully update product (PUT) | ✅ | ✅ | ❌ |
| Partially update product (PATCH) | ✅ | ✅ | ❌ |
| Delete product | ✅ | ❌ | ❌ |

## Common Authentication

The Access Token is passed via the following header.

```http
Authorization: Bearer <access-token>
```

The Refresh Token is delivered as an `HttpOnly`, `Secure`, `SameSite=Strict` cookie in the login/reissue response. The Refresh API requires both an expired Access Token and a valid Refresh cookie.

## Common Responses

### Success - Includes a Single Data Object

```json
{
  "success": true,
  "status": 200,
  "data": {}
}
```

### Success - Includes List Data and meta

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

- `count`: Number of items included in the current page
- `total`: Total number of results across the search
- `page`: Current page number
- `size`: Number of items per page
- `total_pages`: Total number of pages

### Success - No Data

```json
{
  "success": true,
  "status": 201
}
```

### Failure

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

| HTTP Status | code | Meaning |
|---:|---|---|
| 400 | `BAD_REQUEST` | Malformed JSON/path/query or an otherwise invalid request |
| 401 | `UNAUTHORIZED` | Missing authentication, token error, or Refresh Session error |
| 401 | `INVALID_CREDENTIALS` | Email and password do not match |
| 403 | `FORBIDDEN` | Insufficient access permission |
| 404 | `NOT_FOUND` | Target resource or route does not exist |
| 422 | `VALIDATION_ERROR` | DTO validation failed |
| 500 | `INTERNAL_SERVER_ERROR` | Internal server error |

### Scope of Error Information Disclosed

For security reasons, detailed internal server error causes are not exposed in the response.
Detailed causes and debugging information are designed to be looked up in the backend logs using the `request_id`.

## Account API Details

<a id="account-create-request"></a>
### Sign Up Request

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

No authentication required.

- `email`: Must be a valid email format
- `password`: 8-100 characters with no spaces, containing at least 3 of the following character types: uppercase letters, lowercase letters, digits, and other characters (special symbols, Korean characters, emoji, etc.)
- `name`: 1-50 characters (cannot be whitespace only; leading/trailing whitespace is trimmed on save)
- `role`: `customer` or `employee`
- The `admin` role cannot be used in a regular sign-up request

<a id="account-create-response"></a>
### Sign Up Response

```json
{
  "success": true,
  "status": 201
}
```

<a id="account-login-request"></a>
### Log In Request

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

- `email`: Must be a valid email format
- `password`: At least 1 character (cannot be empty)

<a id="account-login-response"></a>
### Log In Response

```json
{
  "success": true,
  "status": 200,
  "data": {
    "access_token": "<access-token>"
  }
}
```

The Refresh Token is not returned in the JSON body but as a `refresh_token` cookie. If the email and password do not match, `INVALID_CREDENTIALS` is returned.

<a id="account-me-request"></a>
### Get My Account Request

```http
GET /accounts/me
Authorization: Bearer <access-token>
```

<a id="account-me-response"></a>
### Get My Account Response

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
### Reissue Access Token Request

```http
POST /accounts/refresh
Authorization: Bearer <expired-access-token>
Cookie: refresh_token=<refresh-token>
```

The signature and expiration state of the expired Access Token are checked, along with the Refresh Token's existence, validity period, and whether it matches the account. This fails if the Access Token has not yet expired or if the cookie is missing.

<a id="account-refresh-response"></a>
### Reissue Access Token Response

```json
{
  "success": true,
  "status": 200,
  "data": {
    "access_token": "<new-access-token>"
  }
}
```

The existing Refresh Token is revoked through rotation, and a new `refresh_token` cookie is issued.

<a id="account-logout-request"></a>
### Log Out Request

```http
POST /accounts/logout
Authorization: Bearer <access-token>
Cookie: refresh_token=<refresh-token>
```

There is no route-level Permission check, but Access Token authentication via `AuthenticatedUser` and a Refresh cookie are required.

<a id="account-logout-response"></a>
### Log Out Response

```json
{
  "success": true,
  "status": 200
}
```

The Refresh Session in Redis/Dragonfly and the `refresh_token` cookie are deleted. An already-issued Access Token may remain valid until it expires.

## Product API Details

All Product APIs require Access Token authentication.


<a id="product-get-request"></a>
### Get a Single Product Request

```http
GET /products/{id}
Authorization: Bearer <access-token>
```

`id` must be 1 or greater.

<a id="product-get-response"></a>
### Get a Single Product Response

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
### List Products Request

```http
GET /products?page=1&size=20&keyword=keyboard
Authorization: Bearer <access-token>
```

- `page`: 1 or greater
- `size`: 1-100
- `keyword`: Optional, up to 20 characters

<a id="product-list-response"></a>
### List Products Response

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
### Create Product Request

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

- `name`: 1-100 characters (cannot be whitespace only; leading/trailing whitespace is trimmed on save)
- `description`: Up to 1000 characters
- `price`: 0 or greater

<a id="product-create-response"></a>
### Create Product Response

```json
{
  "success": true,
  "status": 201
}
```

<a id="product-put-request"></a>
### Fully Update Product Request

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

All three fields must be provided. The same validation rules as product creation apply to each field.

<a id="product-put-response"></a>
### Fully Update Product Response

```json
{
  "success": true,
  "status": 200
}
```

<a id="product-patch-request"></a>
### Partially Update Product Request

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

At least one of `name`, `description`, or `price` must be provided. DTO validation rules apply only to the fields that are provided.

If none are provided, this returns `400 BAD_REQUEST` from a service-level check rather than a DTO validation error (422).

<a id="product-patch-response"></a>
### Partially Update Product Response

```json
{
  "success": true,
  "status": 200
}
```

<a id="product-delete-request"></a>
### Delete Product Request

```http
DELETE /products/{id}
Authorization: Bearer <access-token>
```

`id` must be 1 or greater.

<a id="product-delete-response"></a>
### Delete Product Response

```json
{
  "success": true,
  "status": 200
}
```
