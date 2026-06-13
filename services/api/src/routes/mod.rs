//! Router de la API publica (prefijo /api).

pub mod cards;
pub mod collection;
pub mod health;
pub mod prices;
pub mod scan;
pub mod scans;
pub mod sets;

use axum::routing::{delete, get, post};
use axum::Router;

use crate::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health::health))
        .route("/api/scan", post(scan::create_scan))
        .route("/api/cards", get(cards::list_cards))
        .route("/api/cards/{id}", get(cards::get_card))
        .route("/api/sets", get(sets::list_sets))
        .route("/api/scans", get(scans::list_scans))
        .route("/api/collection", get(collection::list_items))
        .route("/api/collection/export", get(collection::export_collection))
        .route("/api/collection/import", post(collection::import_collection))
        .route("/api/collection/items", post(collection::create_item))
        .route("/api/collection/items/{id}", delete(collection::delete_item))
        .route("/api/prices/{card_id}", get(prices::get_prices))
}
