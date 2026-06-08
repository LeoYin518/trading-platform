use trading_platform::config::AppConfig;
use trading_platform::router::build_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env()?;

    tracing_subscriber::fmt()
        .with_max_level(config.tracing.level)
        .init();

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;
    let app = build_router(pool);

    let listener = tokio::net::TcpListener::bind(config.server.addr()).await?;

    tracing::info!("Server is running, listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
