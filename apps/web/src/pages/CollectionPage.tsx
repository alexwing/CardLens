import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import type { CollectionItem, TagWithCount } from '../lib/types';
import {
  addItemTag,
  exportCollection,
  getCollection,
  getTags,
  imageSrc,
  importCollection,
  removeFromCollection,
  removeItemTag,
} from '../lib/api';
import { intlLocale, useT } from '../lib/i18n';
import type { TranslationKey } from '../lib/locales/es';

/**
 * Pagina de coleccion: lista los items guardados con miniatura, nombre,
 * set, cantidad, fecha y etiquetas; permite eliminarlos, etiquetarlos,
 * filtrarlos (en cliente) y exportar/importar la coleccion en JSON.
 */
export default function CollectionPage() {
  const { t, locale } = useT();
  const [items, setItems] = useState<CollectionItem[]>([]);
  const [tags, setTags] = useState<TagWithCount[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  // Estado de los filtros (todo el filtrado ocurre en el cliente).
  const [search, setSearch] = useState('');
  const [activeTagIds, setActiveTagIds] = useState<string[]>([]);
  const [setFilter, setSetFilter] = useState('');
  const [rarityFilter, setRarityFilter] = useState('');
  const [typeFilter, setTypeFilter] = useState('');
  const [langFilter, setLangFilter] = useState('');

  // Item cuyo control "+ etiqueta" esta abierto, y el texto que se escribe.
  const [tagEditorItemId, setTagEditorItemId] = useState<string | null>(null);
  const [tagDraft, setTagDraft] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [collection, tagList] = await Promise.all([getCollection(), getTags()]);
      setItems(collection.items);
      setTags(tagList);
    } catch {
      setError(t('collection.loadError'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    void load();
  }, [load]);

  // Refresca solo la lista de tags (tras añadir/quitar) para mantener counts.
  const refreshTags = useCallback(async () => {
    try {
      setTags(await getTags());
    } catch {
      /* el conteo se reajustara en la siguiente carga completa */
    }
  }, []);

  // Valores unicos presentes en la coleccion para poblar los selects.
  const setOptions = useMemo(() => {
    const map = new Map<string, string>();
    for (const item of items) {
      const value = item.card.set_id;
      if (value) map.set(value, item.card.set_name ?? value);
    }
    return [...map.entries()]
      .map(([value, label]) => ({ value, label }))
      .sort((a, b) => a.label.localeCompare(b.label, intlLocale(locale)));
  }, [items, locale]);

  const rarityOptions = useMemo(
    () => uniqueSorted(items.map((item) => item.card.rarity), locale),
    [items, locale],
  );
  const typeOptions = useMemo(
    () => uniqueSorted(items.map((item) => item.card.supertype), locale),
    [items, locale],
  );
  const langOptions = useMemo(
    () => uniqueSorted(items.map((item) => item.card.lang), locale),
    [items, locale],
  );

  // Filtrado 100% en cliente: un item pasa si cumple TODOS los filtros activos;
  // para etiquetas, debe tener TODAS las seleccionadas.
  const filteredItems = useMemo(() => {
    const query = search.trim().toLowerCase();
    return items.filter((item) => {
      if (query && !item.card.name.toLowerCase().includes(query)) return false;
      if (setFilter && item.card.set_id !== setFilter) return false;
      if (rarityFilter && item.card.rarity !== rarityFilter) return false;
      if (typeFilter && item.card.supertype !== typeFilter) return false;
      if (langFilter && item.card.lang !== langFilter) return false;
      if (activeTagIds.length > 0) {
        const itemTagIds = new Set(item.tags.map((tag) => tag.id));
        if (!activeTagIds.every((id) => itemTagIds.has(id))) return false;
      }
      return true;
    });
  }, [items, search, setFilter, rarityFilter, typeFilter, langFilter, activeTagIds]);

  const hasActiveFilters =
    search.trim() !== '' ||
    activeTagIds.length > 0 ||
    setFilter !== '' ||
    rarityFilter !== '' ||
    typeFilter !== '' ||
    langFilter !== '';

  function toggleTagFilter(tagId: string) {
    setActiveTagIds((current) =>
      current.includes(tagId)
        ? current.filter((id) => id !== tagId)
        : [...current, tagId],
    );
  }

  function clearFilters() {
    setSearch('');
    setActiveTagIds([]);
    setSetFilter('');
    setRarityFilter('');
    setTypeFilter('');
    setLangFilter('');
  }

  async function handleDelete(item: CollectionItem) {
    const ok = window.confirm(t('collection.removeConfirm', { name: item.card.name }));
    if (!ok) return;
    setDeletingId(item.id);
    try {
      await removeFromCollection(item.id);
      setItems((current) => current.filter((existing) => existing.id !== item.id));
      void refreshTags();
    } catch {
      setError(t('collection.removeError'));
    } finally {
      setDeletingId(null);
    }
  }

  async function handleAddTag(item: CollectionItem) {
    const name = tagDraft.trim();
    if (!name) return;
    setError(null);
    try {
      const tag = await addItemTag(item.id, name);
      setItems((current) =>
        current.map((existing) =>
          existing.id === item.id && !existing.tags.some((existingTag) => existingTag.id === tag.id)
            ? { ...existing, tags: [...existing.tags, tag] }
            : existing,
        ),
      );
      setTagDraft('');
      setTagEditorItemId(null);
      void refreshTags();
    } catch {
      setError(t('tags.addError'));
    }
  }

  async function handleRemoveTag(item: CollectionItem, tagId: string) {
    setError(null);
    try {
      await removeItemTag(item.id, tagId);
      setItems((current) =>
        current.map((existing) =>
          existing.id === item.id
            ? { ...existing, tags: existing.tags.filter((tag) => tag.id !== tagId) }
            : existing,
        ),
      );
      // Si la tag estaba activa como filtro y ya no la tiene nadie, igualmente
      // se queda; el filtro se mantiene hasta que el usuario lo desactive.
      void refreshTags();
    } catch {
      setError(t('tags.removeError'));
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

      {!loading && !error && items.length > 0 && (
        <>
          <div className="filters">
            <div className="filters-row">
              <label className="filters-search">
                <span className="visually-hidden">{t('filters.search')}</span>
                <input
                  type="search"
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  placeholder={t('filters.searchPlaceholder')}
                  aria-label={t('filters.search')}
                />
              </label>
              <button
                type="button"
                className="btn btn-secondary btn-small filters-clear"
                onClick={clearFilters}
                disabled={!hasActiveFilters}
              >
                {t('filters.clear')}
              </button>
            </div>

            {tags.length > 0 && (
              <div className="filters-tags" role="group" aria-label={t('filters.tags')}>
                <button
                  type="button"
                  className={`tag-chip filter-chip${activeTagIds.length === 0 ? ' active' : ''}`}
                  onClick={() => setActiveTagIds([])}
                  aria-pressed={activeTagIds.length === 0}
                >
                  {t('filters.all')}
                </button>
                {tags.map((tag) => (
                  <button
                    key={tag.id}
                    type="button"
                    className={`tag-chip filter-chip${activeTagIds.includes(tag.id) ? ' active' : ''}`}
                    onClick={() => toggleTagFilter(tag.id)}
                    aria-pressed={activeTagIds.includes(tag.id)}
                  >
                    {tag.name} <span className="tag-count">{tag.count}</span>
                  </button>
                ))}
              </div>
            )}

            <div className="filters-selects">
              <select
                value={setFilter}
                onChange={(event) => setSetFilter(event.target.value)}
                aria-label={t('filters.set')}
              >
                <option value="">{t('filters.allSets')}</option>
                {setOptions.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
              <select
                value={rarityFilter}
                onChange={(event) => setRarityFilter(event.target.value)}
                aria-label={t('filters.rarity')}
              >
                <option value="">{t('filters.allRarities')}</option>
                {rarityOptions.map((value) => (
                  <option key={value} value={value}>
                    {value}
                  </option>
                ))}
              </select>
              <select
                value={typeFilter}
                onChange={(event) => setTypeFilter(event.target.value)}
                aria-label={t('filters.type')}
              >
                <option value="">{t('filters.allTypes')}</option>
                {typeOptions.map((value) => (
                  <option key={value} value={value}>
                    {value}
                  </option>
                ))}
              </select>
              <select
                value={langFilter}
                onChange={(event) => setLangFilter(event.target.value)}
                aria-label={t('filters.lang')}
              >
                <option value="">{t('filters.allLangs')}</option>
                {langOptions.map((value) => (
                  <option key={value} value={value}>
                    {value}
                  </option>
                ))}
              </select>
            </div>

            <p className="filters-result-count">
              {filteredItems.length === 1
                ? t('filters.resultCount.one')
                : t('filters.resultCount.other', { count: filteredItems.length })}
            </p>
          </div>

          {filteredItems.length === 0 ? (
            <div className="empty-state">
              <p>{t('filters.noResults')}</p>
            </div>
          ) : (
            <ul className="collection-list">
              {filteredItems.map((item) => (
                <li key={item.id} className="collection-item">
                  <Link
                    to={`/carta/${encodeURIComponent(item.card.id)}`}
                    className="collection-thumb-link"
                  >
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
                    <Link
                      to={`/carta/${encodeURIComponent(item.card.id)}`}
                      className="collection-name"
                    >
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
                    <div className="collection-tags">
                      {item.tags.map((tag) => (
                        <span key={tag.id} className="tag-chip">
                          {tag.name}
                          <button
                            type="button"
                            className="tag-chip-remove"
                            onClick={() => void handleRemoveTag(item, tag.id)}
                            aria-label={t('tags.removeAria', { name: tag.name })}
                          >
                            ×
                          </button>
                        </span>
                      ))}
                      {tagEditorItemId === item.id ? (
                        <span className="tag-editor">
                          <input
                            type="text"
                            list="collection-tag-options"
                            value={tagDraft}
                            onChange={(event) => setTagDraft(event.target.value)}
                            onKeyDown={(event) => {
                              if (event.key === 'Enter') {
                                event.preventDefault();
                                void handleAddTag(item);
                              } else if (event.key === 'Escape') {
                                setTagEditorItemId(null);
                                setTagDraft('');
                              }
                            }}
                            placeholder={t('tags.addPlaceholder')}
                            aria-label={t('tags.add')}
                            autoFocus
                          />
                          <button
                            type="button"
                            className="tag-chip-add"
                            onClick={() => void handleAddTag(item)}
                          >
                            {t('tags.addConfirm')}
                          </button>
                          <button
                            type="button"
                            className="tag-chip-cancel"
                            onClick={() => {
                              setTagEditorItemId(null);
                              setTagDraft('');
                            }}
                          >
                            {t('tags.cancel')}
                          </button>
                        </span>
                      ) : (
                        <button
                          type="button"
                          className="tag-chip tag-chip-add-toggle"
                          onClick={() => {
                            setTagEditorItemId(item.id);
                            setTagDraft('');
                          }}
                        >
                          {t('tags.add')}
                        </button>
                      )}
                    </div>
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
          )}

          {/* Sugerencias compartidas para todos los inputs "+ etiqueta". */}
          <datalist id="collection-tag-options">
            {tags.map((tag) => (
              <option key={tag.id} value={tag.name} />
            ))}
          </datalist>
        </>
      )}
    </div>
  );
}

/** Valores distintos no nulos, ordenados segun el locale. */
function uniqueSorted(values: (string | null)[], locale: 'es' | 'en'): string[] {
  const set = new Set<string>();
  for (const value of values) {
    if (value) set.add(value);
  }
  return [...set].sort((a, b) => a.localeCompare(b, intlLocale(locale)));
}
