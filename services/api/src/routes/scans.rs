//! GET /api/scans (historial de escaneos, paginado)

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::error::ApiError;
use crate::models::{Paginated, ScanSummary};
use crate::AppState;

const PAGE_SIZE: i64 = 20;

#[derive(Debug, Deserialize)]
pub struct ScansQuery {
    pub page: Option<i64>,
}

pub async fn list_scans(
    State(state): State<AppState>,
    Query(query): Query<ScansQuery>,
) -> Result<Json<Paginated<ScanSummary>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * PAGE_SIZE;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM scans")
        .fetch_one(&state.pool)
        .await?;

    let items = sqlx::query_as::<_, ScanSummary>(
        "SELECT id, created_at, best_card_id, confidence, low_confidence \
         FROM scans ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
    )
    .bind(PAGE_SIZE)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(Paginated { items, total, page }))
}
