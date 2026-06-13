//! GET /api/sets

use axum::extract::State;
use axum::Json;

use crate::error::ApiError;
use crate::models::SetInfo;
use crate::AppState;

/// Listado completo de sets del catalogo.
pub async fn list_sets(State(state): State<AppState>) -> Result<Json<Vec<SetInfo>>, ApiError> {
    let sets = sqlx::query_as::<_, SetInfo>(
        "SELECT id, name, series, code, release_date, total, lang \
         FROM sets ORDER BY release_date, id",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(sets))
}
