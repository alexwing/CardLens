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
    /// Proveedor de precios (env `PRICE_PROVIDER`, `null` o `tcgdex`, default `null`).
    pub price_provider: String,
    /// Modelo ONNX del encoder visual (env `MODEL_PATH`).
    pub model_path: PathBuf,
    /// Indice de embeddings (.bin) del reconocedor (env `INDEX_BIN_PATH`).
    pub index_bin_path: PathBuf,
    /// Metadatos del indice (.json) (env `INDEX_CARDS_PATH`).
    pub index_cards_path: PathBuf,
    /// Numero de candidatos a devolver (env `TOP_K`, default 5).
    pub top_k: usize,
    /// Umbral de confianza para marcar low_confidence (env `CONF_THRESHOLD`, default 0.80).
    pub conf_threshold: f64,
    /// Margen minimo top1-top2 (env `MARGIN_THRESHOLD`, default 0.05).
    pub margin_threshold: f64,
    /// Candidatos que se recuperan del indice antes de re-rankear con OCR
    /// (env `SEARCH_K`, default 30).
    pub search_k: usize,
    /// Peso del refuerzo OCR (bonus aditivo sobre el score visual)
    /// (env `W_OCR`, default 0.35).
    pub w_ocr: f64,
    /// Modelo OCR de deteccion de texto (env `OCR_DET_PATH`).
    pub ocr_det_path: PathBuf,
    /// Modelo OCR de reconocimiento de texto (env `OCR_REC_PATH`).
    pub ocr_rec_path: PathBuf,
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

        let price_provider = env::var("PRICE_PROVIDER").unwrap_or_else(|_| "null".to_string());

        let index_dir = data_dir.join("index");
        let model_path = match env::var("MODEL_PATH") {
            Ok(value) => resolve_from_repo_root(&repo_root, &value),
            Err(_) => repo_root.join("models/mobileclip2_s0/vision_model.onnx"),
        };
        let index_bin_path = match env::var("INDEX_BIN_PATH") {
            Ok(value) => resolve_from_repo_root(&repo_root, &value),
            Err(_) => index_dir.join("mobileclip.bin"),
        };
        let index_cards_path = match env::var("INDEX_CARDS_PATH") {
            Ok(value) => resolve_from_repo_root(&repo_root, &value),
            Err(_) => index_dir.join("mobileclip_cards.json"),
        };
        let top_k = env::var("TOP_K")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5usize);
        let conf_threshold = env::var("CONF_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.80f64);
        let margin_threshold = env::var("MARGIN_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.05f64);
        let search_k = env::var("SEARCH_K")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30usize);
        let w_ocr = env::var("W_OCR")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.35f64);
        let ocr_dir = repo_root.join("models/ocrs");
        let ocr_det_path = match env::var("OCR_DET_PATH") {
            Ok(value) => resolve_from_repo_root(&repo_root, &value),
            Err(_) => ocr_dir.join("text-detection.rten"),
        };
        let ocr_rec_path = match env::var("OCR_REC_PATH") {
            Ok(value) => resolve_from_repo_root(&repo_root, &value),
            Err(_) => ocr_dir.join("text-recognition.rten"),
        };

        Ok(Self {
            api_port,
            database_path,
            data_dir,
            price_provider,
            model_path,
            index_bin_path,
            index_cards_path,
            top_k,
            conf_threshold,
            margin_threshold,
            search_k,
            w_ocr,
            ocr_det_path,
            ocr_rec_path,
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
