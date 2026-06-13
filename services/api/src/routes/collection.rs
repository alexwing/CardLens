//! GET /api/collection, POST /api/collection/items, DELETE /api/collection/items/{id}

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::{
    Card, CollectionExport, CollectionExportCard, CollectionExportItem, CollectionImportRequest,
    CollectionImportSummary, CollectionItemResponse, CollectionResponse,
    CreateCollectionItemRequest, SkippedImportItem, COLLECTION_EXPORT_FORMAT,
    COLLECTION_EXPORT_VERSION,
};
use crate::AppState;

/// SELECT del JOIN collection_items + cards + sets, ordenado por fecha.
const COLLECTION_SELECT: &str =
    "SELECT ci.id, ci.quantity, ci.condition, ci.lang AS item_lang, ci.notes, ci.created_at, \
            c.id AS card_id, c.set_id, c.name AS card_name, c.number, c.rarity, c.supertype, \
            c.lang AS card_lang, c.image_url, c.image_local, s.name AS set_name \
     FROM collection_items ci \
     JOIN cards c ON c.id = ci.card_id \
     LEFT JOIN sets s ON s.id = c.set_id \
     ORDER BY ci.created_at DESC";

/// Fila plana del JOIN collection_items + cards + sets.
#[derive(Debug, FromRow)]
struct CollectionRow {
    id: String,
    quantity: i64,
    condition: Option<String>,
    item_lang: Option<String>,
    notes: Option<String>,
    created_at: String,
    card_id: String,
    set_id: String,
    card_name: String,
    number: String,
    rarity: Option<String>,
    supertype: Option<String>,
    card_lang: String,
    image_url: Option<String>,
    image_local: Option<String>,
    set_name: Option<String>,
}

impl From<CollectionRow> for CollectionItemResponse {
    fn from(row: CollectionRow) -> Self {
        CollectionItemResponse {
            id: row.id,
            card: Card {
                id: row.card_id,
                set_id: row.set_id,
                name: row.card_name,
                number: row.number,
                rarity: row.rarity,
                supertype: row.supertype,
                lang: row.card_lang,
                image_url: row.image_url,
                image_local: row.image_local,
                set_name: row.set_name,
            },
            quantity: row.quantity,
            condition: row.condition,
            lang: row.item_lang,
            notes: row.notes,
            created_at: row.created_at,
        }
    }
}

/// Lista todos los items de la coleccion con su carta embebida.
pub async fn list_items(
    State(state): State<AppState>,
) -> Result<Json<CollectionResponse>, ApiError> {
    let rows = sqlx::query_as::<_, CollectionRow>(COLLECTION_SELECT)
        .fetch_all(&state.pool)
        .await?;

    Ok(Json(CollectionResponse {
        items: rows.into_iter().map(Into::into).collect(),
    }))
}

/// Exporta la coleccion completa como documento JSON portable y versionado
/// (GET /api/collection/export). El mismo documento se puede reimportar.
pub async fn export_collection(
    State(state): State<AppState>,
) -> Result<Json<CollectionExport>, ApiError> {
    let rows = sqlx::query_as::<_, CollectionRow>(COLLECTION_SELECT)
        .fetch_all(&state.pool)
        .await?;

    let items: Vec<CollectionExportItem> = rows
        .into_iter()
        .map(|row| CollectionExportItem {
            card_id: row.card_id,
            quantity: row.quantity,
            condition: row.condition,
            lang: row.item_lang,
            notes: row.notes,
            created_at: row.created_at,
            card: CollectionExportCard {
                name: row.card_name,
                set_name: row.set_name,
                number: row.number,
            },
        })
        .collect();

    Ok(Json(CollectionExport {
        format: COLLECTION_EXPORT_FORMAT.to_string(),
        version: COLLECTION_EXPORT_VERSION,
        exported_at: Utc::now().to_rfc3339(),
        count: items.len(),
        items,
    }))
}

