-- Add migration script here
CREATE TABLE account_ledger_entries (
    id BIGSERIAL PRIMARY KEY,
    ledger_group_id BIGINT NOT NULL,
    leg_no SMALLINT NOT NULL CHECK (leg_no IN (1, 2)),
    order_id BIGINT NOT NULL REFERENCES orders(id),
    user_id BIGINT NULL REFERENCES users(id),
    operator_id BIGINT NOT NULL REFERENCES users(id),
    account_type VARCHAR(32) NOT NULL CHECK (
        account_type IN ('UserAvailable', 'UserFrozen', 'PlatformFee')
    ),
    direction VARCHAR(8) NOT NULL CHECK (direction IN ('In', 'Out')),
    event_type VARCHAR(32) NOT NULL CHECK (
        event_type IN ('Freeze', 'Unfreeze', 'SettleWorker', 'PlatformFee')
    ),
    amount BIGINT NOT NULL CHECK (amount >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (ledger_group_id, leg_no)
);

COMMENT ON SEQUENCE account_ledger_group_id_seq IS '账本分组ID序列，用于关联一次可审计资金动作产生的两条流水';
COMMENT ON TABLE account_ledger_entries IS '账户账本流水表，以追加式双向分录记录订单资金流转';
COMMENT ON COLUMN account_ledger_entries.id IS '账本流水ID';
COMMENT ON COLUMN account_ledger_entries.ledger_group_id IS '账本分组ID，同一次资金动作的两条流水共享该ID';
COMMENT ON COLUMN account_ledger_entries.leg_no IS '分录序号，同一账本分组内固定为1或2';
COMMENT ON COLUMN account_ledger_entries.order_id IS '来源订单ID';
COMMENT ON COLUMN account_ledger_entries.user_id IS '用户账户ID；平台账户流水为空';
COMMENT ON COLUMN account_ledger_entries.operator_id IS '触发订单状态流转的操作人ID';
COMMENT ON COLUMN account_ledger_entries.account_type IS '账户类型：用户可用余额、用户冻结余额或平台手续费账户';
COMMENT ON COLUMN account_ledger_entries.direction IS '流水方向：In=入账，Out=出账';
COMMENT ON COLUMN account_ledger_entries.event_type IS '产生流水的业务事件类型';
COMMENT ON COLUMN account_ledger_entries.amount IS '流水金额，单位为分';
COMMENT ON COLUMN account_ledger_entries.created_at IS '创建时间';

CREATE INDEX idx_account_ledger_entries_order_id ON account_ledger_entries(order_id);
CREATE INDEX idx_account_ledger_entries_user_id ON account_ledger_entries(user_id);
CREATE INDEX idx_account_ledger_entries_group_id ON account_ledger_entries(ledger_group_id);
