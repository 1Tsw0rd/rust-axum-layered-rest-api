#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    Account(AccountPermission),
    Product(ProductPermission),
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Account(permission) => permission.as_str(),
            Self::Product(permission) => permission.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountPermission {
    // CreateAccount, 회원가입은 권한이 없어도 접근 가능한 api
    GetMe,

}

impl AccountPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GetMe => "account:get_me",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductPermission {
    CreateProduct,
    GetProduct,
    GetProducts,
    PutProduct,
    PatchProduct,
    DeleteProduct,
}

impl ProductPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CreateProduct => "product:create_product",
            Self::GetProduct => "product:get_product",
            Self::GetProducts => "product:get_products",
            Self::PutProduct => "product:put_product",
            Self::PatchProduct => "product:patch_product",
            Self::DeleteProduct => "product:delete_product",
        }
    }
}

// cargo test --lib common::auth::permission
// cargo test --lib common::auth::permission -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;

    // 시나리오 1: Account Permission이 정의된 문자열로 정상 변환되는지 검증
    #[test]
    fn account_permission_to_string() {
        assert_eq!(
            Permission::Account(AccountPermission::GetMe).as_str(),
            "account:get_me"
        );
    }

    // 시나리오 2: Product Permission이 각각 정의된 문자열로 정상 변환되는지 전수 검증
    #[test]
    fn product_permissions_to_string() {
        assert_eq!(
            Permission::Product(ProductPermission::CreateProduct).as_str(),
            "product:create_product"
        );

        assert_eq!(
            Permission::Product(ProductPermission::GetProduct).as_str(),
            "product:get_product"
        );

        assert_eq!(
            Permission::Product(ProductPermission::GetProducts).as_str(),
            "product:get_products"
        );

        assert_eq!(
            Permission::Product(ProductPermission::PutProduct).as_str(),
            "product:put_product"
        );

        assert_eq!(
            Permission::Product(ProductPermission::PatchProduct).as_str(),
            "product:patch_product"
        );

        assert_eq!(
            Permission::Product(ProductPermission::DeleteProduct).as_str(),
            "product:delete_product"
        );
    }
}
