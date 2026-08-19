-- postgres 초기화 SQL 스크립트
-- 1. updated_at 시간 갱신 함수 정의
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 2. 테이블 생성
-- 1) 계정 테이블 생성
CREATE TABLE IF NOT EXISTS accounts (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2) 역할 테이블 생성
CREATE TABLE IF NOT EXISTS roles (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3) 계정-역할 연결 테이블 생성
-- 하나의 계정이 여러 역할을 가질 수 있도록 N:M 관계로 구성
CREATE TABLE IF NOT EXISTS account_roles (
    account_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,

    PRIMARY KEY (account_id, role_id),

    CONSTRAINT fk_account_roles_account
        FOREIGN KEY (account_id)
        REFERENCES accounts(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_account_roles_role
        FOREIGN KEY (role_id)
        REFERENCES roles(id)
        ON DELETE CASCADE
);

-- 4) 제품 테이블 생성
CREATE TABLE IF NOT EXISTS products (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description VARCHAR(1000) NOT NULL,
    price BIGINT NOT NULL CHECK (price >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3. 명시적 트리거 센서 부착
-- 테이블에서 update 발생 이전에 각 행의 updated_at 컬럼을 현재 시간으로 갱신하는 트리거 생성
CREATE TRIGGER tr_accounts_updated_at
    BEFORE UPDATE ON accounts
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER tr_roles_updated_at
    BEFORE UPDATE ON roles
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();

CREATE TRIGGER tr_products_updated_at
    BEFORE UPDATE ON products
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();

-- 4. 기본 데이터 INSERT
-- ON CONFLICT (name) DO NOTHING 뜻은 데이터가 이미 존재할 경우 에러 내지말고 통과하라는 의미(Upsert)
INSERT INTO roles (name) VALUES
('admin'),
('customer'),
('employee')
ON CONFLICT (name) DO NOTHING;

-- 기본 관리자 계정
-- 로그인:
-- email: admin@example.com
-- password: Admin123!
INSERT INTO accounts (
    email,
    password_hash,
    name
) VALUES (
    'admin@test.com',
    '$argon2id$v=19$m=19456,t=2,p=1$ujqD8M56K+UgbpTxMpatuQ$u3Fb4cO8k6Zxs6p0cE5SVQWqZ8nTq350dWq1oP2WL4c',
    '관리자'
)
ON CONFLICT (email) DO NOTHING;

-- 관리자 계정에 Admin Role 연결
INSERT INTO account_roles (
    account_id,
    role_id
)
SELECT
    a.id,
    r.id
FROM accounts a
CROSS JOIN roles r
WHERE a.email = 'admin@test.com'
  AND r.name = 'admin'
ON CONFLICT (account_id, role_id) DO NOTHING;