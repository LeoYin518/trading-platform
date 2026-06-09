DROP TABLE IF EXISTS platform_fee_records;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS users;

ALTER DATABASE postgres_db SET timezone TO 'Asia/Shanghai';

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

CREATE TABLE orders (
    id BIGSERIAL PRIMARY KEY,
    client_id BIGINT NOT NULL REFERENCES users(id),
    worker_id BIGINT NOT NULL REFERENCES users(id),
    amount BIGINT NOT NULL CHECK (amount > 0),
    fee_amount BIGINT NOT NULL CHECK (fee_amount >= 0),
    worker_amount BIGINT NOT NULL CHECK (worker_amount >= 0),
    description TEXT NOT NULL,
    status VARCHAR(20) NOT NULL CHECK (status IN ('Pending', 'Accepted', 'Completed', 'Cancelled')),
    accepted_at TIMESTAMPTZ NULL,
    completed_at TIMESTAMPTZ NULL,
    cancelled_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (worker_amount + fee_amount = amount)
);

COMMENT ON TABLE orders IS '订单表，保存需求订单、状态流转以及结算拆分金额';
COMMENT ON COLUMN orders.id IS '订单ID';
COMMENT ON COLUMN orders.client_id IS '需求方用户ID';
COMMENT ON COLUMN orders.worker_id IS '服务方用户ID';
COMMENT ON COLUMN orders.amount IS '订单总金额，单位为分';
COMMENT ON COLUMN orders.fee_amount IS '平台服务费，按订单总额10%向下取整，单位为分';
COMMENT ON COLUMN orders.worker_amount IS '服务方应结算金额，等于订单总额减平台服务费，单位为分';
COMMENT ON COLUMN orders.description IS '需求描述';
COMMENT ON COLUMN orders.status IS '订单状态：Pending=待接单，Accepted=已接单，Completed=已完成，Cancelled=已取消';
COMMENT ON COLUMN orders.accepted_at IS '服务方接单时间';
COMMENT ON COLUMN orders.completed_at IS '需求方确认完成时间';
COMMENT ON COLUMN orders.cancelled_at IS '订单取消时间';
COMMENT ON COLUMN orders.created_at IS '创建时间';
COMMENT ON COLUMN orders.updated_at IS '更新时间';

CREATE INDEX idx_orders_client_id ON orders(client_id);
CREATE INDEX idx_orders_worker_id ON orders(worker_id);
CREATE INDEX idx_orders_status ON orders(status);

CREATE TABLE platform_fee_records (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL UNIQUE REFERENCES orders(id),
    fee_amount BIGINT NOT NULL CHECK (fee_amount >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE platform_fee_records IS '平台手续费记录表，仅在订单完成结算时记录平台收入';
COMMENT ON COLUMN platform_fee_records.id IS '平台手续费记录ID';
COMMENT ON COLUMN platform_fee_records.order_id IS '来源订单ID';
COMMENT ON COLUMN platform_fee_records.fee_amount IS '平台手续费金额，单位为分';
COMMENT ON COLUMN platform_fee_records.created_at IS '创建时间';

INSERT INTO users (id, name, role, balance, frozen_balance)
VALUES
    (1, '测试需求方A', 'Client', 100000, 0),
    (2, '测试需求方B', 'Client', 500, 0),
    (3, '测试服务方A', 'Worker', 0, 0),
    (4, '测试服务方B', 'Worker', 2000, 0);

INSERT INTO orders (id, client_id, worker_id, amount, fee_amount, worker_amount, description, status)
VALUES
    (1, 1, 3, 10000, 1000, 9000, '示例待接单订单', 'Pending'),
    (2, 1, 4, 333, 33, 300, '示例小额订单，用于验证手续费向下取整', 'Pending');

SELECT setval(pg_get_serial_sequence('users', 'id'), (SELECT MAX(id) FROM users));
SELECT setval(pg_get_serial_sequence('orders', 'id'), (SELECT MAX(id) FROM orders));
