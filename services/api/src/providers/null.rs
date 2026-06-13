//! Proveedor de precios nulo (default): nunca devuelve cotizaciones.

use crate::models::PriceQuote;

#[derive(Clone, Debug, Default)]
pub struct NullPriceProvider;

impl NullPriceProvider {
    pub async fn prices(
        &self,
        _card_id: &str,
        _tcgdex_id: &str,
        _lang: &str,
    ) -> anyhow::Result<Vec<PriceQuote>> {
        Ok(Vec::new())
    }
}
