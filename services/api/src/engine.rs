//! Motor de reconocimiento on-device embebido en la API (sin Python).
//!
//! Pipeline en proceso: deteccion+recorte de la carta -> embedding visual
//! (MobileCLIP via ONNX) + OCR (ocrs) -> busqueda coseno -> fusion (el OCR
//! refuerza por nombre/numero) -> candidatos. Si falta el modelo/indice o el
//! runtime no esta disponible, la API arranca igual en modo degradado.

use std::sync::{Arc, Mutex};

use recognizer::ocr::OcrReader;
use recognizer::{Embedder, FlatIndex, PreprocessConfig};

use crate::config::Config;

/// Candidato producido por el motor.
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

/// Motor de reconocimiento: encoder + indice + OCR + umbrales. Clonable (Arc).
#[derive(Clone)]
pub struct RecognizerEngine {
    embedder: Arc<Mutex<Embedder>>,
    ocr: Option<Arc<Mutex<OcrReader>>>,
    index: Arc<FlatIndex>,
    top_k: usize,
    search_k: usize,
    w_ocr: f64,
    conf_threshold: f64,
    margin_threshold: f64,
}

impl RecognizerEngine {
    /// Carga modelo + indice (+ OCR opcional). Devuelve `None` (degradado) si
    /// faltan el modelo/indice o el runtime ONNX no esta disponible. El OCR es
    /// opcional: si falta, el reconocimiento usa solo la parte visual.
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
                tracing::warn!("no se pudo inicializar el modelo ONNX (¿ORT_DYLIB_PATH?): {err}");
                return None;
            }
        };

        let ocr = if config.ocr_det_path.exists() && config.ocr_rec_path.exists() {
            match OcrReader::from_files(&config.ocr_det_path, &config.ocr_rec_path) {
                Ok(reader) => {
                    tracing::info!("OCR cargado");
                    Some(Arc::new(Mutex::new(reader)))
                }
                Err(err) => {
                    tracing::warn!("OCR no disponible (solo visual): {err}");
                    None
                }
            }
        } else {
            tracing::warn!("OCR en modo degradado: faltan los modelos rten");
            None
        };

        tracing::info!(
            "reconocedor cargado: {} cartas, OCR={}",
            index.len(),
            ocr.is_some()
        );
        Some(Self {
            embedder: Arc::new(Mutex::new(embedder)),
            ocr,
            index: Arc::new(index),
            top_k: config.top_k,
            search_k: config.search_k.max(config.top_k),
            w_ocr: config.w_ocr,
            conf_threshold: config.conf_threshold,
            margin_threshold: config.margin_threshold,
        })
    }

    /// Numero de cartas en el indice cargado.
    pub fn index_size(&self) -> usize {
        self.index.len()
    }

    /// Detecta+recorta la carta, la embebe y la lee por OCR, recupera los
    /// candidatos mas parecidos y los re-rankea reforzando con el OCR.
    pub async fn analyze(&self, image_bytes: Vec<u8>) -> anyhow::Result<EngineResult> {
        let embedder = self.embedder.clone();
        let ocr = self.ocr.clone();
        let index = self.index.clone();
        let top_k = self.top_k;
        let search_k = self.search_k;
        let w_ocr = self.w_ocr;
        let conf = self.conf_threshold;
        let margin = self.margin_threshold;

        tokio::task::spawn_blocking(move || -> anyhow::Result<EngineResult> {
            // Carta recortada (misma vista para embedding y OCR).
            let card = recognizer::prepare_card(&image_bytes)?;

            let embedding = {
                let mut guard = embedder
                    .lock()
                    .map_err(|_| anyhow::anyhow!("mutex del embedder envenenado"))?;
                guard.embed(&card)?
            };

            let ocr_text = match &ocr {
                Some(reader) => reader
                    .lock()
                    .map_err(|_| anyhow::anyhow!("mutex del OCR envenenado"))?
                    .read_name_region(&card)
                    .unwrap_or_default(),
                None => String::new(),
            };

            let matches = index.search(&embedding, search_k)?;

            let ocr_norm = normalize(&ocr_text);
            let ocr_tokens: Vec<&str> = ocr_norm.split_whitespace().collect();
            let number_guess = extract_number(&ocr_text).map(|n| normalize_number(&n));
            let ocr_usable = ocr_norm.chars().filter(|c| c.is_ascii_alphabetic()).count() >= 3;

            let mut candidates: Vec<EngineCandidate> = matches
                .iter()
                .map(|m| {
                    let visual = m.score.clamp(0.0, 1.0) as f64;
                    let ocr_score = if ocr_usable {
                        let name = name_score(&m.card.name, &ocr_norm, &ocr_tokens);
                        let number_bonus = match &number_guess {
                            Some(guess)
                                if !guess.is_empty()
                                    && *guess == normalize_number(&m.card.number) =>
                            {
                                0.3
                            }
                            _ => 0.0,
                        };
                        (name + number_bonus).min(1.0)
                    } else {
                        0.0
                    };
                    // Refuerzo aditivo: el OCR sube la carta cuyo nombre lee,
                    // sin penalizar al resto cuando el OCR no aporta.
                    let final_score = (visual + w_ocr * ocr_score).min(1.0);
                    EngineCandidate {
                        card_id: m.card.card_id.clone(),
                        visual_score: visual,
                        ocr_score,
                        final_score,
                    }
                })
                .collect();

            candidates.sort_by(|a, b| {
                b.final_score
                    .partial_cmp(&a.final_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            candidates.truncate(top_k);

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

/// Normaliza texto para comparar: minusculas, solo [a-z0-9], resto -> espacio.
fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = true;
    for ch in text.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_space = false;
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }
    out.trim().to_string()
}

/// Puntuacion de nombre: fraccion de palabras del nombre (>=3 letras) que
/// aparecen en el texto OCR (por substring o por similitud difusa).
fn name_score(name: &str, ocr_norm: &str, ocr_tokens: &[&str]) -> f64 {
    let name_norm = normalize(name);
    let tokens: Vec<&str> = name_norm
        .split_whitespace()
        .filter(|t| t.len() >= 3)
        .collect();
    if tokens.is_empty() {
        return 0.0;
    }
    let mut matched = 0usize;
    for token in &tokens {
        let found = ocr_norm.contains(token)
            || ocr_tokens
                .iter()
                .any(|other| strsim::jaro_winkler(token, other) >= 0.88);
        if found {
            matched += 1;
        }
    }
    matched as f64 / tokens.len() as f64
}

/// Extrae el numero de carta (numerador del patron N/N) del texto OCR.
fn extract_number(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        if let Some((left, right)) = token.split_once('/') {
            let left_digits: String = left.chars().filter(|c| c.is_ascii_digit()).collect();
            let right_digits: String = right.chars().filter(|c| c.is_ascii_digit()).collect();
            if !left_digits.is_empty()
                && !right_digits.is_empty()
                && left_digits.len() <= 3
                && right_digits.len() <= 4
            {
                return Some(left_digits);
            }
        }
    }
    None
}

/// Normaliza un numero de carta para comparar: solo digitos sin ceros a la izquierda.
fn normalize_number(value: &str) -> String {
    let digits: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.trim_start_matches('0').to_string()
}
