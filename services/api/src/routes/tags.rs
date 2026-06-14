//! GET /api/tags, POST /api/tags, DELETE /api/tags/{id}
//!
//! Etiquetas reutilizables para clasificar los items de la coleccion.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use sqlx::SqliteConnection;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::{TagRef, TagWithCount};
use crate::AppState;

/// Body de POST /api/tags y de POST /api/collection/items/{id}/tags.
#[derive(Debug, Deserialize)]
pub struct TagNameRequest {
    pub name: String,
}

/// Crea la tag con ese nombre o devuelve la existente (match case-insensitive).
///
/// Trabaja sobre una conexion concreta para poder usarse tanto desde el pool
/// como dentro de una transaccion (importacion). Es idempotente. Devuelve
/// `BadRequest` si el nombre queda vacio tras recortar espacios.
pub async fn upsert_tag_by_name(
    conn: &mut SqliteConnection,
    raw_name: &str,
) -> Result<TagRef, ApiError> {
    let name = raw_name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest(
            "el nombre de la etiqueta no puede estar vacio".to_string(),
        ));
    }

    // Reutiliza la tag existente (comparacion case-insensitive por el indice).
    let existing: Option<TagRef> =
        sqlx::query_as::<_, TagRef>("SELECT id, name FROM tags WHERE name = ?1 COLLATE NOCASE")
            .bind(name)
            .fetch_optional(&mut *conn)
            .await?;
    if let Some(tag) = existing {
        return Ok(tag);
    }

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO tags (id, name, created_at) VALUES (?1, ?2, ?3)")
        .bind(&id)
        .bind(name)
        .bind(&created_at)
        .execute(&mut *conn)
        .await?;

    Ok(TagRef {
        id,
        name: name.to_string(),
    })
}

/// Variante para callers que solo tienen el pool: adquiere una conexion.
pub async fn upsert_tag_by_name_pool(pool: &SqlitePool, raw_name: &str) -> Result<TagRef, ApiError> {
    let mut conn = pool.acquire().await?;
    upsert_tag_by_name(&mut conn, raw_name).await
}

/// Lista todas las etiquetas con el numero de items que las usan, por nombre.
pub async fn list_tags(State(state): State<AppState>) -> Result<Json<Vec<TagWithCount>>, ApiError> {
    let tags = sqlx::query_as::<_, TagWithCount>(
        "SELECT t.id, t.name, \
                (SELECT COUNT(*) FROM collection_item_tags cit WHERE cit.tag_id = t.id) AS count \
         FROM tags t \
         ORDER BY t.name COLLATE NOCASE",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(tags))
}

/// Crea una etiqueta (o devuelve la existente, case-insensitive). 201 {id,name}.
pub async fn create_tag(
    State(state): State<AppState>,
    Json(body): Json<TagNameRequest>,
) -> Result<(StatusCode, Json<TagRef>), ApiError> {
    let tag = upsert_tag_by_name_pool(&state.pool, &body.name).await?;
    Ok((StatusCode::CREATED, Json(tag)))
}

/// Elimina una etiqueta. Las asociaciones caen por ON DELETE CASCADE. 204; 404
/// si no existe.
pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM tags WHERE id = ?1")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("etiqueta no encontrada: {id}")));
    }
    Ok(StatusCode::NO_CONTENT)
}
