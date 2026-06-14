import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import type { ScanCandidateView, ScanResponse } from '../lib/types';
import { addToCollection, ApiRequestError, getCollection, imageSrc } from '../lib/api';
import { useT } from '../lib/i18n';
import CardTile from './CardTile';
import ConfidenceBar from './ConfidenceBar';

/**
 * Panel de resultado de un escaneo: mejor candidato con su confianza,
 * alternativas clicables si la confianza es baja y boton para guardar
 * la carta elegida en la coleccion.
 */
interface ResultPanelProps {
  result: ScanResponse;
}

type SaveState = 'idle' | 'saving' | 'saved' | 'exists' | 'error';

export default function ResultPanel({ result }: ResultPanelProps) {
  const { t } = useT();
  const [selected, setSelected] = useState<ScanCandidateView | null>(
    result.candidates.length > 0 ? result.candidates[0] : null
  );
  const [saveState, setSaveState] = useState<SaveState>('idle');
  // IDs de cartas ya en la coleccion, para avisar ANTES de pulsar Guardar.
  const [collectionIds, setCollectionIds] = useState<Set<string> | null>(null);

  useEffect(() => {
    getCollection()
      .then((response) => setCollectionIds(new Set(response.items.map((item) => item.card.id))))
      .catch(() => setCollectionIds(new Set()));
  }, []);

  if (!result.best || !selected) {
    return (
      <section className="result-panel">
        <h2>{t('result.noMatch.title')}</h2>
        <p className="empty-state">{t('result.noMatch.text')}</p>
      </section>
    );
  }

  const card = selected.card;
  const alternatives = result.candidates.filter((candidate) => candidate.card.id !== card.id);
  // Estado efectivo del boton: si ya esta en la coleccion, se refleja sin pulsar.
  const alreadyInCollection = collectionIds?.has(card.id) ?? false;
  const effectiveState: SaveState =
    saveState !== 'idle' ? saveState : alreadyInCollection ? 'exists' : 'idle';

  async function handleSave() {
    if (!selected) return;
    setSaveState('saving');
    try {
      await addToCollection({
        card_id: selected.card.id,
        scan_id: result.scan_id,
        quantity: 1,
        condition: null,
        lang: selected.card.lang,
        notes: null,
      });
      setSaveState('saved');
      setCollectionIds((prev) => new Set(prev ?? []).add(selected.card.id));
    } catch (err) {
      // 409 = la carta ya esta en la coleccion (no es un fallo real).
      if (err instanceof ApiRequestError && err.status === 409) {
        setSaveState('exists');
        setCollectionIds((prev) => new Set(prev ?? []).add(selected.card.id));
      } else {
        setSaveState('error');
      }
    }
  }

  return (
    <section className="result-panel">
      <h2>{t('result.title')}</h2>

      {result.low_confidence && (
        <div className="warning-banner" role="alert">
          {t('result.lowConfidence')}
        </div>
      )}

      <div className="best-card">
        {imageSrc(card) ? (
          <img className="best-card-image" src={imageSrc(card)} alt={card.name} />
        ) : (
          <div className="best-card-image card-tile-placeholder" aria-hidden="true">
            ?
          </div>
        )}
        <div className="best-card-info">
          <h3>
            <Link to={`/carta/${encodeURIComponent(card.id)}`}>{card.name}</Link>
          </h3>
          <p className="best-card-meta">
            {t('common.setNumber', { set: card.set_name ?? card.set_id, number: card.number })}
            {card.rarity ? ` · ${card.rarity}` : ''}
          </p>
          <p className="best-card-meta">
            {t('common.language')}: {card.lang}
          </p>
          <div className="best-card-confidence">
            <span className="label">{t('common.confidence')}</span>
            <ConfidenceBar value={selected.confidence} />
          </div>
        </div>
      </div>

      {result.low_confidence && alternatives.length > 0 && (
        <div className="alternatives">
          <h4>{t('result.altPrompt')}</h4>
          <div className="alternatives-grid">
            {alternatives.map((candidate) => (
              <CardTile
                key={candidate.card.id}
                card={candidate.card}
                confidence={candidate.confidence}
                selected={false}
                onClick={() => {
                  setSelected(candidate);
                  setSaveState('idle');
                }}
              />
            ))}
          </div>
        </div>
      )}

      <div className="result-actions">
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => void handleSave()}
          disabled={
            effectiveState === 'saving' || effectiveState === 'saved' || effectiveState === 'exists'
          }
        >
          {effectiveState === 'saving'
            ? t('result.saving')
            : effectiveState === 'saved'
              ? t('result.saved')
              : effectiveState === 'exists'
                ? t('result.alreadySaved')
                : t('result.save')}
        </button>
        {effectiveState === 'saved' && (
          <p className="success-text">
            {t('result.savedText')} <Link to="/coleccion">{t('result.viewCollection')}</Link>
          </p>
        )}
        {effectiveState === 'exists' && (
          <p className="success-text">
            {t('result.alreadyInCollection')} <Link to="/coleccion">{t('result.viewCollection')}</Link>
          </p>
        )}
        {effectiveState === 'error' && <p className="error-text">{t('result.saveError')}</p>}
      </div>
    </section>
  );
}
