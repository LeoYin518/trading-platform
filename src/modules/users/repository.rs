use crate::modules::users::model::User;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use std::io;

#[derive(FromRow)]
struct UserRow {
    id: i64,
    name: String,
    role: String,
    balance: i64,
    frozen_balance: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = sqlx::Error;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            role: parse_column(row.role, "role")?,
            balance: row.balance,
            frozen_balance: row.frozen_balance,
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

pub async fn find_user_by_id_for_update(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, name, role, balance, frozen_balance, created_at, updated_at
        FROM users
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?;

    user.map(User::try_from).transpose()
}

pub async fn transfer_available_to_frozen(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    amount: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET balance = balance - $1,
            frozen_balance = frozen_balance + $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(amount)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn decrease_frozen_balance(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    amount: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET frozen_balance = frozen_balance - $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(amount)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn increase_available_balance(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    amount: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET balance = balance + $1,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(amount)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
