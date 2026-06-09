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
