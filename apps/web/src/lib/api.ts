/**
 * Cliente HTTP tipado contra la API publica (Rust axum).
 * Base configurable con VITE_API_URL (default http://localhost:8787).
 */
import type {
  AddCollectionItemPayload,
  Card,
  CardListResponse,
  CardQueryParams,
  CollectionExport,
  CollectionImportSummary,
  CollectionItem,
  CollectionResponse,
  HealthResponse,
  PricesResponse,
  ScanResponse,
  SetInfo,
  TagRef,
  TagWithCount,
  UpdateCollectionItemPayload,
} from './types';

export const API_BASE: string = import.meta.env.VITE_API_URL || 'http://localhost:8787';

/** Error de la API que conserva el codigo HTTP para que el llamante lo distinga. */
export class ApiRequestError extends Error {
  status: number;
  constructor(status: number, path: string) {
    super(`Error de la API (${status}) en ${path}`);
    this.name = 'ApiRequestError';
    this.status = status;
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, init);
  if (!response.ok) {
    throw new ApiRequestError(response.status, path);
  }
  if (response.status === 204) {
    return undefined as T;
  }
  return (await response.json()) as T;
}

/** Envia una foto al endpoint de escaneo (multipart, campo "image"). */
export function scanImage(blob: Blob): Promise<ScanResponse> {
  const form = new FormData();
  form.append('image', blob, 'scan.jpg');
  return request<ScanResponse>('/api/scan', { method: 'POST', body: form });
}

/** Busca cartas del catalogo con filtros y paginacion. */
export function getCards(params: CardQueryParams = {}): Promise<CardListResponse> {
  const search = new URLSearchParams();
  if (params.q) search.set('q', params.q);
  if (params.set_id) search.set('set_id', params.set_id);
  if (params.page !== undefined) search.set('page', String(params.page));
  if (params.page_size !== undefined) search.set('page_size', String(params.page_size));
  const qs = search.toString();
  return request<CardListResponse>(`/api/cards${qs ? `?${qs}` : ''}`);
}

/** Detalle de una carta por id. */
export function getCard(id: string): Promise<Card> {
  return request<Card>(`/api/cards/${encodeURIComponent(id)}`);
}

/** Lista de sets del catalogo. */
export function getSets(): Promise<SetInfo[]> {
  return request<SetInfo[]>('/api/sets');
}

/** Coleccion completa del usuario. */
export function getCollection(): Promise<CollectionResponse> {
  return request<CollectionResponse>('/api/collection');
}

/** Anade un item a la coleccion. Devuelve el item creado (201). */
export function addToCollection(payload: AddCollectionItemPayload): Promise<CollectionItem> {
  return request<CollectionItem>('/api/collection/items', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
}

/** Elimina un item de la coleccion (204). */
export function removeFromCollection(id: string): Promise<void> {
  return request<void>(`/api/collection/items/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

/** Actualiza un item de la coleccion (nota, cantidad, estado). Devuelve el item. */
export function updateCollectionItem(
  id: string,
  payload: UpdateCollectionItemPayload,
): Promise<CollectionItem> {
  return request<CollectionItem>(`/api/collection/items/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });
}

/** Descarga la coleccion como documento JSON portable y versionado. */
export function exportCollection(): Promise<CollectionExport> {
  return request<CollectionExport>('/api/collection/export');
}

/**
 * Importa items de coleccion. `mode`: 'merge' (actualiza por card_id) o
 * 'replace' (vacia y rellena). Devuelve un resumen con omitidos.
 */
export function importCollection(
  items: unknown,
  mode: 'merge' | 'replace' = 'merge',
): Promise<CollectionImportSummary> {
  return request<CollectionImportSummary>('/api/collection/import', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ mode, items }),
  });
}

/** Lista de etiquetas con el numero de items que usan cada una. */
export function getTags(): Promise<TagWithCount[]> {
  return request<TagWithCount[]>('/api/tags');
}

/** Crea una etiqueta (o devuelve la existente, case-insensitive). */
export function createTag(name: string): Promise<TagRef> {
  return request<TagRef>('/api/tags', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name }),
  });
}

/** Elimina una etiqueta y todas sus asociaciones (cascade). */
export function deleteTag(id: string): Promise<void> {
  return request<void>(`/api/tags/${encodeURIComponent(id)}`, { method: 'DELETE' });
}

/** Asocia una etiqueta (por nombre) a un item; crea la tag si no existe. */
export function addItemTag(itemId: string, name: string): Promise<TagRef> {
  return request<TagRef>(`/api/collection/items/${encodeURIComponent(itemId)}/tags`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name }),
  });
}

/** Quita la asociacion de una etiqueta con un item (no borra la tag). */
export function removeItemTag(itemId: string, tagId: string): Promise<void> {
  return request<void>(
    `/api/collection/items/${encodeURIComponent(itemId)}/tags/${encodeURIComponent(tagId)}`,
    { method: 'DELETE' },
  );
}

/** Precios cacheados de una carta. */
export function getPrices(cardId: string): Promise<PricesResponse> {
  return request<PricesResponse>(`/api/prices/${encodeURIComponent(cardId)}`);
}

/** Estado de la API y del servicio ML. */
export function health(): Promise<HealthResponse> {
  return request<HealthResponse>('/api/health');
}

/**
 * URL de imagen para una carta. Orden de preferencia:
 *  1. VITE_IMAGE_BASE (tu CDN, p. ej. https://cardlens.mappuzzle.xyz/catalog):
 *     se construye `${base}/${card_id}.png`. Para self-hosting de las imagenes.
 *  2. image_url remota (TCGdex): fiable y disponible siempre (online). Es lo
 *     que usa el ejecutable empaquetado, que no lleva imagenes locales.
 *  3. image_local servida por la API (/images/*): util en desarrollo local.
 */
export function imageSrc(card: Card): string {
  const cdn = import.meta.env.VITE_IMAGE_BASE as string | undefined;
  if (cdn && cdn.trim()) {
    return `${cdn.replace(/\/+$/, '')}/${encodeURIComponent(card.id)}.png`;
  }
  if (card.image_url) {
    return card.image_url;
  }
  if (card.image_local) {
    const path = card.image_local.startsWith('/') ? card.image_local : `/${card.image_local}`;
    return `${API_BASE}${path}`;
  }
  return '';
}
