use crate::modules::orders::model::dto::CreateOrderRequest;
use crate::modules::orders::model::{Order, OrderStatus};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use std::io;

#[derive(FromRow)]
struct OrderRow {
    id: i64,
    client_id: i64,
    worker_id: i64,
    amount: i64,
    fee_amount: i64,
    worker_amount: i64,
    description: String,
    status: String,
    accepted_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    cancelled_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<OrderRow> for Order {
    type Error = sqlx::Error;

    fn try_from(row: OrderRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            client_id: row.client_id,
            worker_id: row.worker_id,
            amount: row.amount,
            fee_amount: row.fee_amount,
            worker_amount: row.worker_amount,
            description: row.description,
            status: parse_column(row.status, "status")?,
            accepted_at: row.accepted_at,
            completed_at: row.completed_at,
            cancelled_at: row.cancelled_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn parse_column<T>(value: String, column: &'static str) -> Result<T, sqlx::Error>
where
    T: std::str::FromStr<Err = String>,
{
    value.parse().map_err(|error| sqlx::Error::ColumnDecode {
        index: column.into(),
        source: Box::new(io::Error::new(io::ErrorKind::InvalidData, error)),
    })
}

pub async fn find_order_by_id_for_update(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i64,
) -> Result<Option<Order>, sqlx::Error> {
    let order = sqlx::query_as::<_, OrderRow>(
        r#"
        SELECT id, client_id, worker_id, amount, fee_amount, worker_amount, description, status,
               accepted_at, completed_at, cancelled_at, created_at, updated_at
        FROM orders
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(order_id)
    .fetch_optional(&mut **tx)
    .await?;

    order.map(Order::try_from).transpose()
}

pub async fn create_order(
    tx: &mut Transaction<'_, Postgres>,
    request: &CreateOrderRequest,
) -> Result<Order, sqlx::Error> {
    let fee_amount = request.amount / 10;
    let worker_amount = request.amount - fee_amount;

    let order = sqlx::query_as::<_, OrderRow>(
        r#"
        INSERT INTO orders (client_id, worker_id, amount, fee_amount, worker_amount, description, status)
        VALUES ($1, $2, $3, $4, $5, $6, 'Pending')
        RETURNING id, client_id, worker_id, amount, fee_amount, worker_amount, description, status,
                  accepted_at, completed_at, cancelled_at, created_at, updated_at
        "#,
    )
    .bind(request.client_id)
    .bind(request.worker_id)
    .bind(request.amount)
    .bind(fee_amount)
    .bind(worker_amount)
    .bind(request.description.trim())
    .fetch_one(&mut **tx)
    .await?;

    Order::try_from(order)
}

pub async fn update_order_status(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i64,
    status: OrderStatus,
) -> Result<Order, sqlx::Error> {
    let timestamp_column = match status {
        OrderStatus::Accepted => "accepted_at",
        OrderStatus::Completed => "completed_at",
        OrderStatus::Cancelled => "cancelled_at",
        OrderStatus::Pending => "updated_at",
    };
    let sql = format!(
        r#"
        UPDATE orders
        SET status = $1, {timestamp_column} = NOW(), updated_at = NOW()
        WHERE id = $2
        RETURNING id, client_id, worker_id, amount, fee_amount, worker_amount, description, status,
                  accepted_at, completed_at, cancelled_at, created_at, updated_at
        "#
    );

    let order = sqlx::query_as::<_, OrderRow>(&sql)
        .bind(status.to_string())
        .bind(order_id)
        .fetch_one(&mut **tx)
        .await?;

    Order::try_from(order)
}

pub async fn insert_platform_fee_record(
    tx: &mut Transaction<'_, Postgres>,
    order_id: i64,
    fee_amount: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO platform_fee_records (order_id, fee_amount)
        VALUES ($1, $2)
        "#,
    )
    .bind(order_id)
    .bind(fee_amount)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
