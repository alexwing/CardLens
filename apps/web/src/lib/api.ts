/**
 * Cliente HTTP tipado contra la API publica (Rust axum).
 * Base configurable con VITE_API_URL (default http://localhost:8787).
 */
import type {
  AddCollectionItemPayload,
  Card,
  CardListResponse,
  CardQueryParams,
  CollectionItem,
  CollectionResponse,
  HealthResponse,
  PricesResponse,
  ScanResponse,
  SetInfo,
} from './types';

export const API_BASE: string = import.meta.env.VITE_API_URL || 'http://localhost:8787';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, init);
  if (!response.ok) {
    throw new Error(`Error de la API (${response.status}) en ${path}`);
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

/** Precios cacheados de una carta. */
export function getPrices(cardId: string): Promise<PricesResponse> {
  return request<PricesResponse>(`/api/prices/${encodeURIComponent(cardId)}`);
}

/** Estado de la API y del servicio ML. */
export function health(): Promise<HealthResponse> {
  return request<HealthResponse>('/api/health');
}

/**
 * URL de imagen para una carta: prioriza la copia local servida por la API
 * (/images/*) y cae a la URL oficial remota si no hay copia local.
 */
export function imageSrc(card: Card): string {
  if (card.image_local) {
    const path = card.image_local.startsWith('/') ? card.image_local : `/${card.image_local}`;
    return `${API_BASE}${path}`;
  }
  return card.image_url ?? '';
}
