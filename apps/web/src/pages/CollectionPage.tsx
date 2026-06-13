import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import type { CollectionItem } from '../lib/types';
import { getCollection, imageSrc, removeFromCollection } from '../lib/api';

/**
 * Pagina de coleccion: lista los items guardados con miniatura, nombre,
 * set, cantidad y fecha, y permite eliminarlos con confirmacion.
 */
export default function CollectionPage() {
  const [items, setItems] = useState<CollectionItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await getCollection();
      setItems(response.items);
    } catch {
      setError('No se pudo cargar la colección. Comprueba que la API está en marcha.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleDelete(item: CollectionItem) {
    const ok = window.confirm(`¿Eliminar «${item.card.name}» de tu colección?`);
    if (!ok) return;
    setDeletingId(item.id);
    try {
      await removeFromCollection(item.id);
      setItems((current) => current.filter((existing) => existing.id !== item.id));
    } catch {
      setError('No se pudo eliminar la carta. Inténtalo de nuevo.');
    } finally {
      setDeletingId(null);
    }
  }

  function formatDate(iso: string): string {
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) {
      return iso;
    }
    return date.toLocaleDateString('es-ES', { day: '2-digit', month: 'short', year: 'numeric' });
  }

  return (
    <div className="page collection-page">
      <header className="page-header">
        <h1>Mi colección</h1>
        <p className="page-subtitle">
          {items.length === 1 ? '1 carta guardada' : `${items.length} cartas guardadas`}
        </p>
      </header>

      {loading && (
        <div className="loading" role="status">
          <div className="spinner" aria-hidden="true" />
          <p>Cargando colección…</p>
        </div>
      )}

      {error && (
        <div className="error-banner" role="alert">
          {error}
        </div>
      )}

      {!loading && !error && items.length === 0 && (
        <div className="empty-state">
          <p>Todavía no tienes cartas en la colección.</p>
          <Link to="/" className="btn btn-primary">
            Escanear mi primera carta
          </Link>
        </div>
      )}

      <ul className="collection-list">
        {items.map((item) => (
          <li key={item.id} className="collection-item">
            <Link to={`/carta/${encodeURIComponent(item.card.id)}`} className="collection-thumb-link">
              {imageSrc(item.card) ? (
                <img
                  className="collection-thumb"
                  src={imageSrc(item.card)}
                  alt={item.card.name}
                  loading="lazy"
                />
              ) : (
                <div className="collection-thumb card-tile-placeholder" aria-hidden="true">
                  ?
                </div>
              )}
            </Link>
            <div className="collection-info">
              <Link to={`/carta/${encodeURIComponent(item.card.id)}`} className="collection-name">
                {item.card.name}
              </Link>
              <span className="collection-meta">
                {item.card.set_name ?? item.card.set_id} · Nº {item.card.number}
              </span>
              <span className="collection-meta">
                Cantidad: {item.quantity} · Añadida el {formatDate(item.created_at)}
              </span>
            </div>
            <button
              type="button"
              className="btn btn-danger btn-small"
              onClick={() => void handleDelete(item)}
              disabled={deletingId === item.id}
              aria-label={`Eliminar ${item.card.name}`}
            >
              {deletingId === item.id ? '…' : 'Eliminar'}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
