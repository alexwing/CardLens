import { useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import type { Card, PriceQuote } from '../lib/types';
import { getCard, getPrices, imageSrc } from '../lib/api';

/**
 * Detalle de una carta: imagen oficial, metadata del catalogo y precios.
 * Si no hay fuente de precios configurada (lista vacia) muestra un estado
 * vacio elegante.
 */
export default function CardPage() {
  const { id } = useParams<{ id: string }>();
  const [card, setCard] = useState<Card | null>(null);
  const [prices, setPrices] = useState<PriceQuote[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!id) return;
    let cancelled = false;

    async function load(cardId: string) {
      setLoading(true);
      setError(null);
      try {
        const cardData = await getCard(cardId);
        if (cancelled) return;
        setCard(cardData);
        try {
          const priceData = await getPrices(cardId);
          if (!cancelled) {
            setPrices(priceData.prices);
          }
        } catch {
          if (!cancelled) {
            setPrices([]);
          }
        }
      } catch {
        if (!cancelled) {
          setError('No se pudo cargar la carta. Puede que no exista en el catálogo.');
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void load(id);
    return () => {
      cancelled = true;
    };
  }, [id]);

  function formatMoney(value: number | null, currency: string): string {
    if (value === null) return '—';
    try {
      return new Intl.NumberFormat('es-ES', { style: 'currency', currency }).format(value);
    } catch {
      return `${value.toFixed(2)} ${currency}`;
    }
  }

  return (
    <div className="page card-page">
      <header className="page-header">
        <Link to="/coleccion" className="back-link">
          ← Volver
        </Link>
      </header>

      {loading && (
        <div className="loading" role="status">
          <div className="spinner" aria-hidden="true" />
          <p>Cargando carta…</p>
        </div>
      )}

      {error && (
        <div className="error-banner" role="alert">
          {error}
        </div>
      )}

      {!loading && card && (
        <>
          <div className="card-detail">
            {imageSrc(card) ? (
              <img className="card-detail-image" src={imageSrc(card)} alt={card.name} />
            ) : (
              <div className="card-detail-image card-tile-placeholder" aria-hidden="true">
                ?
              </div>
            )}
            <div className="card-detail-info">
              <h1>{card.name}</h1>
              <dl className="card-detail-meta">
                <div>
                  <dt>Set</dt>
                  <dd>{card.set_name ?? card.set_id}</dd>
                </div>
                <div>
                  <dt>Número</dt>
                  <dd>{card.number}</dd>
                </div>
                <div>
                  <dt>Rareza</dt>
                  <dd>{card.rarity ?? '—'}</dd>
                </div>
                <div>
                  <dt>Tipo</dt>
                  <dd>{card.supertype ?? '—'}</dd>
                </div>
                <div>
                  <dt>Idioma</dt>
                  <dd>{card.lang}</dd>
                </div>
              </dl>
            </div>
          </div>

          <section className="prices-section">
            <h2>Precios</h2>
            {prices === null && <p className="hint">Cargando precios…</p>}
            {prices !== null && prices.length === 0 && (
              <div className="empty-state">
                <p>Sin fuente de precios configurada.</p>
                <p className="hint">
                  Configura un proveedor de precios en la API (PRICE_PROVIDER) para ver
                  cotizaciones aquí.
                </p>
              </div>
            )}
            {prices !== null && prices.length > 0 && (
              <table className="prices-table">
                <thead>
                  <tr>
                    <th>Fuente</th>
                    <th>Mercado</th>
                    <th>Mínimo</th>
                    <th>Máximo</th>
                    <th>Tendencia</th>
                  </tr>
                </thead>
                <tbody>
                  {prices.map((quote) => (
                    <tr key={quote.source}>
                      <td>{quote.source}</td>
                      <td>{formatMoney(quote.market, quote.currency)}</td>
                      <td>{formatMoney(quote.low, quote.currency)}</td>
                      <td>{formatMoney(quote.high, quote.currency)}</td>
                      <td>{formatMoney(quote.trend, quote.currency)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>
        </>
      )}
    </div>
  );
}
