//! Punto de entrada del **binario** de la API (desarrollo y *sidecar* de
//! escritorio). Solo lee la configuracion del entorno y delega en la libreria
//! [`pokemon_card_api::serve`]. La logica del servidor vive en `lib.rs` para
//! poder embeberla tambien en proceso (app Tauri en Android, sin sidecar).

use pokemon_card_api::config::Config;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;
    pokemon_card_api::serve(config).await
}
