//! Punto de entrada de la API de negocio (axum + sqlx + SQLite).
//!
//! Al arrancar crea los directorios de datos, abre/crea data/app.db con WAL,
//! ejecuta las migraciones y levanta el servidor HTTP en 0.0.0.0:API_PORT.

mod config;
mod engine;
mod error;
mod models;
mod providers;
mod routes;

use std::net::SocketAddr;

use axum::extract::DefaultBodyLimit;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::engine::RecognizerEngine;
use crate::providers::PriceProviderKind;

/// Tamano maximo de subida (fotos de cartas).
const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;

/// Estado compartido por todos los handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Config,
    pub engine: Option<RecognizerEngine>,
    pub price_provider: PriceProviderKind,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;

    // Directorios de datos (data, data/scans, data/images) si faltan.
    std::fs::create_dir_all(&config.data_dir)?;
    std::fs::create_dir_all(config.data_dir.join("scans"))?;
    std::fs::create_dir_all(config.data_dir.join("images"))?;
    if let Some(parent) = config.database_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Pool SQLite con creacion automatica y journal_mode WAL.
    // busy_timeout deja que las escrituras esperen en vez de fallar con
    // SQLITE_BUSY cuando otro proceso (p. ej. la ingesta del catalogo)
    // tiene el lock de escritura.
    let connect_options = SqliteConnectOptions::new()
        .filename(&config.database_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5))
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("base de datos lista en {}", config.database_path.display());

    // Reconocedor on-device (MobileCLIP + ONNX). None = modo degradado.
    let engine = RecognizerEngine::load(&config);

    let state = AppState {
        pool,
        engine,
        price_provider: PriceProviderKind::from_name(&config.price_provider),
        config: config.clone(),
    };

    let app = routes::api_router()
        .nest_service("/images", ServeDir::new(config.data_dir.join("images")))
        .nest_service("/scans", ServeDir::new(config.data_dir.join("scans")))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.api_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("API escuchando en http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
