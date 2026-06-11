use sqlx::{Postgres, Transaction};

pub async fn insert_freeze_entries(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i64,
    client_id: i64,
    operator_id: i64,
    amount: i64,
) -> Result<(), sqlx::Error> {
    insert_double_entry(
        tx,
        LedgerEntry {
            order_id,
            user_id: Some(client_id),
            operator_id,
            account_type: "UserAvailable",
            direction: "Out",
            event_type: "Freeze",
            amount,
        },
        LedgerEntry {
            order_id,
            user_id: Some(client_id),
            operator_id,
            account_type: "UserFrozen",
            direction: "In",
            event_type: "Freeze",
            amount,
        },
    )
    .await
}

pub async fn insert_worker_settlement_entries(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i64,
    client_id: i64,
    worker_id: i64,
    operator_id: i64,
    amount: i64,
) -> Result<(), sqlx::Error> {
    insert_double_entry(
        tx,
        LedgerEntry {
            order_id,
            user_id: Some(client_id),
            operator_id,
            account_type: "UserFrozen",
            direction: "Out",
            event_type: "SettleWorker",
            amount,
        },
        LedgerEntry {
            order_id,
            user_id: Some(worker_id),
            operator_id,
            account_type: "UserAvailable",
            direction: "In",
            event_type: "SettleWorker",
            amount,
        },
    )
    .await
}

pub async fn insert_platform_fee_entries(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i64,
    client_id: i64,
    operator_id: i64,
    fee_amount: i64,
) -> Result<(), sqlx::Error> {
    insert_double_entry(
        tx,
        LedgerEntry {
            order_id,
            user_id: Some(client_id),
            operator_id,
            account_type: "UserFrozen",
            direction: "Out",
            event_type: "PlatformFee",
            amount: fee_amount,
        },
        LedgerEntry {
            order_id,
            user_id: None,
            operator_id,
            account_type: "PlatformFee",
            direction: "In",
            event_type: "PlatformFee",
            amount: fee_amount,
        },
    )
    .await
}

struct LedgerEntry {
    order_id: i64,
    user_id: Option<i64>,
    operator_id: i64,
    account_type: &'static str,
    direction: &'static str,
    event_type: &'static str,
    amount: i64,
}

async fn insert_double_entry(
    tx: &mut Transaction<'_, Postgres>,
    debit: LedgerEntry,
    credit: LedgerEntry,
) -> Result<(), sqlx::Error> {
    let ledger_group_id =
        sqlx::query_scalar::<_, i64>("SELECT nextval('account_ledger_group_id_seq')::BIGINT")
            .fetch_one(&mut **tx)
            .await?;

    insert_entry(tx, ledger_group_id, 1, debit).await?;
    insert_entry(tx, ledger_group_id, 2, credit).await
}

async fn insert_entry(
    tx: &mut Transaction<'_, Postgres>,
    ledger_group_id: i64,
    leg_no: i16,
    entry: LedgerEntry,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO account_ledger_entries (
            ledger_group_id, leg_no, order_id, user_id, operator_id,
            account_type, direction, event_type, amount
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(ledger_group_id)
    .bind(leg_no)
    .bind(entry.order_id)
    .bind(entry.user_id)
    .bind(entry.operator_id)
    .bind(entry.account_type)
    .bind(entry.direction)
    .bind(entry.event_type)
    .bind(entry.amount)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
