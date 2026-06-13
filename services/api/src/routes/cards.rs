//! GET /api/cards y GET /api/cards/{id}

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::error::ApiError;
use crate::models::{Card, Paginated, CARD_SELECT};
use crate::AppState;

const DEFAULT_PAGE_SIZE: i64 = 20;
const MAX_PAGE_SIZE: i64 = 100;

#[derive(Debug, Deserialize)]
pub struct CardsQuery {
    pub q: Option<String>,
    pub set_id: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// Listado paginado de cartas; `q` filtra por nombre con LIKE, `set_id` por set.
pub async fn list_cards(
    State(state): State<AppState>,
    Query(query): Query<CardsQuery>,
) -> Result<Json<Paginated<Card>>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = (page - 1) * page_size;

    let q = query
        .q
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let set_id = query
        .set_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cards c \
         WHERE (?1 IS NULL OR c.name LIKE '%' || ?1 || '%') \
           AND (?2 IS NULL OR c.set_id = ?2)",
    )
    .bind(&q)
    .bind(&set_id)
    .fetch_one(&state.pool)
    .await?;

    let sql = format!(
        "{CARD_SELECT} \
         WHERE (?1 IS NULL OR c.name LIKE '%' || ?1 || '%') \
           AND (?2 IS NULL OR c.set_id = ?2) \
         ORDER BY c.set_id, c.id \
         LIMIT ?3 OFFSET ?4"
    );
    let items = sqlx::query_as::<_, Card>(&sql)
        .bind(&q)
        .bind(&set_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.pool)
        .await?;

    Ok(Json(Paginated { items, total, page }))
}

/// Detalle de una carta por id.
pub async fn get_card(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Card>, ApiError> {
    let card = Card::fetch_by_id(&state.pool, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("carta no encontrada: {id}")))?;
    Ok(Json(card))
}
