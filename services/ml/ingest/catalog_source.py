"""Conector de catalogo desacoplado.

El resto de la ingesta depende solo de la interfaz :class:`CatalogSource`;
:class:`TCGdexSource` es la implementacion concreta contra la API publica
TCGdex v2 (gratuita, sin API key, multiidioma).
"""

from __future__ import annotations

import logging
import time
from abc import ABC, abstractmethod
from typing import Optional

import httpx

logger = logging.getLogger("ingest.catalog")


class CatalogSource(ABC):
    """Interfaz abstracta de una fuente de catalogo de cartas."""

    @abstractmethod
    def list_sets(self, lang: str) -> list[dict]:
        """Lista resumida de sets disponibles en un idioma."""

    @abstractmethod
    def get_set(self, set_id: str, lang: str) -> Optional[dict]:
        """Detalle de un set (incluida su lista de cartas) o None si falla."""

    @abstractmethod
    def get_card(self, card_id: str, lang: str) -> Optional[dict]:
        """Detalle completo de una carta o None si falla."""

    @abstractmethod
    def image_url(self, card: dict, quality: str = "low") -> Optional[str]:
        """URL de la imagen de una carta en la calidad pedida ('low'/'high')."""


class TCGdexSource(CatalogSource):
    """Implementacion contra https://api.tcgdex.net/v2 (httpx sincrono)."""

    BASE_URL = "https://api.tcgdex.net/v2"
    RETRIES = 3
    RETRY_PAUSE_S = 2.0

    def __init__(self, timeout: float = 30.0) -> None:
        self._client = httpx.Client(
            timeout=timeout,
            follow_redirects=True,
            headers={"User-Agent": "PokemonCardDetector/0.1 (ingesta local)"},
        )

    def _get_json(self, path: str) -> Optional[dict | list]:
        url = f"{self.BASE_URL}{path}"
        for attempt in range(1, self.RETRIES + 1):
            try:
                response = self._client.get(url)
                response.raise_for_status()
                return response.json()
            except Exception as exc:
                logger.warning(
                    "Fallo GET %s (intento %s/%s): %s", url, attempt, self.RETRIES, exc
                )
                if attempt < self.RETRIES:
                    time.sleep(self.RETRY_PAUSE_S * attempt)
        return None

    def list_sets(self, lang: str) -> list[dict]:
        data = self._get_json(f"/{lang}/sets")
        return data if isinstance(data, list) else []

    def get_set(self, set_id: str, lang: str) -> Optional[dict]:
        data = self._get_json(f"/{lang}/sets/{set_id}")
        return data if isinstance(data, dict) else None

    def get_card(self, card_id: str, lang: str) -> Optional[dict]:
        data = self._get_json(f"/{lang}/cards/{card_id}")
        return data if isinstance(data, dict) else None

    def image_url(self, card: dict, quality: str = "low") -> Optional[str]:
        base = (card or {}).get("image")
        if not base:
            return None
        return f"{base}/{quality}.png"

    def close(self) -> None:
        self._client.close()

    def __enter__(self) -> "TCGdexSource":
        return self

    def __exit__(self, exc_type, exc_value, traceback) -> None:
        self.close()
