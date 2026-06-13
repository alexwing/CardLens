//! POST /api/scan: guarda la foto, la reconoce on-device (Rust), enriquece
//! los candidatos con el catalogo y persiste el resultado.

use anyhow::Context;
use axum::extract::{Multipart, State};
use axum::Json;
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::engine::EngineCandidate;
use crate::error::ApiError;
use crate::models::{BestMatch, CandidateResponse, Card, ScanResponse};
use crate::AppState;

pub async fn create_scan(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<ScanResponse>, ApiError> {
    // 1. Extraer el campo multipart "image".
    let mut image_bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::BadRequest(format!("multipart invalido: {error}")))?
    {
        if field.name() == Some("image") {
            let data = field
                .bytes()
                .await
                .map_err(|error| ApiError::BadRequest(format!("no se pudo leer la imagen: {error}")))?;
            image_bytes = Some(data.to_vec());
        }
    }
    let image_bytes = image_bytes
        .ok_or_else(|| ApiError::BadRequest("falta el campo multipart 'image'".to_string()))?;
    if image_bytes.is_empty() {
        return Err(ApiError::BadRequest("la imagen esta vacia".to_string()));
    }

    // 2. Guardar la imagen en data/scans/{scan_id}.jpg.
    let scan_id = Uuid::new_v4().to_string();
    let file_name = format!("{scan_id}.jpg");
    let absolute_path = state.config.data_dir.join("scans").join(&file_name);
    tokio::fs::write(&absolute_path, &image_bytes)
        .await
        .with_context(|| format!("no se pudo guardar la imagen en {}", absolute_path.display()))?;
    let image_path = format!("data/scans/{file_name}");
    let created_at = Utc::now().to_rfc3339();

    // 3. Reconocimiento on-device. Sin motor (modo degradado) -> respuesta vacia.
    let engine = match &state.engine {
        Some(engine) => engine,
        None => {
            sqlx::query(
                "INSERT INTO scans (id, created_at, image_path, status, low_confidence) \
                 VALUES (?1, ?2, ?3, 'done', 1)",
            )
            .bind(&scan_id)
            .bind(&created_at)
            .bind(&image_path)
            .execute(&state.pool)
            .await?;
            return Ok(Json(ScanResponse {
                scan_id,
                low_confidence: true,
                best: None,
                candidates: Vec::new(),
            }));
        }
    };

    let analysis = match engine.analyze(image_bytes).await {
        Ok(analysis) => analysis,
        Err(error) => {
            // Persistencia best-effort del intento fallido y 500 al cliente.
            let _ = sqlx::query(
                "INSERT INTO scans (id, created_at, image_path, status, low_confidence) \
                 VALUES (?1, ?2, ?3, 'error', 1)",
            )
            .bind(&scan_id)
            .bind(&created_at)
            .bind(&image_path)
            .execute(&state.pool)
            .await;
            return Err(ApiError::Internal(error.context("fallo de reconocimiento")));
        }
    };

    // 4. Enriquecer cada candidato con metadata del catalogo (cards JOIN sets).
    let mut enriched: Vec<(Card, &EngineCandidate)> = Vec::with_capacity(analysis.candidates.len());
    for candidate in &analysis.candidates {
        match Card::fetch_by_id(&state.pool, &candidate.card_id).await? {
            Some(card) => enriched.push((card, candidate)),
            None => {
                tracing::warn!(card_id = %candidate.card_id, "candidato sin carta en catalogo, se omite");
            }
        }
    }

    let best = enriched.first().map(|(card, candidate)| BestMatch {
        card: card.clone(),
        confidence: candidate.final_score,
    });

    // raw_json para auditoria del escaneo.
    let raw_json = serde_json::to_string(&json!({
        "engine": "mobileclip-visual",
        "low_confidence": analysis.low_confidence,
        "candidates": analysis.candidates.iter().map(|c| json!({
            "card_id": c.card_id,
            "visual_score": c.visual_score,
            "ocr_score": c.ocr_score,
            "final_score": c.final_score,
        })).collect::<Vec<_>>(),
    }))
    .ok();

    // 5. Persistir scans y scan_candidates en una transaccion.
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO scans (id, created_at, image_path, status, best_card_id, confidence, low_confidence, raw_json) \
         VALUES (?1, ?2, ?3, 'done', ?4, ?5, ?6, ?7)",
    )
    .bind(&scan_id)
    .bind(&created_at)
    .bind(&image_path)
    .bind(best.as_ref().map(|b| b.card.id.clone()))
    .bind(best.as_ref().map(|b| b.confidence))
    .bind(analysis.low_confidence)
    .bind(&raw_json)
    .execute(&mut *tx)
    .await?;

    for (index, (card, candidate)) in enriched.iter().enumerate() {
        sqlx::query(
            "INSERT INTO scan_candidates (scan_id, card_id, \"rank\", visual_score, ocr_score, final_score) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(&scan_id)
        .bind(&card.id)
        .bind((index + 1) as i64)
        .bind(candidate.visual_score)
        .bind(candidate.ocr_score)
        .bind(candidate.final_score)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    // 6. Respuesta segun contrato.
    let candidates = enriched
        .into_iter()
        .map(|(card, candidate)| CandidateResponse {
            card,
            confidence: candidate.final_score,
            visual_score: candidate.visual_score,
            ocr_score: candidate.ocr_score,
        })
        .collect();

    Ok(Json(ScanResponse {
        scan_id,
        low_confidence: analysis.low_confidence,
        best,
        candidates,
    }))
}
