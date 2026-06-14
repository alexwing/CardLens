//! GET /api/collection, POST /api/collection/items, DELETE /api/collection/items/{id}

use std::collections::HashMap;

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
    CreateCollectionItemRequest, SkippedImportItem, TagRef, UpdateCollectionItemRequest,
    COLLECTION_EXPORT_FORMAT, COLLECTION_EXPORT_VERSION,
};
use crate::routes::tags::{upsert_tag_by_name, upsert_tag_by_name_pool, TagNameRequest};
use crate::AppState;

/// SELECT del JOIN collection_items + cards + sets. Sin ORDER BY/WHERE para
/// poder reutilizarlo tanto en el listado (append ORDER BY) como en la consulta
/// de un solo item (append WHERE).
const COLLECTION_SELECT: &str =
    "SELECT ci.id, ci.quantity, ci.condition, ci.lang AS item_lang, ci.notes, ci.created_at, \
            c.id AS card_id, c.set_id, c.name AS card_name, c.number, c.rarity, c.supertype, \
            c.lang AS card_lang, c.image_url, c.image_local, s.name AS set_name \
     FROM collection_items ci \
     JOIN cards c ON c.id = ci.card_id \
     LEFT JOIN sets s ON s.id = c.set_id";

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
            // Las tags se rellenan aparte (ver `tags_by_item`) para evitar N+1.
            tags: Vec::new(),
        }
    }
}

/// Fila plana del JOIN collection_item_tags + tags, con el item al que pertenece.
#[derive(Debug, FromRow)]
struct ItemTagRow {
    item_id: String,
    id: String,
    name: String,
}