/// Importa items de coleccion desde un documento JSON (POST /api/collection/import).
///
/// `mode = "merge"` (por defecto): por cada item, si ya existe uno con el mismo
/// card_id actualiza cantidad/estado/idioma/notas; si no, lo inserta. Reimportar
/// el mismo fichero es idempotente. `mode = "replace"`: vacia la coleccion y la
/// rellena con el documento. Las cartas que no existen en el catalogo local se
/// omiten (no se puede satisfacer la clave foranea) y se listan en el resumen.
pub async fn import_collection(
    State(state): State<AppState>,
    Json(body): Json<CollectionImportRequest>,
) -> Result<Json<CollectionImportSummary>, ApiError> {
    let mode = body.mode.unwrap_or_else(|| "merge".to_string());
    if mode != "merge" && mode != "replace" {
        return Err(ApiError::BadRequest(format!(
            "mode invalido: '{mode}' (usa 'merge' o 'replace')"
        )));
    }

    let mut tx = state.pool.begin().await?;

    if mode == "replace" {
        sqlx::query("DELETE FROM collection_items")
            .execute(&mut *tx)
            .await?;
    }

    let mut imported = 0usize;
    let mut updated = 0usize;
    let mut skipped: Vec<SkippedImportItem> = Vec::new();

    for item in body.items {
        if item.quantity < 1 {
            skipped.push(SkippedImportItem {
                card_id: item.card_id,
                reason: "quantity debe ser mayor o igual que 1".to_string(),
            });
            continue;
        }

        let card_exists: Option<String> = sqlx::query_scalar("SELECT id FROM cards WHERE id = ?1")
            .bind(&item.card_id)
            .fetch_optional(&mut *tx)
            .await?;
        if card_exists.is_none() {
            skipped.push(SkippedImportItem {
                card_id: item.card_id,
                reason: "carta no encontrada en el catalogo local".to_string(),
            });
            continue;
        }

        // En modo merge, reutiliza el item existente para ese card_id.
        if mode == "merge" {
            let existing_id: Option<String> =
                sqlx::query_scalar("SELECT id FROM collection_items WHERE card_id = ?1 LIMIT 1")
                    .bind(&item.card_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if let Some(existing_id) = existing_id {
                sqlx::query(
                    "UPDATE collection_items \
                     SET quantity = ?1, condition = ?2, lang = ?3, notes = ?4 \
                     WHERE id = ?5",
                )
                .bind(item.quantity)
                .bind(&item.condition)
                .bind(&item.lang)
                .bind(&item.notes)
                .bind(&existing_id)
                .execute(&mut *tx)
                .await?;
                updated += 1;
                continue;
            }
        }

        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO collection_items \
                (id, user_id, card_id, scan_id, quantity, condition, lang, notes, created_at) \
             VALUES (?1, NULL, ?2, NULL, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(&id)
        .bind(&item.card_id)
        .bind(item.quantity)
        .bind(&item.condition)
        .bind(&item.lang)
        .bind(&item.notes)
        .bind(&created_at)
        .execute(&mut *tx)
        .await?;
        imported += 1;
    }

    tx.commit().await?;

    Ok(Json(CollectionImportSummary {
        mode,
        imported,
        updated,
        skipped,
    }))
}

/// Crea un item de coleccion. Responde 201 con el item creado.
pub async fn create_item(
    State(state): State<AppState>,
    Json(body): Json<CreateCollectionItemRequest>,
) -> Result<(StatusCode, Json<CollectionItemResponse>), ApiError> {
    if body.quantity < 1 {
        return Err(ApiError::BadRequest(
            "quantity debe ser mayor o igual que 1".to_string(),
        ));
    }

    let card = Card::fetch_by_id(&state.pool, &body.card_id)
        .await?
        .ok_or_else(|| {
            ApiError::BadRequest(format!("card_id no existe en el catalogo: {}", body.card_id))
        })?;

    if let Some(scan_id) = &body.scan_id {
        let exists: Option<String> = sqlx::query_scalar("SELECT id FROM scans WHERE id = ?1")
            .bind(scan_id)
            .fetch_optional(&state.pool)
            .await?;
        if exists.is_none() {
            return Err(ApiError::BadRequest(format!(
                "scan_id no existe: {scan_id}"
            )));
        }
    }

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO collection_items (id, user_id, card_id, scan_id, quantity, condition, lang, notes, created_at) \
         VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(&id)
    .bind(&body.card_id)
    .bind(&body.scan_id)
    .bind(body.quantity)
    .bind(&body.condition)
    .bind(&body.lang)
    .bind(&body.notes)
    .bind(&created_at)
    .execute(&state.pool)
    .await?;

    let item = CollectionItemResponse {
        id,
        card,
        quantity: body.quantity,
        condition: body.condition,
        lang: body.lang,
        notes: body.notes,
        created_at,
    };
    Ok((StatusCode::CREATED, Json(item)))
}

/// Elimina un item de coleccion. Responde 204; 404 si no existe.
pub async fn delete_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM collection_items WHERE id = ?1")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "item de coleccion no encontrado: {id}"
        )));
    }
    Ok(StatusCode::NO_CONTENT)
}
