use tracing::Level;
use trading_platform::router::build_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let app = build_router(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    tracing::info!("Server is running, listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
