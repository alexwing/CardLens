//! Proveedor de precios basado en TCGdex v2.
//!
//! Hace GET https://api.tcgdex.net/v2/{lang}/cards/{tcgdex_id} y parsea el campo
//! opcional `pricing` de forma 100% defensiva con serde_json::Value: si el campo
//! no existe, la peticion falla o el formato cambia, devuelve un vector vacio.
//! Este proveedor JAMAS devuelve error.

use std::time::Duration;

use serde_json::{Map, Value};

use crate::models::PriceQuote;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_BASE_URL: &str = "https://api.tcgdex.net/v2";

#[derive(Clone, Debug)]
pub struct TcgdexPriceProvider {
    client: reqwest::Client,
    base_url: String,
}

impl TcgdexPriceProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self {
            client,
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub async fn prices(
        &self,
        card_id: &str,
        tcgdex_id: &str,
        lang: &str,
    ) -> anyhow::Result<Vec<PriceQuote>> {
        let url = format!("{}/{}/cards/{}", self.base_url, lang, tcgdex_id);
        let response = match self.client.get(&url).send().await {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(%card_id, %error, "TCGdex no responde, sin precios");
                return Ok(Vec::new());
            }
        };
        if !response.status().is_success() {
            tracing::warn!(%card_id, status = %response.status(), "TCGdex devolvio error, sin precios");
            return Ok(Vec::new());
        }
        let body: Value = match response.json().await {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(%card_id, %error, "respuesta de TCGdex no es JSON, sin precios");
                return Ok(Vec::new());
            }
        };
        Ok(parse_pricing(&body))
    }
}

impl Default for TcgdexPriceProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Extrae cotizaciones del campo opcional `pricing` del detalle de carta.
fn parse_pricing(card: &Value) -> Vec<PriceQuote> {
    let Some(pricing) = card.get("pricing").and_then(Value::as_object) else {
        return Vec::new();
    };
    let now = chrono::Utc::now().to_rfc3339();
    let mut quotes = Vec::new();

    for (source, value) in pricing {
        let Some(source_obj) = value.as_object() else {
            continue;
        };
        let currency = currency_of(source_obj).unwrap_or_else(|| default_currency(source));

        // Primero intenta extraer numeros del nivel del source...
        if let Some(quote) = quote_from_object(source, source_obj, &currency, &now) {
            quotes.push(quote);
            continue;
        }
        // ...y si no hay, busca en sub-objetos (variantes tipo normal/holofoil).
        for variant in source_obj.values() {
            if let Some(variant_obj) = variant.as_object() {
                let variant_currency = currency_of(variant_obj).unwrap_or_else(|| currency.clone());
                if let Some(quote) = quote_from_object(source, variant_obj, &variant_currency, &now)
                {
                    quotes.push(quote);
                    break;
                }
            }
        }
    }
    quotes
}

/// Construye una cotizacion si el objeto contiene algun valor numerico conocido.
fn quote_from_object(
    source: &str,
    obj: &Map<String, Value>,
    currency: &str,
    now: &str,
) -> Option<PriceQuote> {
    let market = first_number(obj, &["market", "marketPrice", "avg", "avg1", "averageSellPrice"]);
    let low = first_number(obj, &["low", "lowPrice", "lowest", "minPrice"]);
    let high = first_number(obj, &["high", "highPrice", "highest", "maxPrice"]);
    let trend = first_number(obj, &["trend", "trendPrice"]);

    if market.is_none() && low.is_none() && high.is_none() && trend.is_none() {
        return None;
    }
    Some(PriceQuote {
        source: source.to_string(),
        currency: currency.to_string(),
        market,
        low,
        high,
        trend,
        updated_at: now.to_string(),
    })
}

fn currency_of(obj: &Map<String, Value>) -> Option<String> {
    obj.get("unit")
        .or_else(|| obj.get("currency"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn default_currency(source: &str) -> String {
    match source {
        "tcgplayer" => "USD",
        _ => "EUR",
    }
    .to_string()
}

fn first_number(obj: &Map<String, Value>, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| obj.get(*key).and_then(Value::as_f64))
}
