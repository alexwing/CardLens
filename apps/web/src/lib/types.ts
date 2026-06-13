/**
 * Tipos espejo del contrato de la API publica (Rust axum, prefijo /api).
 * Cualquier cambio aqui debe ir acompanado del cambio equivalente en la API.
 */

/** Carta del catalogo tal como la devuelve la API. */
export interface Card {
  id: string;
  set_id: string;
  name: string;
  number: string;
  rarity: string | null;
  supertype: string | null;
  lang: string;
  image_url: string | null;
  image_local: string | null;
  set_name: string | null;
}

/** Candidato de un escaneo, ya enriquecido con la metadata de la carta. */
export interface ScanCandidateView {
  card: Card;
  confidence: number;
  visual_score: number;
  ocr_score: number;
}

/** Respuesta de POST /api/scan. */
export interface ScanResponse {
  scan_id: string;
  low_confidence: boolean;
  best: {
    card: Card;
    confidence: number;
  } | null;
  candidates: ScanCandidateView[];
}

/** Elemento de GET /api/sets. */
export interface SetInfo {
  id: string;
  name: string;
  series: string | null;
  code: string | null;
  release_date: string | null;
  total: number | null;
  lang: string;
}

/** Elemento de la coleccion (GET /api/collection). */
export interface CollectionItem {
  id: string;
  card: Card;
  quantity: number;
  condition: string | null;
  lang: string | null;
  notes: string | null;
  created_at: string;
}

/** Cotizacion de precio de una fuente (GET /api/prices/{card_id}). */
export interface PriceQuote {
  source: string;
  currency: string;
  market: number | null;
  low: number | null;
  high: number | null;
  trend: number | null;
  updated_at: string;
}

/** Respuesta de GET /api/health. */
export interface HealthResponse {
  status: string;
  ml: {
    reachable: boolean;
  };
}

/** Respuesta paginada de GET /api/cards. */
export interface CardListResponse {
  items: Card[];
  total: number;
  page: number;
}

/** Respuesta de GET /api/collection. */
export interface CollectionResponse {
  items: CollectionItem[];
}

/** Respuesta de GET /api/prices/{card_id}. */
export interface PricesResponse {
  card_id: string;
  prices: PriceQuote[];
}

/** Cuerpo de POST /api/collection/items. */
export interface AddCollectionItemPayload {
  card_id: string;
  scan_id: string | null;
  quantity: number;
  condition: string | null;
  lang: string | null;
  notes: string | null;
}

/** Parametros de consulta de GET /api/cards. */
export interface CardQueryParams {
  q?: string;
  set_id?: string;
  page?: number;
  page_size?: number;
}
