import { useState } from 'react';
import { Link } from 'react-router-dom';
import type { ScanCandidateView, ScanResponse } from '../lib/types';
import { addToCollection, imageSrc } from '../lib/api';
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

type SaveState = 'idle' | 'saving' | 'saved' | 'error';

export default function ResultPanel({ result }: ResultPanelProps) {
  const [selected, setSelected] = useState<ScanCandidateView | null>(
    result.candidates.length > 0 ? result.candidates[0] : null
  );
  const [saveState, setSaveState] = useState<SaveState>('idle');

  if (!result.best || !selected) {
    return (
      <section className="result-panel">
        <h2>Sin coincidencias</h2>
        <p className="empty-state">
          No se ha podido identificar ninguna carta en la foto. Prueba con mejor luz, fondo liso y
          la carta ocupando la mayor parte del encuadre.
        </p>
      </section>
    );
  }

  const card = selected.card;
  const alternatives = result.candidates.filter((candidate) => candidate.card.id !== card.id);

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
    } catch {
      setSaveState('error');
    }
  }

  return (
    <section className="result-panel">
      <h2>Resultado</h2>

      {result.low_confidence && (
        <div className="warning-banner" role="alert">
          Confianza baja: revisa las alternativas y elige la carta correcta antes de guardar.
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
            {card.set_name ?? card.set_id} · Nº {card.number}
            {card.rarity ? ` · ${card.rarity}` : ''}
          </p>
          <p className="best-card-meta">Idioma: {card.lang}</p>
          <div className="best-card-confidence">
            <span className="label">Confianza</span>
            <ConfidenceBar value={selected.confidence} />
          </div>
        </div>
      </div>

      {result.low_confidence && alternatives.length > 0 && (
        <div className="alternatives">
          <h4>¿No es esta? Elige la correcta:</h4>
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
          disabled={saveState === 'saving' || saveState === 'saved'}
        >
          {saveState === 'saving'
            ? 'Guardando…'
            : saveState === 'saved'
              ? 'Guardada en la colección ✓'
              : 'Guardar en colección'}
        </button>
        {saveState === 'saved' && (
          <p className="success-text">
            Carta añadida. <Link to="/coleccion">Ver colección</Link>
          </p>
        )}
        {saveState === 'error' && (
          <p className="error-text">No se pudo guardar la carta. Inténtalo de nuevo.</p>
        )}
      </div>
    </section>
  );
}
