import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import type { Card, CollectionItem, PriceQuote } from '../lib/types';
import { getCard, getCollection, getPrices, imageSrc, updateCollectionItem } from '../lib/api';
import { intlLocale, useT } from '../lib/i18n';

type NoteState = 'idle' | 'saving' | 'saved' | 'error';

/**
 * Detalle de una carta: imagen oficial, metadata del catalogo y precios.
 * Si la carta esta en la coleccion, permite editar su nota personal.
 */
export default function CardPage() {
  const { t, locale } = useT();
  const navigate = useNavigate();
  const { id } = useParams<{ id: string }>();
  const [card, setCard] = useState<Card | null>(null);
  const [prices, setPrices] = useState<PriceQuote[] | null>(null);
  const [hasError, setHasError] = useState(false);
  const [loading, setLoading] = useState(true);
  // Item de coleccion para esta carta (si esta guardada) y edicion de su nota.
  const [collectionItem, setCollectionItem] = useState<CollectionItem | null>(null);
  const [note, setNote] = useState('');
  const [noteState, setNoteState] = useState<NoteState>('idle');

  useEffect(() => {
    if (!id) return;
    let cancelled = false;

    async function load(cardId: string) {
      setLoading(true);
      setHasError(false);
      setCollectionItem(null);
      setNote('');
      setNoteState('idle');
      try {
        const cardData = await getCard(cardId);
        if (cancelled) return;
        setCard(cardData);

        // Precios (no bloqueante).
        getPrices(cardId)
          .then((priceData) => {
            if (!cancelled) setPrices(priceData.prices);
          })
          .catch(() => {
            if (!cancelled) setPrices([]);
          });

        // ¿Esta en la coleccion? Si lo esta, cargamos su nota.
        getCollection()
          .then((collection) => {
            if (cancelled) return;
            const item = collection.items.find((entry) => entry.card.id === cardId) ?? null;
            setCollectionItem(item);
            setNote(item?.notes ?? '');
          })
          .catch(() => {
            if (!cancelled) setCollectionItem(null);
          });
      } catch {
        if (!cancelled) setHasError(true);
      } finally {
        if (!cancelled) setLoading(false);
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

  async function saveNote() {
    if (!collectionItem) return;
    setNoteState('saving');
    try {
      const updated = await updateCollectionItem(collectionItem.id, { notes: note });
      setCollectionItem(updated);
      setNoteState('saved');
    } catch {
      setNoteState('error');
    }
  }

  return (
    <div className="page card-page">
      <header className="page-header">
        <button type="button" className="back-link" onClick={() => navigate(-1)}>
          {t('card.back')}
        </button>
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
              {collectionItem && <span className="badge-collection">{t('card.inCollection')}</span>}
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

          <section className="note-section">
            <h2>{t('card.note.title')}</h2>
            {collectionItem ? (
              <>
                <textarea
                  className="note-textarea"
                  value={note}
                  rows={4}
                  placeholder={t('card.note.placeholder')}
                  onChange={(event) => {
                    setNote(event.target.value);
                    if (noteState !== 'idle') setNoteState('idle');
                  }}
                />
                <div className="note-actions">
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => void saveNote()}
                    disabled={noteState === 'saving'}
                  >
                    {noteState === 'saving' ? t('card.note.saving') : t('card.note.save')}
                  </button>
                  {noteState === 'saved' && <span className="success-text">{t('card.note.saved')}</span>}
                  {noteState === 'error' && <span className="error-text">{t('card.note.error')}</span>}
                </div>
              </>
            ) : (
              <p className="hint">{t('card.note.addToCollection')}</p>
            )}
          </section>
        </>
      )}
    </div>
  );
}
