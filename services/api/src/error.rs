//! Error unificado de la API con conversion a respuesta JSON.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Recurso inexistente -> 404.
    #[error("{0}")]
    NotFound(String),

    /// Peticion invalida del cliente -> 400.
    #[error("{0}")]
    BadRequest(String),

    /// El servicio ML no responde o devuelve error -> 502.
    #[error("{0}")]
    MlUnavailable(String),

    /// Error de base de datos -> 500 (404 si la fila no existe).
    #[error(transparent)]
    Database(#[from] sqlx::Error),

    /// Cualquier otro error interno -> 500.
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::MlUnavailable(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            ApiError::Database(sqlx::Error::RowNotFound) => {
                (StatusCode::NOT_FOUND, "recurso no encontrado".to_string())
            }
            other => {
                tracing::error!(error = %other, "error interno");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "error interno del servidor".to_string(),
                )
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
