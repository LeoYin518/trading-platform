DROP TABLE IF EXISTS users;

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    role VARCHAR(20) NOT NULL CHECK (role IN ('Client', 'Worker')),
    balance BIGINT NOT NULL DEFAULT 0 CHECK (balance >= 0),
    frozen_balance BIGINT NOT NULL DEFAULT 0 CHECK (frozen_balance >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE users IS '用户表，保存需求方和服务方的基础信息与账户余额';
COMMENT ON COLUMN users.id IS '用户ID';
COMMENT ON COLUMN users.name IS '用户名称';
COMMENT ON COLUMN users.role IS '用户角色：Client=需求方，Worker=服务方';
COMMENT ON COLUMN users.balance IS '可用余额，单位为分';
COMMENT ON COLUMN users.frozen_balance IS '冻结余额，单位为分，服务方接单后冻结需求方资金';
COMMENT ON COLUMN users.created_at IS '创建时间';
COMMENT ON COLUMN users.updated_at IS '更新时间';


INSERT INTO users (id, name, role, balance, frozen_balance)
VALUES
    (1, '测试需求方A', 'Client', 100000, 0),
    (2, '测试需求方B', 'Client', 500, 0),
    (3, '测试服务方A', 'Worker', 0, 0),
    (4, '测试服务方B', 'Worker', 2000, 0);



SELECT setval(pg_get_serial_sequence('users', 'id'), (SELECT MAX(id) FROM users));
