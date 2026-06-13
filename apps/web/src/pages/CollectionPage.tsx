import { useCallback, useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import type { CollectionItem } from '../lib/types';
import {
  exportCollection,
  getCollection,
  imageSrc,
  importCollection,
  removeFromCollection,
} from '../lib/api';
import { intlLocale, useT } from '../lib/i18n';
import type { TranslationKey } from '../lib/locales/es';

/**
 * Pagina de coleccion: lista los items guardados con miniatura, nombre,
 * set, cantidad y fecha, permite eliminarlos, y exportar/importar la
 * coleccion completa en formato JSON.
 */
export default function CollectionPage() {
  const { t, locale } = useT();
  const [items, setItems] = useState<CollectionItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await getCollection();
      setItems(response.items);
    } catch {
      setError(t('collection.loadError'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleDelete(item: CollectionItem) {
    const ok = window.confirm(t('collection.removeConfirm', { name: item.card.name }));
    if (!ok) return;
    setDeletingId(item.id);
    try {
      await removeFromCollection(item.id);
      setItems((current) => current.filter((existing) => existing.id !== item.id));
    } catch {
      setError(t('collection.removeError'));
    } finally {
      setDeletingId(null);
    }
  }

  async function handleExport() {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const doc = await exportCollection();
      const blob = new Blob([JSON.stringify(doc, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const today = new Date().toISOString().slice(0, 10);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `coleccion-pokemon-${today}.json`;
      document.body.appendChild(anchor);
      anchor.click();
      anchor.remove();
      URL.revokeObjectURL(url);
      setNotice(
        doc.count === 1
          ? t('collection.exported.one')
          : t('collection.exported.other', { count: doc.count }),
      );
    } catch {
      setError(t('collection.exportError'));
    } finally {
      setBusy(false);
    }
  }

  function handleImportClick() {
    setError(null);
    setNotice(null);
    fileInputRef.current?.click();
  }

  async function handleFileChange(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    // Permite volver a elegir el mismo fichero mas tarde.
    event.target.value = '';
    if (!file) return;

    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const text = await file.text();
      const parsed: unknown = JSON.parse(text);
      // Acepta el documento exportado ({items:[...]}) o un array suelto.
      const list = Array.isArray(parsed)
        ? parsed
        : (parsed as { items?: unknown }).items;
      if (!Array.isArray(list)) {
        setError(t('collection.importInvalid'));
        return;
      }

      const replace = window.confirm(t('collection.importConfirm'));
      const summary = await importCollection(list, replace ? 'replace' : 'merge');
      await load();

      const parts = [
        t('collection.import.added', { count: summary.imported }),
        t('collection.import.updated', { count: summary.updated }),
      ];
      if (summary.skipped.length > 0) {
        parts.push(t('collection.import.skipped', { count: summary.skipped.length }));
      }
      const modeLabel = t(`collection.mode.${summary.mode}` as TranslationKey);
      setNotice(t('collection.import.summary', { mode: modeLabel, parts: parts.join(' · ') }));
    } catch (err) {
      if (err instanceof SyntaxError) {
        setError(t('collection.importNotJson'));
      } else {
        setError(t('collection.importError'));
      }
    } finally {
      setBusy(false);
    }
  }

  function formatDate(iso: string): string {
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) {
      return iso;
    }
    return date.toLocaleDateString(intlLocale(locale), {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    });
  }

  return (
    <div className="page collection-page">
      <header className="page-header">
        <h1>{t('collection.title')}</h1>
        <p className="page-subtitle">
          {items.length === 1
            ? t('collection.count.one')
            : t('collection.count.other', { count: items.length })}
        </p>
      </header>

      <div className="collection-actions">
        <button
          type="button"
          className="btn btn-secondary btn-small"
          onClick={() => void handleExport()}
          disabled={busy || items.length === 0}
        >
          {t('collection.export')}
        </button>
        <button
          type="button"
          className="btn btn-secondary btn-small"
          onClick={handleImportClick}
          disabled={busy}
        >
          {t('collection.import')}
        </button>
        <input
          ref={fileInputRef}
          type="file"
          accept="application/json,.json"
          onChange={(event) => void handleFileChange(event)}
          hidden
        />
      </div>

      {notice && (
        <div className="info-banner" role="status">
          {notice}
        </div>
      )}

      {loading && (
        <div className="loading" role="status">
          <div className="spinner" aria-hidden="true" />
          <p>{t('collection.loading')}</p>
        </div>
      )}

      {error && (
        <div className="error-banner" role="alert">
          {error}
        </div>
      )}

      {!loading && !error && items.length === 0 && (
        <div className="empty-state">
          <p>{t('collection.empty')}</p>
          <Link to="/" className="btn btn-primary">
            {t('collection.emptyCta')}
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
                {t('common.setNumber', {
                  set: item.card.set_name ?? item.card.set_id,
                  number: item.card.number,
                })}
              </span>
              <span className="collection-meta">
                {t('collection.quantityAdded', {
                  qty: item.quantity,
                  date: formatDate(item.created_at),
                })}
              </span>
            </div>
            <button
              type="button"
              className="btn btn-danger btn-small"
              onClick={() => void handleDelete(item)}
              disabled={deletingId === item.id}
              aria-label={t('collection.removeAria', { name: item.card.name })}
            >
              {deletingId === item.id ? '…' : t('collection.remove')}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
