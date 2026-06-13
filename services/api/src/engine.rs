//! Motor de reconocimiento on-device embebido en la API (sin Python).
//!
//! Carga el encoder visual (MobileCLIP via ONNX Runtime) y el indice de
//! embeddings, y resuelve cada escaneo en proceso: embedding + busqueda coseno.
//! De momento es solo visual (sin OCR ni recorte); el OCR se portara despues.
//! Si falta el modelo/indice o el runtime ONNX no esta disponible, la API
//! arranca igual y queda en modo degradado (se carga como `None`).

use std::sync::{Arc, Mutex};

use recognizer::{Embedder, FlatIndex, PreprocessConfig};

use crate::config::Config;

/// Candidato producido por el motor (solo visual: ocr_score=0, final=visual).
pub struct EngineCandidate {
    pub card_id: String,
    pub visual_score: f64,
    pub ocr_score: f64,
    pub final_score: f64,
}

/// Resultado de analizar una imagen.
pub struct EngineResult {
    pub candidates: Vec<EngineCandidate>,
    pub low_confidence: bool,
}

/// Motor de reconocimiento: encoder + indice + umbrales. Clonable (Arc).
#[derive(Clone)]
pub struct RecognizerEngine {
    embedder: Arc<Mutex<Embedder>>,
    index: Arc<FlatIndex>,
    top_k: usize,
    conf_threshold: f64,
    margin_threshold: f64,
}

impl RecognizerEngine {
    /// Carga modelo + indice. Devuelve `None` (degradado) si faltan ficheros o
    /// el runtime ONNX no esta disponible.
    pub fn load(config: &Config) -> Option<Self> {
        let model = &config.model_path;
        let bin = &config.index_bin_path;
        let cards = &config.index_cards_path;
        if !model.exists() || !bin.exists() || !cards.exists() {
            tracing::warn!(
                "reconocedor en modo degradado: falta modelo o indice (model={}, bin={}, cards={})",
                model.display(),
                bin.display(),
                cards.display()
            );
            return None;
        }

        let index = match FlatIndex::load(bin, cards) {
            Ok(index) => index,
            Err(err) => {
                tracing::warn!("no se pudo cargar el indice del reconocedor: {err}");
                return None;
            }
        };

        let model_bytes = match std::fs::read(model) {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::warn!("no se pudo leer el modelo ONNX: {err}");
                return None;
            }
        };

        let embedder = match Embedder::from_bytes(&model_bytes, PreprocessConfig::mobileclip2_s0()) {
            Ok(embedder) => embedder,
            Err(err) => {
                tracing::warn!(
                    "no se pudo inicializar el modelo ONNX (¿ORT_DYLIB_PATH?): {err}"
                );
                return None;
            }
        };

        tracing::info!(
            "reconocedor cargado: {} cartas en el indice (solo visual)",
            index.len()
        );
        Some(Self {
            embedder: Arc::new(Mutex::new(embedder)),
            index: Arc::new(index),
            top_k: config.top_k,
            conf_threshold: config.conf_threshold,
            margin_threshold: config.margin_threshold,
        })
    }

    /// Numero de cartas en el indice cargado.
    pub fn index_size(&self) -> usize {
        self.index.len()
    }

    /// Embebe la imagen y busca las cartas mas parecidas (solo similitud
    /// visual). La inferencia (CPU) corre en un hilo de bloqueo para no
    /// bloquear el runtime async.
    pub async fn analyze(&self, image_bytes: Vec<u8>) -> anyhow::Result<EngineResult> {
        let embedder = self.embedder.clone();
        let index = self.index.clone();
        let top_k = self.top_k;
        let conf = self.conf_threshold;
        let margin = self.margin_threshold;

        tokio::task::spawn_blocking(move || -> anyhow::Result<EngineResult> {
            let embedding = {
                let mut guard = embedder
                    .lock()
                    .map_err(|_| anyhow::anyhow!("mutex del embedder envenenado"))?;
                guard.embed_bytes_detected(&image_bytes)?
            };
            let matches = index.search(&embedding, top_k)?;

            let candidates: Vec<EngineCandidate> = matches
                .iter()
                .map(|m| {
                    let score = m.score.clamp(0.0, 1.0) as f64;
                    EngineCandidate {
                        card_id: m.card.card_id.clone(),
                        visual_score: score,
                        ocr_score: 0.0,
                        final_score: score,
                    }
                })
                .collect();

            let low_confidence = match (candidates.first(), candidates.get(1)) {
                (None, _) => true,
                (Some(top), Some(second)) => {
                    top.final_score < conf || (top.final_score - second.final_score) < margin
                }
                (Some(top), None) => top.final_score < conf,
            };

            Ok(EngineResult {
                candidates,
                low_confidence,
            })
        })
        .await
        .map_err(|err| anyhow::anyhow!("tarea de inferencia cancelada: {err}"))?
    }
}