/// Carga TODAS las asociaciones item-tag de una vez y las agrupa por item_id.
/// Evita una query por item (N+1). Dentro de cada item las tags van por nombre.
async fn tags_by_item(pool: &sqlx::SqlitePool) -> Result<HashMap<String, Vec<TagRef>>, ApiError> {
    let rows = sqlx::query_as::<_, ItemTagRow>(
        "SELECT cit.item_id, t.id, t.name \
         FROM collection_item_tags cit \
         JOIN tags t ON t.id = cit.tag_id \
         ORDER BY t.name COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<String, Vec<TagRef>> = HashMap::new();
    for row in rows {
        map.entry(row.item_id).or_default().push(TagRef {
            id: row.id,
            name: row.name,
        });
    }
    Ok(map)
}

/// Lista todos los items de la coleccion con su carta embebida.
pub async fn list_items(
    State(state): State<AppState>,
) -> Result<Json<CollectionResponse>, ApiError> {
    let rows = sqlx::query_as::<_, CollectionRow>(&format!(
        "{COLLECTION_SELECT} ORDER BY ci.created_at DESC"
    ))
    .fetch_all(&state.pool)
    .await?;

    // Segunda query con todas las tags, agrupadas por item (sin N+1).
    let mut tags = tags_by_item(&state.pool).await?;

    let items: Vec<CollectionItemResponse> = rows
        .into_iter()
        .map(|row| {
            let mut item: CollectionItemResponse = row.into();
            item.tags = tags.remove(&item.id).unwrap_or_default();
            item
        })
        .collect();

    Ok(Json(CollectionResponse { items }))
}

/// Exporta la coleccion completa como documento JSON portable y versionado
/// (GET /api/collection/export). El mismo documento se puede reimportar.
pub async fn export_collection(
    State(state): State<AppState>,
) -> Result<Json<CollectionExport>, ApiError> {
    let rows = sqlx::query_as::<_, CollectionRow>(&format!(
        "{COLLECTION_SELECT} ORDER BY ci.created_at DESC"
    ))
    .fetch_all(&state.pool)
    .await?;

    let mut tags = tags_by_item(&state.pool).await?;

    let items: Vec<CollectionExportItem> = rows
        .into_iter()
        .map(|row| {
            let tag_names = tags
                .remove(&row.id)
                .unwrap_or_default()
                .into_iter()
                .map(|tag| tag.name)
                .collect();
            CollectionExportItem {
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
                tags: tag_names,
            }
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
        let mut item_id: Option<String> = None;
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
                item_id = Some(existing_id);
            }
        }

        if item_id.is_none() {
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
            item_id = Some(id);
        }

        // Reasocia las tags por nombre (creandolas si no existen). Se vacian
        // primero las asociaciones del item para que reimportar sea idempotente.
        // Los nombres en blanco se ignoran.
        let item_id = item_id.expect("item_id resuelto arriba");
        sqlx::query("DELETE FROM collection_item_tags WHERE item_id = ?1")
            .bind(&item_id)
            .execute(&mut *tx)
            .await?;
        for tag_name in &item.tags {
            if tag_name.trim().is_empty() {
                continue;
            }
            // &mut *tx desreferencia la transaccion a su conexion subyacente.
            let tag = upsert_tag_by_name(&mut tx, tag_name).await?;
            sqlx::query(
                "INSERT OR IGNORE INTO collection_item_tags (item_id, tag_id) VALUES (?1, ?2)",
            )
            .bind(&item_id)
            .bind(&tag.id)
            .execute(&mut *tx)
            .await?;
        }
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

    // Control de duplicados: una carta solo puede estar una vez en la coleccion.
    // (El indice unico ux_collection_user_card es el respaldo a nivel de BD.)
    let already_present: Option<String> =
        sqlx::query_scalar("SELECT id FROM collection_items WHERE card_id = ?1 AND user_id IS NULL LIMIT 1")
            .bind(&body.card_id)
            .fetch_optional(&state.pool)
            .await?;
    if already_present.is_some() {
        return Err(ApiError::Conflict(format!(
            "la carta ya esta en tu coleccion: {}",
            body.card_id
        )));
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
        tags: Vec::new(),
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

/// Actualiza un item de coleccion (PATCH). Pensado sobre todo para editar la
/// nota; tambien admite cantidad/estado. Cada campo se aplica con COALESCE (si
/// se omite, conserva el valor). 404 si el item no existe. Responde 200 con el
/// item actualizado (con sus etiquetas).
pub async fn update_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCollectionItemRequest>,
) -> Result<Json<CollectionItemResponse>, ApiError> {
    let result = sqlx::query(
        "UPDATE collection_items \
         SET notes = COALESCE(?1, notes), \
             quantity = COALESCE(?2, quantity), \
             condition = COALESCE(?3, condition) \
         WHERE id = ?4",
    )
    .bind(body.notes.as_deref())
    .bind(body.quantity)
    .bind(body.condition.as_deref())
    .bind(&id)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "item de coleccion no encontrado: {id}"
        )));
    }

    let row = sqlx::query_as::<_, CollectionRow>(&format!("{COLLECTION_SELECT} WHERE ci.id = ?1"))
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    let mut item: CollectionItemResponse = row.into();
    item.tags = sqlx::query_as::<_, TagRef>(
        "SELECT t.id, t.name FROM collection_item_tags cit \
         JOIN tags t ON t.id = cit.tag_id \
         WHERE cit.item_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(item))
}

/// Asocia una etiqueta (por nombre) a un item de coleccion. Crea o reutiliza la
/// tag por nombre (case-insensitive) y es idempotente respecto a la asociacion.
/// 404 si el item no existe. Responde 200 con {id,name} de la tag.
pub async fn add_item_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<TagNameRequest>,
) -> Result<Json<TagRef>, ApiError> {
    let item_exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM collection_items WHERE id = ?1")
            .bind(&id)
            .fetch_optional(&state.pool)
            .await?;
    if item_exists.is_none() {
        return Err(ApiError::NotFound(format!(
            "item de coleccion no encontrado: {id}"
        )));
    }

    let tag = upsert_tag_by_name_pool(&state.pool, &body.name).await?;
    sqlx::query("INSERT OR IGNORE INTO collection_item_tags (item_id, tag_id) VALUES (?1, ?2)")
        .bind(&id)
        .bind(&tag.id)
        .execute(&state.pool)
        .await?;

    Ok(Json(tag))
}

/// Quita la asociacion de una etiqueta con un item de coleccion. La tag en si
/// no se borra (puede seguir usandose en otros items). Responde 204.
pub async fn remove_item_tag(
    State(state): State<AppState>,
    Path((id, tag_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM collection_item_tags WHERE item_id = ?1 AND tag_id = ?2")
        .bind(&id)
        .bind(&tag_id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
