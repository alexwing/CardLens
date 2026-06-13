//! Configuracion de la API leida desde variables de entorno.

use std::env;
use std::path::{Component, Path, PathBuf};

/// Configuracion global del servicio.
#[derive(Clone, Debug)]
pub struct Config {
    /// Puerto HTTP de la API (env `API_PORT`, default 8787).
    pub api_port: u16,
    /// Ruta absoluta del fichero SQLite (env `DATABASE_PATH`, default `<raiz del repo>/data/app.db`).
    pub database_path: PathBuf,
    /// Directorio absoluto de datos (env `DATA_DIR`, default `<raiz del repo>/data`).
    pub data_dir: PathBuf,
    /// URL base del servicio ML (env `ML_SERVICE_URL`, default `http://127.0.0.1:8001`).
    pub ml_service_url: String,
    /// Proveedor de precios (env `PRICE_PROVIDER`, `null` o `tcgdex`, default `null`).
    pub price_provider: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let api_port: u16 = match env::var("API_PORT") {
            Ok(value) => value
                .parse()
                .map_err(|_| anyhow::anyhow!("API_PORT invalido: {value}"))?,
            Err(_) => 8787,
        };

        // Raiz del repo: <manifest>/../.. (services/api -> raiz). Las rutas
        // relativas de DATA_DIR/DATABASE_PATH se resuelven SIEMPRE contra la
        // raiz del repo, no contra el cwd: asi `DATA_DIR=./data` del .env de
        // la raiz apunta a <repo>/data aunque la API se arranque con
        // `cd services/api; cargo run`, igual que la ingesta Python.
        // (En despliegues fuera del repo, p. ej. Docker, usa rutas absolutas.)
        let repo_root = normalize(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        let default_data_dir = repo_root.join("data");

        let data_dir = match env::var("DATA_DIR") {
            Ok(value) => resolve_from_repo_root(&repo_root, &value),
            Err(_) => default_data_dir.clone(),
        };

        let database_path = match env::var("DATABASE_PATH") {
            Ok(value) => resolve_from_repo_root(&repo_root, &value),
            Err(_) => default_data_dir.join("app.db"),
        };

        let ml_service_url = env::var("ML_SERVICE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8001".to_string())
            .trim_end_matches('/')
            .to_string();

        let price_provider = env::var("PRICE_PROVIDER").unwrap_or_else(|_| "null".to_string());

        Ok(Self {
            api_port,
            database_path,
            data_dir,
            ml_service_url,
            price_provider,
        })
    }
}

/// Convierte una ruta (posiblemente relativa) en absoluta respecto a la raiz
/// del repo. Las rutas absolutas se respetan tal cual.
fn resolve_from_repo_root(repo_root: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    let absolute = if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    };
    normalize(&absolute)
}

/// Normalizacion lexica: elimina `.` y resuelve `..` sin tocar el sistema de ficheros.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}
