import { useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import type { Card, PriceQuote } from '../lib/types';
import { getCard, getPrices, imageSrc } from '../lib/api';
import { intlLocale, useT } from '../lib/i18n';

/**
 * Detalle de una carta: imagen oficial, metadata del catalogo y precios.
 * Si no hay fuente de precios configurada (lista vacia) muestra un estado
 * vacio elegante.
 */
export default function CardPage() {
  const { t, locale } = useT();
  const { id } = useParams<{ id: string }>();
  const [card, setCard] = useState<Card | null>(null);
  const [prices, setPrices] = useState<PriceQuote[] | null>(null);
  const [hasError, setHasError] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!id) return;
    let cancelled = false;

    async function load(cardId: string) {
      setLoading(true);
      setHasError(false);
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
          setHasError(true);
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
      return new Intl.NumberFormat(intlLocale(locale), { style: 'currency', currency }).format(value);
    } catch {
      return `${value.toFixed(2)} ${currency}`;
    }
  }

  return (
    <div className="page card-page">
      <header className="page-header">
        <Link to="/coleccion" className="back-link">
          {t('card.back')}
        </Link>
      </header>

      {loading && (
        <div className="loading" role="status">
          <div className="spinner" aria-hidden="true" />
          <p>{t('card.loading')}</p>
        </div>
      )}

      {hasError && (
        <div className="error-banner" role="alert">
          {t('card.error')}
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
                  <dt>{t('card.meta.set')}</dt>
                  <dd>{card.set_name ?? card.set_id}</dd>
                </div>
                <div>
                  <dt>{t('card.meta.number')}</dt>
                  <dd>{card.number}</dd>
                </div>
                <div>
                  <dt>{t('card.meta.rarity')}</dt>
                  <dd>{card.rarity ?? '—'}</dd>
                </div>
                <div>
                  <dt>{t('card.meta.type')}</dt>
                  <dd>{card.supertype ?? '—'}</dd>
                </div>
                <div>
                  <dt>{t('common.language')}</dt>
                  <dd>{card.lang}</dd>
                </div>
              </dl>
            </div>
          </div>

          <section className="prices-section">
            <h2>{t('card.prices')}</h2>
            {prices === null && <p className="hint">{t('card.prices.loading')}</p>}
            {prices !== null && prices.length === 0 && (
              <div className="empty-state">
                <p>{t('card.prices.empty')}</p>
                <p className="hint">{t('card.prices.emptyHint')}</p>
              </div>
            )}
            {prices !== null && prices.length > 0 && (
              <table className="prices-table">
                <thead>
                  <tr>
                    <th>{t('card.prices.source')}</th>
                    <th>{t('card.prices.market')}</th>
                    <th>{t('card.prices.low')}</th>
                    <th>{t('card.prices.high')}</th>
                    <th>{t('card.prices.trend')}</th>
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
