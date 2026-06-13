//! GET /api/health

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::AppState;

pub async fn health(State(state): State<AppState>) -> Json<Value> {
    let (loaded, index_size) = match &state.engine {
        Some(engine) => (true, engine.index_size()),
        None => (false, 0),
    };
    Json(json!({
        "status": "ok",
        // Compatibilidad con el cliente: "ml.reachable" = reconocedor cargado.
        "ml": { "reachable": loaded },
        "recognizer": {
            "loaded": loaded,
            "index_size": index_size,
            "engine": "mobileclip-visual"
        }
    }))
}
