import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import type { Card } from '../lib/types';
import { getCards, imageSrc } from '../lib/api';
import { useT } from '../lib/i18n';

type SearchState = 'idle' | 'searching' | 'done' | 'error';

/**
 * Buscador de cartas del catalogo. Escribe y, con un pequeno retardo (debounce),
 * consulta el backend (que busca por nombre, numero y set) mostrando hasta 10
 * resultados que enlazan con la ficha de cada carta.
 */
export default function SearchPage() {
  const { t } = useT();
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<Card[] | null>(null);
  const [state, setState] = useState<SearchState>('idle');
  // Identifica la peticion en curso para descartar respuestas obsoletas.
  const reqId = useRef(0);

  useEffect(() => {
    const q = query.trim();
    if (q.length < 2) {
      setResults(null);
      setState('idle');
      return;
    }
    const myId = ++reqId.current;
    setState('searching');
    const handle = setTimeout(() => {
      getCards({ q, page_size: 10 })
        .then((res) => {
          if (myId !== reqId.current) return;
          setResults(res.items);
          setState('done');
        })
        .catch(() => {
          if (myId !== reqId.current) return;
          setState('error');
        });
    }, 250);
    return () => clearTimeout(handle);
  }, [query]);

  const trimmed = query.trim();

  return (
    <div className="page search-page">
      <header className="page-header">
        <h1>{t('search.title')}</h1>
        <p className="subtitle">{t('search.subtitle')}</p>
      </header>

      <input
        type="search"
        className="search-input"
        value={query}
        placeholder={t('search.placeholder')}
        aria-label={t('search.placeholder')}
        onChange={(event) => setQuery(event.target.value)}
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
      />

      {trimmed.length > 0 && trimmed.length < 2 && <p className="hint">{t('search.minChars')}</p>}
      {state === 'searching' && <p className="hint">{t('search.searching')}</p>}
      {state === 'error' && <p className="error-text">{t('search.error')}</p>}
      {state === 'done' && results && results.length === 0 && (
        <p className="empty-state">{t('search.empty', { q: trimmed })}</p>
      )}

      {state === 'done' && results && results.length > 0 && (
        <>
          <ul className="search-results">
            {results.map((card) => (
              <li key={card.id}>
                <Link to={`/carta/${encodeURIComponent(card.id)}`} className="search-result">
                  {imageSrc(card) ? (
                    <img
                      className="search-result-thumb"
                      src={imageSrc(card)}
                      alt={card.name}
                      loading="lazy"
                    />
                  ) : (
                    <span className="search-result-thumb card-tile-placeholder" aria-hidden="true">
                      ?
                    </span>
                  )}
                  <span className="search-result-info">
                    <span className="search-result-name">{card.name}</span>
                    <span className="search-result-meta">
                      {t('common.setNumber', {
                        set: card.set_name ?? card.set_id,
                        number: card.number,
                      })}
                      {card.rarity ? ` · ${card.rarity}` : ''} · {card.lang}
                    </span>
                  </span>
                </Link>
              </li>
            ))}
          </ul>
          <p className="hint">{t('search.resultsHint')}</p>
        </>
      )}
    </div>
  );
}
