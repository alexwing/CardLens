//! Cliente HTTP hacia el servicio ML interno (FastAPI).

use std::time::Duration;

use anyhow::Context;
use serde::Deserialize;

const ANALYZE_TIMEOUT: Duration = Duration::from_secs(60);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(2);

/// Cliente del servicio ML (ML_SERVICE_URL).
#[derive(Clone, Debug)]
pub struct MlClient {
    base_url: String,
    analyze_client: reqwest::Client,
    health_client: reqwest::Client,
}

impl MlClient {
    pub fn new(base_url: String) -> Self {
        let analyze_client = reqwest::Client::builder()
            .timeout(ANALYZE_TIMEOUT)
            .build()
            .expect("no se pudo construir el cliente HTTP de analisis");
        let health_client = reqwest::Client::builder()
            .timeout(HEALTH_TIMEOUT)
            .build()
            .expect("no se pudo construir el cliente HTTP de health");
        Self {
            base_url,
            analyze_client,
            health_client,
        }
    }

    /// Comprueba si el servicio ML responde (usado por GET /api/health).
    pub async fn health(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        match self.health_client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    /// Envia la imagen al servicio ML (POST /analyze, multipart campo "image").
    /// Devuelve la respuesta tipada y el cuerpo JSON crudo (para persistirlo).
    pub async fn analyze(
        &self,
        image_bytes: Vec<u8>,
        file_name: &str,
    ) -> anyhow::Result<(MlAnalyzeResponse, String)> {
        let part = reqwest::multipart::Part::bytes(image_bytes)
            .file_name(file_name.to_string())
            .mime_str("image/jpeg")
            .context("mime invalido para la imagen")?;
        let form = reqwest::multipart::Form::new().part("image", part);

        let url = format!("{}/analyze", self.base_url);
        let response = self
            .analyze_client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .with_context(|| format!("no se pudo contactar con el servicio ML en {url}"))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("no se pudo leer la respuesta del servicio ML")?;
        if !status.is_success() {
            anyhow::bail!("el servicio ML devolvio HTTP {status}: {body}");
        }

        let parsed: MlAnalyzeResponse = serde_json::from_str(&body)
            .context("respuesta del servicio ML con formato inesperado")?;
        Ok((parsed, body))
    }
}

// ---------------------------------------------------------------------------
// Structs tipados de la respuesta de POST /analyze segun el contrato.
// Campos con default para tolerar respuestas parciales sin romper.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct MlAnalyzeResponse {
    #[serde(default)]
    pub detection: Option<MlDetection>,
    #[serde(default)]
    pub ocr: Option<MlOcr>,
    #[serde(default)]
    pub candidates: Vec<MlCandidate>,
    #[serde(default)]
    pub low_confidence: bool,
    #[serde(default)]
    pub timing_ms: Option<MlTiming>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MlDetection {
    #[serde(default)]
    pub found: bool,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub quad: Option<Vec<[f64; 2]>>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MlOcr {
    #[serde(default)]
    pub lines: Vec<MlOcrLine>,
    #[serde(default)]
    pub name_guess: Option<String>,
    #[serde(default)]
    pub number_guess: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MlOcrLine {
    pub text: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MlCandidate {
    pub card_id: String,
    pub visual_score: f64,
    pub ocr_score: f64,
    pub final_score: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct MlTiming {
    #[serde(default)]
    pub detect: f64,
    #[serde(default)]
    pub ocr: f64,
    #[serde(default)]
    pub embed: f64,
    #[serde(default)]
    pub search: f64,
}
