//! GET /api/prices/{card_id} con cache de 24h en la tabla `prices`.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Duration, Utc};

use crate::error::ApiError;
use crate::models::{Card, PriceQuote, PricesResponse};
use crate::AppState;

const CACHE_TTL_HOURS: i64 = 24;

pub async fn get_prices(
    State(state): State<AppState>,
    Path(card_id): Path<String>,
) -> Result<Json<PricesResponse>, ApiError> {
    // La carta debe existir en el catalogo.
    Card::fetch_by_id(&state.pool, &card_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("carta no encontrada: {card_id}")))?;

    let cached = fetch_cached(&state, &card_id).await?;
    let cache_is_fresh = !cached.is_empty() && cached.iter().all(|quote| is_fresh(&quote.updated_at));
    if cache_is_fresh {
        return Ok(Json(PricesResponse {
            card_id,
            prices: cached,
        }));
    }

    // tcgdex_id y lang se derivan partiendo card_id por el ULTIMO guion bajo.
    let Some((tcgdex_id, lang)) = card_id.rsplit_once('_') else {
        return Err(ApiError::BadRequest(format!(
            "card_id con formato invalido (se esperaba tcgdexId_lang): {card_id}"
        )));
    };
    let (tcgdex_id, lang) = (tcgdex_id.to_string(), lang.to_string());

    match state.price_provider.prices(&card_id, &tcgdex_id, &lang).await {
        Ok(quotes) if !quotes.is_empty() => {
            for quote in &quotes {
                upsert_quote(&state, &card_id, quote).await?;
            }
            Ok(Json(PricesResponse {
                card_id,
                prices: quotes,
            }))
        }
        Ok(_) => {
            // Sin cotizaciones nuevas: devuelve la cache (aunque este caducada) o vacio.
            Ok(Json(PricesResponse {
                card_id,
                prices: cached,
            }))
        }
        Err(error) => {
            tracing::warn!(%card_id, %error, "el proveedor de precios fallo, se devuelve la cache");
            Ok(Json(PricesResponse {
                card_id,
                prices: cached,
            }))
        }
    }
}

async fn fetch_cached(state: &AppState, card_id: &str) -> Result<Vec<PriceQuote>, sqlx::Error> {
    sqlx::query_as::<_, PriceQuote>(
        "SELECT source, currency, market, low, high, trend, updated_at \
         FROM prices WHERE card_id = ?1 ORDER BY source",
    )
    .bind(card_id)
    .fetch_all(&state.pool)
    .await
}

async fn upsert_quote(
    state: &AppState,
    card_id: &str,
    quote: &PriceQuote,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO prices (card_id, source, currency, market, low, high, trend, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
         ON CONFLICT(card_id, source) DO UPDATE SET \
             currency = excluded.currency, market = excluded.market, low = excluded.low, \
             high = excluded.high, trend = excluded.trend, updated_at = excluded.updated_at",
    )
    .bind(card_id)
    .bind(&quote.source)
    .bind(&quote.currency)
    .bind(quote.market)
    .bind(quote.low)
    .bind(quote.high)
    .bind(quote.trend)
    .bind(&quote.updated_at)
    .execute(&state.pool)
    .await?;
    Ok(())
}

/// true si el timestamp RFC3339 tiene menos de 24h de antiguedad.
fn is_fresh(updated_at: &str) -> bool {
    match DateTime::parse_from_rfc3339(updated_at) {
        Ok(timestamp) => {
            Utc::now() - timestamp.with_timezone(&Utc) < Duration::hours(CACHE_TTL_HOURS)
        }
        Err(_) => false,
    }
}
