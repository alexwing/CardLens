//! Libreria de la API de negocio (axum + sqlx + SQLite + reconocedor).
//!
//! Permite arrancar el servidor de dos formas:
//! - como **binario** independiente (desarrollo y *sidecar* de escritorio), via
//!   `main.rs`, que solo lee la configuracion del entorno y llama a [`serve`];
//! - **embebido en proceso** dentro de la app Tauri (escritorio/Android, que no
//!   puede lanzar un sidecar), construyendo una [`Config`] a mano y llamando a
//!   [`serve`] sobre un runtime tokio propio.

pub mod config;
pub mod engine;
pub mod error;
pub mod models;
pub mod providers;
pub mod routes;

use std::net::SocketAddr;

use axum::extract::DefaultBodyLimit;
use axum::Router;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::engine::RecognizerEngine;
use crate::providers::PriceProviderKind;

/// Tamano maximo de subida (fotos de cartas).
pub const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;

/// Estado compartido por todos los handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Config,
    pub engine: Option<RecognizerEngine>,
    pub price_provider: PriceProviderKind,
}

/// Crea los directorios de datos, abre/crea la base SQLite con WAL, ejecuta las
/// migraciones, carga el reconocedor on-device y devuelve el estado compartido.
pub async fn build_state(config: Config) -> anyhow::Result<AppState> {
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

    Ok(AppState {
        pool,
        engine,
        price_provider: PriceProviderKind::from_name(&config.price_provider),
        config,
    })
}

/// Construye el `Router` de axum completo (rutas /api + estaticos + capas).
pub fn build_router(state: AppState) -> Router {
    let data_dir = state.config.data_dir.clone();
    routes::api_router()
        .nest_service("/images", ServeDir::new(data_dir.join("images")))
        .nest_service("/scans", ServeDir::new(data_dir.join("scans")))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
}

/// Arranca el servidor HTTP en `0.0.0.0:config.api_port` y sirve hasta que el
/// proceso (o la tarea tokio que lo aloja) termine.
pub async fn serve(config: Config) -> anyhow::Result<()> {
    let port = config.api_port;
    let state = build_state(config).await?;
    let app = build_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("API escuchando en http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
