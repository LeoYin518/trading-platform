use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::atomic::{AtomicU64, Ordering};
use trading_platform::{
    errors::AppError,
    modules::orders::{
        OrderService,
        model::{OrderStatus, dto::CreateOrderRequest},
    },
};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct TestContext {
    pool: PgPool,
    service: OrderService,
    user_ids: Vec<i64>,
    order_ids: Vec<i64>,
}

impl TestContext {
    fn new(pool: PgPool) -> Self {
        Self {
            service: OrderService::new(pool.clone()),
            pool,
            user_ids: Vec::new(),
            order_ids: Vec::new(),
        }
    }

    async fn create_user(&mut self, role: &str, balance: i64) -> Result<i64, sqlx::Error> {
        let name = unique_name("user");
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO users (name, role, balance, frozen_balance)
            VALUES ($1, $2, $3, 0)
            RETURNING id
            "#,
        )
        .bind(name)
        .bind(role)
        .bind(balance)
        .fetch_one(&self.pool)
        .await?;

        self.user_ids.push(id);
        Ok(id)
    }

    async fn create_order(
        &mut self,
        client_id: i64,
        worker_id: i64,
        amount: i64,
        status: &str,
    ) -> Result<i64, sqlx::Error> {
        let fee_amount = amount / 10;
        let worker_amount = amount - fee_amount;
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO orders (
                client_id, worker_id, amount, fee_amount, worker_amount, description, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(client_id)
        .bind(worker_id)
        .bind(amount)
        .bind(fee_amount)
        .bind(worker_amount)
        .bind(unique_name("order"))
        .bind(status)
        .fetch_one(&self.pool)
        .await?;

        self.order_ids.push(id);
        Ok(id)
    }

    async fn cleanup(&self) -> Result<(), sqlx::Error> {
        for order_id in &self.order_ids {
            sqlx::query("DELETE FROM platform_fee_records WHERE order_id = $1")
                .bind(order_id)
                .execute(&self.pool)
                .await?;
            sqlx::query("DELETE FROM orders WHERE id = $1")
                .bind(order_id)
                .execute(&self.pool)
                .await?;
        }

        for user_id in &self.user_ids {
            sqlx::query("DELETE FROM users WHERE id = $1")
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }
}

fn unique_name(prefix: &str) -> String {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("orders_flow_{prefix}_{id}")
}

async fn test_context() -> Result<Option<TestContext>, sqlx::Error> {
    let Ok(database_url) = std::env::var("TEST_DATABASE_URL") else {
        return Ok(None);
    };
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    Ok(Some(TestContext::new(pool)))
}

async fn user_balances(pool: &PgPool, user_id: i64) -> Result<(i64, i64), sqlx::Error> {
    sqlx::query_as::<_, (i64, i64)>("SELECT balance, frozen_balance FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
}

async fn order_status(pool: &PgPool, order_id: i64) -> Result<String, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT status FROM orders WHERE id = $1")
        .bind(order_id)
        .fetch_one(pool)
        .await
}

#[tokio::test]
async fn create_order_persists_pending_order_with_fee_breakdown() -> TestResult {
    let Some(mut ctx) = test_context().await? else {
        return Ok(());
    };

    let client_id = ctx.create_user("Client", 100_000).await?;
    let worker_id = ctx.create_user("Worker", 0).await?;
    let order = ctx
        .service
        .create_order(CreateOrderRequest {
            client_id,
            worker_id,
            amount: 333,
            description: "verify fee breakdown".to_string(),
        })
        .await?;
    ctx.order_ids.push(order.id);

    assert_eq!(order.status, OrderStatus::Pending);
    assert_eq!(order.fee_amount, 33);
    assert_eq!(order.worker_amount, 300);

    let persisted = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT status, fee_amount, worker_amount FROM orders WHERE id = $1",
    )
    .bind(order.id)
    .fetch_one(&ctx.pool)
    .await?;
    assert_eq!(persisted, ("Pending".to_string(), 33, 300));

    ctx.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn accepting_order_freezes_client_available_balance() -> TestResult {
    let Some(mut ctx) = test_context().await? else {
        return Ok(());
    };

    let client_id = ctx.create_user("Client", 10_000).await?;
    let worker_id = ctx.create_user("Worker", 0).await?;
    let order_id = ctx
        .create_order(client_id, worker_id, 1_000, "Pending")
        .await?;

    ctx.service
        .update_order_status(order_id, "Accepted".to_string(), worker_id)
        .await?;

    assert_eq!(order_status(&ctx.pool, order_id).await?, "Accepted");
    assert_eq!(user_balances(&ctx.pool, client_id).await?, (9_000, 1_000));

    ctx.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn completing_order_settles_funds_and_records_platform_fee() -> TestResult {
    let Some(mut ctx) = test_context().await? else {
        return Ok(());
    };

    let client_id = ctx.create_user("Client", 10_000).await?;
    let worker_id = ctx.create_user("Worker", 200).await?;
    let order_id = ctx
        .create_order(client_id, worker_id, 1_000, "Pending")
        .await?;

    ctx.service
        .update_order_status(order_id, "Accepted".to_string(), worker_id)
        .await?;
    ctx.service
        .update_order_status(order_id, "Completed".to_string(), client_id)
        .await?;

    assert_eq!(order_status(&ctx.pool, order_id).await?, "Completed");
    assert_eq!(user_balances(&ctx.pool, client_id).await?, (9_000, 0));
    assert_eq!(user_balances(&ctx.pool, worker_id).await?, (1_100, 0));

    let fee_amount = sqlx::query_scalar::<_, i64>(
        "SELECT fee_amount FROM platform_fee_records WHERE order_id = $1",
    )
    .bind(order_id)
    .fetch_one(&ctx.pool)
    .await?;
    assert_eq!(fee_amount, 100);

    ctx.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn cancelling_pending_order_does_not_move_funds() -> TestResult {
    let Some(mut ctx) = test_context().await? else {
        return Ok(());
    };

    let client_id = ctx.create_user("Client", 10_000).await?;
    let worker_id = ctx.create_user("Worker", 0).await?;
    let order_id = ctx
        .create_order(client_id, worker_id, 1_000, "Pending")
        .await?;

    ctx.service
        .update_order_status(order_id, "Cancelled".to_string(), client_id)
        .await?;

    assert_eq!(order_status(&ctx.pool, order_id).await?, "Cancelled");
    assert_eq!(user_balances(&ctx.pool, client_id).await?, (10_000, 0));
    assert_eq!(user_balances(&ctx.pool, worker_id).await?, (0, 0));

    ctx.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn invalid_target_status_is_rejected_without_database_changes() -> TestResult {
    let Some(mut ctx) = test_context().await? else {
        return Ok(());
    };

    let client_id = ctx.create_user("Client", 10_000).await?;
    let worker_id = ctx.create_user("Worker", 0).await?;
    let order_id = ctx
        .create_order(client_id, worker_id, 1_000, "Pending")
        .await?;

    let error = ctx
        .service
        .update_order_status(order_id, "abc".to_string(), worker_id)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::BadRequest(_)));
    assert_eq!(order_status(&ctx.pool, order_id).await?, "Pending");
    assert_eq!(user_balances(&ctx.pool, client_id).await?, (10_000, 0));

    ctx.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn invalid_status_transition_does_not_move_funds() -> TestResult {
    let Some(mut ctx) = test_context().await? else {
        return Ok(());
    };

    let client_id = ctx.create_user("Client", 10_000).await?;
    let worker_id = ctx.create_user("Worker", 0).await?;
    let order_id = ctx
        .create_order(client_id, worker_id, 1_000, "Accepted")
        .await?;

    let error = ctx
        .service
        .update_order_status(order_id, "Cancelled".to_string(), client_id)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Conflict(_)));
    assert_eq!(order_status(&ctx.pool, order_id).await?, "Accepted");
    assert_eq!(user_balances(&ctx.pool, client_id).await?, (10_000, 0));

    ctx.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn insufficient_balance_cannot_accept_order() -> TestResult {
    let Some(mut ctx) = test_context().await? else {
        return Ok(());
    };

    let client_id = ctx.create_user("Client", 500).await?;
    let worker_id = ctx.create_user("Worker", 0).await?;
    let order_id = ctx
        .create_order(client_id, worker_id, 1_000, "Pending")
        .await?;

    let error = ctx
        .service
        .update_order_status(order_id, "Accepted".to_string(), worker_id)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Conflict(_)));
    assert_eq!(order_status(&ctx.pool, order_id).await?, "Pending");
    assert_eq!(user_balances(&ctx.pool, client_id).await?, (500, 0));

    ctx.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn only_assigned_worker_can_accept_order() -> TestResult {
    let Some(mut ctx) = test_context().await? else {
        return Ok(());
    };

    let client_id = ctx.create_user("Client", 10_000).await?;
    let worker_id = ctx.create_user("Worker", 0).await?;
    let other_worker_id = ctx.create_user("Worker", 0).await?;
    let order_id = ctx
        .create_order(client_id, worker_id, 1_000, "Pending")
        .await?;

    let error = ctx
        .service
        .update_order_status(order_id, "Accepted".to_string(), other_worker_id)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
    assert_eq!(order_status(&ctx.pool, order_id).await?, "Pending");

    ctx.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn only_client_can_complete_order() -> TestResult {
    let Some(mut ctx) = test_context().await? else {
        return Ok(());
    };

    let client_id = ctx.create_user("Client", 10_000).await?;
    let worker_id = ctx.create_user("Worker", 0).await?;
    let order_id = ctx
        .create_order(client_id, worker_id, 1_000, "Pending")
        .await?;

    ctx.service
        .update_order_status(order_id, "Accepted".to_string(), worker_id)
        .await?;
    let error = ctx
        .service
        .update_order_status(order_id, "Completed".to_string(), worker_id)
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Forbidden(_)));
    assert_eq!(order_status(&ctx.pool, order_id).await?, "Accepted");
    assert_eq!(user_balances(&ctx.pool, client_id).await?, (9_000, 1_000));

    ctx.cleanup().await?;
    Ok(())
}
