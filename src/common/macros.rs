// HTTP Method + Handler + Permission을 한 묶음으로 등록한다.
//
// 사용 예:
//
// secured_route!(
//     state.jwt.clone();
//     get(controller::get_product) => Product(GetProduct),
//     put(controller::put_product) => Product(PutProduct),
// );
//
// 동작 방식:
// 1. 각 HTTP Method별 MethodRouter를 독립적으로 생성
// 2. 각 MethodRouter에 Authorization + Authentication 적용
// 3. 마지막에 merge()하여 하나의 MethodRouter로 결합
#[macro_export]
macro_rules! secured_route {
    (
        $jwt:expr;
        $method:ident($handler:path) => $permission:expr $(,)?
    ) => {{
        axum::routing::$method($handler)
            // route_layer 실제 실행 순서는 Authentication → Authorization
            // Authorization
            .route_layer(
                axum::middleware::from_fn_with_state(
                    $permission,
                    $crate::common::auth::authorization::authorization_middleware,
                )
            )
            // Authentication
            .route_layer(
                axum::middleware::from_fn_with_state(
                    $jwt,
                    $crate::common::middleware::authentication_middleware,
                )
            )
    }};
    // 여러 Route
    (
        $jwt:expr;
        $method:ident($handler:path) => $permission:expr,
        $($rest:tt)+
    ) => {{
        // 현재 호출에서 전달받은 JwtConfig 값을 jwt 변수의 소유권으로 가져옴
        let jwt = $jwt;

        // 현재 MethodRouter에는 clone된 JwtConfig 소유권 전달
        let current = $crate::secured_route!(
            jwt.clone();
            $method($handler) => $permission
        );

        // 나머지 Route에는 원본 JwtConfig 소유권 전달
        let rest = $crate::secured_route!(
            jwt;
            $($rest)+
        );

        // merge로 하나의 MethodRouter로 결합
        current.merge(rest)
    }};
}
/*
 해석: 이 부분은 src/domains/product/mod.rs에 있는 내용과 같이 보면 좋음

 아래 코드를 예시로 설명해보자면
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

 $jwt:expr;는 표현식(expr)을 받는 매크로 변수로 여기에 들어오는 것은 state.jwt.clone()

 $method:ident에서 ident는 식별자를 받음, get(controller::get_product)에서 get을 의미
 axum::routing::$method($handler)는 axum::routing::get(controller::get_product)

 $handler:path는 경로(path)를 받는 matcher로 controller::get_product가 들어감

 $permission:expr은 Product(GetProduct)

 $($rest:tt)+는 tt는 TokenTree이고, +는 최소 1개 이상 반복을 의미
 put, patch, delete 애들을 의미

 정리하자면
 $method
→ get

$handler
→ controller::get_product

$permission
→ Product(GetProduct)

$rest
→ put(controller::put_product) => Product(PutProduct),
   delete(controller::delete_product) => Product(DeleteProduct),

추가로 $(,)?는 마지막 쉼표가 있어도 되고 없어도 된다는 의미

*/

/*
================================================================================
[secured_route! 매크로 재귀 동작 원리]
================================================================================

사용 예:

secured_route!(
    state.jwt.clone();
    get(controller::get_product) => Product(GetProduct),
    put(controller::put_product) => Product(PutProduct),
    patch(controller::patch_product) => Product(PatchProduct),
    delete(controller::delete_product) => Product(DeleteProduct),
);

GET + PUT + PATCH + DELETE
          │
          ▼
┌──────────────────────────────────────┐
│ 1단계                                 │
│                                      │
│ $method     = get                    │
│ $handler    = controller::get_product│
│ $permission = Product(GetProduct)    │
│                                      │
│ $rest = PUT + PATCH + DELETE         │
└──────────────────┬───────────────────┘
                   │
                   ▼
          PUT + PATCH + DELETE
                   │
                   ▼
┌─────────────────────────────────────┐
│ 2단계                              │
│                                     │
│ $method     = put                   │
│ $handler    = controller::put_product│
│ $permission = Product(PutProduct)   │
│                                     │
│ $rest = PATCH + DELETE              │
└──────────────────┬──────────────────┘
                   │
                   ▼
             PATCH + DELETE
                   │
                   ▼
┌─────────────────────────────────────┐
│ 3단계                              │
│                                     │
│ $method     = patch                 │
│ $handler    = controller::patch_product│
│ $permission = Product(PatchProduct) │
│                                     │
│ $rest = DELETE                      │
└──────────────────┬──────────────────┘
                   │
                   ▼
                 DELETE
                   │
                   ▼
┌─────────────────────────────────────┐
│ 4단계                              │
│                                     │
│ $method     = delete                │
│ $handler    = controller::delete_product│
│ $permission = Product(DeleteProduct)│
│                                     │
│ $rest 없음                          │
│ → 단일 Route 패턴 선택              │
│ → 재귀 종료                         │
└──────────────────┬──────────────────┘
                   │
                   ▼
-------------------------------------------------------------------------------
[merge()로 다시 하나의 MethodRouter로 결합]
-------------------------------------------------------------------------------

마지막 Route부터 결과가 다시 올라오면서 merge()된다.

DELETE
  ↓
PATCH + DELETE
  ↓
PUT + PATCH + DELETE
  ↓
GET + PUT + PATCH + DELETE

최종:

GET + PUT + PATCH + DELETE
        ↓
하나의 MethodRouter


즉 재귀는:

"하나씩 분리하면서 아래로 내려간다."
                ↓
"마지막 Route에서 재귀를 종료한다."
                ↓
"merge()하면서 다시 하나로 합쳐진다."

-------------------------------------------------------------------------------
[JwtConfig 소유권 흐름]
-------------------------------------------------------------------------------

let jwt = $jwt;

현재 MethodRouter:
jwt.clone()
→ current에 전달

나머지 Route:
jwt
→ rest에 전달

즉:

jwt
├── clone() → 현재 MethodRouter
└── move     → 나머지 Route

재귀가 반복되면서 각 MethodRouter가 사용할 JwtConfig가 하나씩 만들어진다.

호출부의:

state.jwt.clone()

은 매크로 전체에서 사용할 JwtConfig를 처음 확보하는 것이고,

재귀 내부의:

jwt.clone()

은 현재 MethodRouter와 나머지 Route가 각각 JwtConfig를 소유할 수 있도록
필요한 시점에 한 번 복사하는 것이다.
*/
