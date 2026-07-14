use std::net::SocketAddr;

use equipment_reservation::{
    app_router, telemetry, AppState, Config,
};
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    let telemetry_handles = match telemetry::init_telemetry(&config.service_name, &config.otel_endpoint)
    {
        Ok(h) => Some(h),
        Err(err) => {
            eprintln!("telemetry init failed (continuing without OTLP export): {err:#}");
            // Fallback logging without OTel layer
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
                )
                .init();
            None
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let db = equipment_reservation::infra::Db::new(pool);
    let store = equipment_reservation::infra::ObjectStore::new(&config).await?;
    let state = AppState::new(db, store, config.clone());
    let app = app_router(state);

    let addr: SocketAddr = config.bind_addr.parse()?;
    info!(%addr, "listening");
    let listener = TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    if let Some(handles) = telemetry_handles {
        telemetry::shutdown(handles);
    }
    Ok(())
}
