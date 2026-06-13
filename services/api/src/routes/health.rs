//! GET /api/health

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::AppState;

pub async fn health(State(state): State<AppState>) -> Json<Value> {
    let reachable = state.ml.health().await;
    Json(json!({
        "status": "ok",
        "ml": { "reachable": reachable }
    }))
}
