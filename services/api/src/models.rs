//! Modelos de base de datos y structs de respuesta del contrato JSON.
//! Los nombres de campo serializados son EXACTOS al contrato del monorepo.

use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use sqlx::SqlitePool;

/// SELECT base de cartas enriquecidas con el nombre del set (JOIN cards+sets).
pub const CARD_SELECT: &str = "SELECT c.id, c.set_id, c.name, c.number, c.rarity, c.supertype, \
     c.lang, c.image_url, c.image_local, s.name AS set_name \
     FROM cards c LEFT JOIN sets s ON s.id = c.set_id";

/// Carta del catalogo tal y como la expone la API publica.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Card {
    pub id: String,
    pub set_id: String,
    pub name: String,
    pub number: String,
    pub rarity: Option<String>,
    pub supertype: Option<String>,
    pub lang: String,
    pub image_url: Option<String>,
    pub image_local: Option<String>,
    pub set_name: Option<String>,
}

impl Card {
    /// Busca una carta por id con el nombre de su set.
    pub async fn fetch_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Card>, sqlx::Error> {
        let sql = format!("{CARD_SELECT} WHERE c.id = ?1");
        sqlx::query_as::<_, Card>(&sql)
            .bind(id)
            .fetch_optional(pool)
            .await
    }
}

/// Set del catalogo (respuesta de GET /api/sets).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SetInfo {
    pub id: String,
    pub name: String,
    pub series: Option<String>,
    pub code: Option<String>,
    pub release_date: Option<String>,
    pub total: Option<i64>,
    pub lang: String,
}

/// Fila completa de la tabla `scans`.
#[derive(Debug, Clone, Serialize, FromRow)]
#[allow(dead_code)]
pub struct Scan {
    pub id: String,
    pub created_at: String,
    pub image_path: String,
    pub status: String,
    pub best_card_id: Option<String>,
    pub confidence: Option<f64>,
    pub low_confidence: bool,
    pub raw_json: Option<String>,
}

/// Resumen de escaneo para el listado paginado (GET /api/scans).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ScanSummary {
    pub id: String,
    pub created_at: String,
    pub best_card_id: Option<String>,
    pub confidence: Option<f64>,
    pub low_confidence: bool,
}

/// Fila completa de la tabla `scan_candidates`.
#[derive(Debug, Clone, Serialize, FromRow)]
#[allow(dead_code)]
pub struct ScanCandidate {
    pub id: i64,
    pub scan_id: String,
    pub card_id: String,
    pub rank: i64,
    pub visual_score: f64,
    pub ocr_score: f64,
    pub final_score: f64,
}

/// Fila completa de la tabla `collection_items`.
#[derive(Debug, Clone, Serialize, FromRow)]
#[allow(dead_code)]
pub struct CollectionItem {
    pub id: String,
    pub user_id: Option<String>,
    pub card_id: String,
    pub scan_id: Option<String>,
    pub quantity: i64,
    pub condition: Option<String>,
    pub lang: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

/// Cotizacion de precio de una fuente externa (sin card_id, como en el contrato).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PriceQuote {
    pub source: String,
    pub currency: String,
    pub market: Option<f64>,
    pub low: Option<f64>,
    pub high: Option<f64>,
    pub trend: Option<f64>,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Structs de respuesta del contrato
// ---------------------------------------------------------------------------

/// Respuesta paginada generica: {"items": [...], "total": n, "page": n}.
#[derive(Debug, Serialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
}

/// Respuesta de POST /api/scan.
#[derive(Debug, Serialize)]
pub struct ScanResponse {
    pub scan_id: String,
    pub low_confidence: bool,
    pub best: Option<BestMatch>,
    pub candidates: Vec<CandidateResponse>,
}

#[derive(Debug, Serialize)]
pub struct BestMatch {
    pub card: Card,
    pub confidence: f64,
}

#[derive(Debug, Serialize)]
pub struct CandidateResponse {
    pub card: Card,
    pub confidence: f64,
    pub visual_score: f64,
    pub ocr_score: f64,
}

/// Item de coleccion con la carta embebida (GET/POST /api/collection*).
#[derive(Debug, Serialize)]
pub struct CollectionItemResponse {
    pub id: String,
    pub card: Card,
    pub quantity: i64,
    pub condition: Option<String>,
    pub lang: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

/// Respuesta de GET /api/collection.
#[derive(Debug, Serialize)]
pub struct CollectionResponse {
    pub items: Vec<CollectionItemResponse>,
}

/// Body de POST /api/collection/items.
#[derive(Debug, Deserialize)]
pub struct CreateCollectionItemRequest {
    pub card_id: String,
    #[serde(default)]
    pub scan_id: Option<String>,
    #[serde(default = "default_quantity")]
    pub quantity: i64,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

fn default_quantity() -> i64 {
    1
}

/// Respuesta de GET /api/prices/{card_id}.
#[derive(Debug, Serialize)]
pub struct PricesResponse {
    pub card_id: String,
    pub prices: Vec<PriceQuote>,
}

// ---------------------------------------------------------------------------
// Importacion / exportacion de la coleccion en JSON
// ---------------------------------------------------------------------------

/// Identificador del formato de fichero de coleccion exportado.
pub const COLLECTION_EXPORT_FORMAT: &str = "pokemoncarddetector.collection";
/// Version del esquema del fichero de coleccion (subir si cambia el formato).
pub const COLLECTION_EXPORT_VERSION: u32 = 1;

/// Snapshot legible de la carta dentro del fichero exportado. Solo
/// informativo: la importacion usa unicamente `card_id`.
#[derive(Debug, Serialize)]
pub struct CollectionExportCard {
    pub name: String,
    pub set_name: Option<String>,
    pub number: String,
}

/// Item de la coleccion en el documento exportado.
#[derive(Debug, Serialize)]
pub struct CollectionExportItem {
    pub card_id: String,
    pub quantity: i64,
    pub condition: Option<String>,
    pub lang: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub card: CollectionExportCard,
}

/// Documento de exportacion (GET /api/collection/export).
#[derive(Debug, Serialize)]
pub struct CollectionExport {
    pub format: String,
    pub version: u32,
    pub exported_at: String,
    pub count: usize,
    pub items: Vec<CollectionExportItem>,
}

/// Item dentro del documento de importacion. Los campos extra del fichero
/// exportado (como `card`) se ignoran, de modo que un fichero exportado se
/// puede reimportar tal cual.
#[derive(Debug, Deserialize)]
pub struct CollectionImportItem {
    pub card_id: String,
    #[serde(default = "default_quantity")]
    pub quantity: i64,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

/// Body de POST /api/collection/import. Acepta el mismo documento que produce
/// la exportacion (format/version/exported_at/count se ignoran).
/// `mode`: "merge" (por defecto, actualiza por card_id) o "replace"
/// (vacia la coleccion antes de insertar).
#[derive(Debug, Deserialize)]
pub struct CollectionImportRequest {
    #[serde(default)]
    pub mode: Option<String>,
    pub items: Vec<CollectionImportItem>,
}

/// Carta omitida durante la importacion, con el motivo.
#[derive(Debug, Serialize)]
pub struct SkippedImportItem {
    pub card_id: String,
    pub reason: String,
}

/// Resumen de POST /api/collection/import.
#[derive(Debug, Serialize)]
pub struct CollectionImportSummary {
    pub mode: String,
    pub imported: usize,
    pub updated: usize,
    pub skipped: Vec<SkippedImportItem>,
}
