use crate::common::{
    auth::{
        extractor::AuthenticatedUser,
        permission::{AccountPermission, Permission, ProductPermission},
        role::AccountRole,
    },
    error::AppError,
};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::collections::HashSet;

// 단일 Role이 가진 Permission 집합을 반환하는 내부 헬퍼
fn permissions_for_role(role: AccountRole) -> HashSet<Permission> {
    match role {
        AccountRole::Admin => HashSet::from([
            Permission::Account(AccountPermission::GetMe),
            Permission::Product(ProductPermission::CreateProduct),
            Permission::Product(ProductPermission::GetProduct),
            Permission::Product(ProductPermission::GetProducts),
            Permission::Product(ProductPermission::PutProduct),
            Permission::Product(ProductPermission::PatchProduct),
            Permission::Product(ProductPermission::DeleteProduct),
        ]),

        AccountRole::Employee => HashSet::from([
            Permission::Account(AccountPermission::GetMe),
            Permission::Product(ProductPermission::GetProduct),
            Permission::Product(ProductPermission::GetProducts),
            Permission::Product(ProductPermission::CreateProduct),
            Permission::Product(ProductPermission::PutProduct),
            Permission::Product(ProductPermission::PatchProduct),
        ]),

        AccountRole::Customer => HashSet::from([
            Permission::Account(AccountPermission::GetMe),
            Permission::Product(ProductPermission::GetProduct),
            Permission::Product(ProductPermission::GetProducts),
        ]),
    }
}

// 여러 Role의 Permission 합집합으로 반환: [여기에 합집합 예시로...]
fn permissions_for_roles(roles: &[AccountRole]) -> HashSet<Permission> {
    roles
        .iter()
        .flat_map(|role| permissions_for_role(*role))
        .collect()
}

// 특정 Role 목록이 요구된 Permission을 가지고 있는지 확인
fn has_permission(roles: &[AccountRole], required: Permission) -> bool {
    permissions_for_roles(roles).contains(&required)
}

// Authorization Middleware
// Authentication Middleware가 먼저 생성한
// AuthenticatedUser를 request.extensions()에서 가져와
// 요구된 Permission을 검사
// 인증 자체는 수행하지 않음
pub async fn authorization_middleware(
    State(required): State<Permission>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .ok_or_else(|| AppError::Internal("인증된 사용자 정보가 없습니다.".into()))?;

    if !has_permission(&user.roles, required) {
        return Err(AppError::Forbidden);
    }

    Ok(next.run(request).await)
}

// cargo test --lib common::auth::authorization
// cargo test --lib common::auth::authorization -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;

    // 시나리오 1: Admin 역할의 모든 권한 허용 여부 전수 검증
    #[test]
    fn test_admin_permissions_all_cases() {
        let admin = &[AccountRole::Admin];

        assert!(has_permission(
            admin,
            Permission::Account(AccountPermission::GetMe)
        ));
        assert!(has_permission(
            admin,
            Permission::Product(ProductPermission::CreateProduct)
        ));
        assert!(has_permission(
            admin,
            Permission::Product(ProductPermission::GetProduct)
        ));
        assert!(has_permission(
            admin,
            Permission::Product(ProductPermission::GetProducts)
        ));
        assert!(has_permission(
            admin,
            Permission::Product(ProductPermission::PutProduct)
        ));
        assert!(has_permission(
            admin,
            Permission::Product(ProductPermission::PatchProduct)
        ));
        assert!(has_permission(
            admin,
            Permission::Product(ProductPermission::DeleteProduct)
        ));
    }

    // 시나리오 2: Employee 역할의 권한 세부 분기 전수 검증 (Delete만 전용 차단)
    #[test]
    fn test_employee_permissions_all_cases() {
        let employee = &[AccountRole::Employee];

        assert!(has_permission(
            employee,
            Permission::Account(AccountPermission::GetMe)
        ));
        assert!(has_permission(
            employee,
            Permission::Product(ProductPermission::CreateProduct)
        ));
        assert!(has_permission(
            employee,
            Permission::Product(ProductPermission::GetProduct)
        ));
        assert!(has_permission(
            employee,
            Permission::Product(ProductPermission::GetProducts)
        ));
        assert!(has_permission(
            employee,
            Permission::Product(ProductPermission::PutProduct)
        ));
        assert!(has_permission(
            employee,
            Permission::Product(ProductPermission::PatchProduct)
        ));

        // 직원은 제품 삭제 권한이 무조건 없어야 성공!
        assert!(!has_permission(
            employee,
            Permission::Product(ProductPermission::DeleteProduct)
        ));
    }

    // 시나리오 3: Customer 역할의 권한 세부 분기 전수 검증 (조회 및 내 정보 외엔 전부 차단)
    #[test]
    fn test_customer_permissions_all_cases() {
        let customer = &[AccountRole::Customer];

        // 허용 케이스
        assert!(has_permission(
            customer,
            Permission::Account(AccountPermission::GetMe)
        ));
        assert!(has_permission(
            customer,
            Permission::Product(ProductPermission::GetProduct)
        ));
        assert!(has_permission(
            customer,
            Permission::Product(ProductPermission::GetProducts)
        ));

        // 차단 케이스 (쓰기, 수정, 삭제 일절 차단)
        assert!(!has_permission(
            customer,
            Permission::Product(ProductPermission::CreateProduct)
        ));
        assert!(!has_permission(
            customer,
            Permission::Product(ProductPermission::PutProduct)
        ));
        assert!(!has_permission(
            customer,
            Permission::Product(ProductPermission::PatchProduct)
        ));
        assert!(!has_permission(
            customer,
            Permission::Product(ProductPermission::DeleteProduct)
        ));
    }

    // 시나리오 4: 여러 Role을 가진 사용자가 각 Role의 Permission 합집합으로 권한을 가지는지 검증
    #[test]
    fn test_multiple_roles_union_matrix() {
        // Customer(조회만 가능)와 Employee(쓰기/수정 가능)가 동시에 세팅된 특수 계정
        let mixed_roles = [AccountRole::Customer, AccountRole::Employee];

        // 두 역할의 권한이 유기적으로 합쳐져 통과되는지 확인
        assert!(has_permission(
            &mixed_roles,
            Permission::Product(ProductPermission::CreateProduct)
        ));
        assert!(has_permission(
            &mixed_roles,
            Permission::Product(ProductPermission::GetProduct)
        ));
        assert!(has_permission(
            &mixed_roles,
            Permission::Product(ProductPermission::PatchProduct)
        ));

        // 여전히 두 역할 모두 Delete 권한은 소유하지 않으므로 거부 처리되어야 정상!
        assert!(!has_permission(
            &mixed_roles,
            Permission::Product(ProductPermission::DeleteProduct)
        ));

        // 여기에 초강력 Admin까지 가세한다면? -> 드디어 삭제 권한까지 자동 개방되는지 검증
        let super_roles = [
            AccountRole::Customer,
            AccountRole::Employee,
            AccountRole::Admin,
        ];
        assert!(has_permission(
            &super_roles,
            Permission::Product(ProductPermission::DeleteProduct)
        ));
    }

    // 시나리오 5: 권한이 비어있는 경우 거부 되어야 함
    #[test]
    fn test_empty_roles_have_no_permission() {
        let roles: [AccountRole; 0] = [];

        assert!(!has_permission(
            &roles,
            Permission::Product(ProductPermission::GetProduct),
        ));
    }
}
