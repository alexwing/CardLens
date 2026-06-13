//! Conectores de precios desacoplados. Seleccion por env PRICE_PROVIDER=null|tcgdex.
//! Dispatch por enum para evitar la dependencia async_trait.

pub mod null;
pub mod tcgdex;

use crate::models::PriceQuote;
use null::NullPriceProvider;
use tcgdex::TcgdexPriceProvider;

/// Proveedor de precios activo.
#[derive(Clone, Debug)]
pub enum PriceProviderKind {
    Null(NullPriceProvider),
    Tcgdex(TcgdexPriceProvider),
}

impl PriceProviderKind {
    /// Construye el proveedor a partir del valor de PRICE_PROVIDER.
    /// Cualquier valor desconocido cae en `null` (con aviso en el log).
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "tcgdex" => Self::Tcgdex(TcgdexPriceProvider::new()),
            "null" | "" => Self::Null(NullPriceProvider),
            other => {
                tracing::warn!("PRICE_PROVIDER desconocido '{other}', se usa 'null'");
                Self::Null(NullPriceProvider)
            }
        }
    }

    /// Obtiene cotizaciones para una carta.
    /// `tcgdex_id` y `lang` se derivan de `card_id` partiendo por el ultimo guion bajo.
    pub async fn prices(
        &self,
        card_id: &str,
        tcgdex_id: &str,
        lang: &str,
    ) -> anyhow::Result<Vec<PriceQuote>> {
        match self {
            Self::Null(provider) => provider.prices(card_id, tcgdex_id, lang).await,
            Self::Tcgdex(provider) => provider.prices(card_id, tcgdex_id, lang).await,
        }
    }
}
