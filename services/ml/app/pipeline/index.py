"""Indice FAISS de cartas (IndexFlatIP, vectores L2-normalizados = coseno).

Carga lazy de ``data/index/faiss.index`` y ``data/index/cards.json``
(lista alineada fila a fila con objetos {card_id, name, number, set_id, lang}).

Si el indice no existe o ``faiss`` no esta instalado, el modulo degrada:
``loaded()`` devuelve False y ``search`` devuelve lista vacia.
"""

from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Optional

import numpy as np

from app.config import get_settings

logger = logging.getLogger("ml.index")


class CardIndex:
    """Acceso de solo lectura al indice FAISS y su metadata alineada."""

    def __init__(self, index_path: Path, cards_path: Path) -> None:
        self._index_path = index_path
        self._cards_path = cards_path
        self._index = None
        self._cards: list[dict] = []
        self._load_attempted = False

    def _ensure_loaded(self) -> None:
        if self._load_attempted:
            return
        self._load_attempted = True

        if not self._index_path.exists() or not self._cards_path.exists():
            logger.warning(
                "Indice FAISS no encontrado (%s): /analyze respondera sin candidatos. "
                "Genera el indice con ingest/build_index.py",
                self._index_path,
            )
            return
        try:
            import faiss
        except ImportError:
            logger.warning("faiss-cpu no esta instalado: busqueda visual deshabilitada")
            return
        try:
            index = faiss.read_index(str(self._index_path))
            cards = json.loads(self._cards_path.read_text(encoding="utf-8"))
            if int(index.ntotal) != len(cards):
                logger.warning(
                    "Desalineacion indice/metadata: %s vectores vs %s cartas",
                    index.ntotal,
                    len(cards),
                )
            self._index = index
            self._cards = cards
            logger.info("Indice FAISS cargado: %s vectores", index.ntotal)
        except Exception:
            logger.exception("No se pudo cargar el indice FAISS")
            self._index = None
            self._cards = []

    def loaded(self) -> bool:
        self._ensure_loaded()
        return self._index is not None

    def size(self) -> int:
        self._ensure_loaded()
        return int(self._index.ntotal) if self._index is not None else 0

    def search(self, vec: np.ndarray, k: int) -> list[tuple[dict, float]]:
        """Busca los k vecinos mas cercanos. Devuelve [(card_meta, score)]."""
        if not self.loaded():
            return []
        total = int(self._index.ntotal)
        if total == 0:
            return []
        k = min(int(k), total)
        query = np.asarray(vec, dtype=np.float32).reshape(1, -1)
        scores, indices = self._index.search(query, k)
        results: list[tuple[dict, float]] = []
        for row_id, score in zip(indices[0], scores[0]):
            row_id = int(row_id)
            if row_id < 0 or row_id >= len(self._cards):
                continue
            results.append((self._cards[row_id], float(score)))
        return results


_index_singleton: Optional[CardIndex] = None


def get_index() -> CardIndex:
    """Singleton lazy del indice de cartas."""
    global _index_singleton
    if _index_singleton is None:
        settings = get_settings()
        _index_singleton = CardIndex(settings.index_path, settings.cards_json_path)
    return _index_singleton
