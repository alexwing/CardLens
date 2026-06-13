"""Ingesta del catalogo de cartas a la base SQLite compartida.

Uso (desde services/ml, con el venv activado):
    python -m ingest.ingest_catalog --langs en es --sets swsh3 base1
    python -m ingest.ingest_catalog --langs en --all

Requisito previo: la API Rust debe haber creado el esquema
(``cargo run`` en services/api genera data/app.db con sus migraciones).

Convenciones de IDs del contrato:
- card_id = tcgdexId + '_' + lang   (p. ej. swsh3-136_en)
- set_id  = tcgdexSetId + '_' + lang
"""

from __future__ import annotations

import argparse
import json
import logging
import sqlite3
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

import httpx

SERVICE_ROOT = Path(__file__).resolve().parents[1]
if str(SERVICE_ROOT) not in sys.path:
    sys.path.insert(0, str(SERVICE_ROOT))

from app.config import get_settings  # noqa: E402
from ingest.catalog_source import CatalogSource, TCGdexSource  # noqa: E402

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("ingest.catalog")

SCHEMA_HINT = (
    "Arranca antes la API Rust (cargo run en services/api) para crear el esquema"
)

# Pausa de cortesia entre peticiones a la API externa.
REQUEST_PAUSE_S = 0.2
DOWNLOAD_RETRIES = 3
DOWNLOAD_RETRY_PAUSE_S = 2.0


def utc_now_iso() -> str:
    return (
        datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
    )


def check_database(db_path: Path) -> None:
    """Comprueba que data/app.db existe y tiene la tabla cards; si no, sale con 1."""
    if not db_path.exists():
        logger.error("No existe la base de datos %s. %s", db_path, SCHEMA_HINT)
        sys.exit(1)
    try:
        connection = sqlite3.connect(db_path)
        try:
            row = connection.execute(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'cards'"
            ).fetchone()
        finally:
            connection.close()
    except sqlite3.Error as exc:
        logger.error("No se pudo abrir %s (%s). %s", db_path, exc, SCHEMA_HINT)
        sys.exit(1)
    if row is None:
        logger.error("La base %s no tiene la tabla 'cards'. %s", db_path, SCHEMA_HINT)
        sys.exit(1)


def asset_url(base: Optional[str], extension: str = "png") -> Optional[str]:
    """TCGdex devuelve URLs de assets sin extension: se anade '.png'."""
    if not base:
        return None
    if base.endswith((".png", ".jpg", ".webp")):
        return base
    return f"{base}.{extension}"


def download_image(client: httpx.Client, url: str, destination: Path) -> bool:
    """Descarga una imagen con reintentos. Devuelve True si el archivo existe."""
    if destination.exists():
        return True
    destination.parent.mkdir(parents=True, exist_ok=True)
    for attempt in range(1, DOWNLOAD_RETRIES + 1):
        try:
            response = client.get(url)
            response.raise_for_status()
            destination.write_bytes(response.content)
            return True
        except Exception as exc:
            logger.warning(
                "Fallo la descarga %s (intento %s/%s): %s",
                url,
                attempt,
                DOWNLOAD_RETRIES,
                exc,
            )
            if attempt < DOWNLOAD_RETRIES:
                time.sleep(DOWNLOAD_RETRY_PAUSE_S * attempt)
    return False


def upsert_set(connection: sqlite3.Connection, detail: dict, lang: str) -> str:
    """Inserta/actualiza un set. Devuelve el set_id con sufijo de idioma."""
    set_row_id = f"{detail.get('id')}_{lang}"
    serie = detail.get("serie") or {}
    series_name = serie.get("name") if isinstance(serie, dict) else str(serie)
    card_count = detail.get("cardCount") or {}
    total = None
    if isinstance(card_count, dict):
        total = card_count.get("official") or card_count.get("total")
    connection.execute(
        """
        INSERT OR REPLACE INTO sets
            (id, name, series, code, release_date, total, lang, symbol_url, logo_url)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            set_row_id,
            detail.get("name") or detail.get("id") or "",
            series_name,
            detail.get("tcgOnline"),
            detail.get("releaseDate"),
            total,
            lang,
            asset_url(detail.get("symbol")),
            asset_url(detail.get("logo")),
        ),
    )
    return set_row_id


def upsert_card(
    connection: sqlite3.Connection,
    card: dict,
    set_row_id: str,
    lang: str,
    image_url: Optional[str],
    image_local: Optional[str],
) -> str:
    """Inserta/actualiza una carta. Devuelve el card_id con sufijo de idioma."""
    card_id = f"{card.get('id')}_{lang}"
    subtypes_values = [value for value in (card.get("stage"), card.get("suffix")) if value]
    subtypes = (
        json.dumps(subtypes_values, ensure_ascii=False) if subtypes_values else None
    )
    connection.execute(
        """
        INSERT OR REPLACE INTO cards
            (id, set_id, name, number, rarity, supertype, subtypes, lang,
             image_url, image_local, illustrator, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """,
        (
            card_id,
            set_row_id,
            card.get("name") or "",
            str(card.get("localId") or ""),
            card.get("rarity"),
            card.get("category"),
            subtypes,
            lang,
            image_url,
            image_local,
            card.get("illustrator"),
            utc_now_iso(),
        ),
    )
    return card_id


def ingest_set(
    source: CatalogSource,
    connection: sqlite3.Connection,
    http_client: httpx.Client,
    images_dir: Path,
    tcgdex_set_id: str,
    lang: str,
) -> tuple[int, int]:
    """Ingesta un set completo. Devuelve (cartas insertadas, imagenes descargadas)."""
    detail = source.get_set(tcgdex_set_id, lang)
    if not detail:
        logger.warning("Set %s (%s) no disponible; se omite", tcgdex_set_id, lang)
        return 0, 0

    set_row_id = upsert_set(connection, detail, lang)
    connection.commit()

    briefs = detail.get("cards") or []
    logger.info("Set %s (%s): %s cartas", set_row_id, detail.get("name"), len(briefs))

    cards_done = 0
    images_done = 0
    for brief in briefs:
        tcgdex_card_id = brief.get("id")
        if not tcgdex_card_id:
            continue
        time.sleep(REQUEST_PAUSE_S)
        card = source.get_card(tcgdex_card_id, lang) or brief
        card_id = f"{tcgdex_card_id}_{lang}"

        image_local = None
        low_url = source.image_url(card, "low")
        if low_url:
            destination = images_dir / f"{card_id}.png"
            if download_image(http_client, low_url, destination):
                image_local = f"/images/{card_id}.png"
            time.sleep(REQUEST_PAUSE_S)

        upsert_card(
            connection,
            card,
            set_row_id,
            lang,
            image_url=source.image_url(card, "high"),
            image_local=image_local,
        )
        connection.commit()
        cards_done += 1
        if image_local:
            images_done += 1
    return cards_done, images_done


def parse_args(argv: Optional[list[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Ingesta el catalogo TCGdex en data/app.db y descarga imagenes."
    )
    parser.add_argument(
        "--langs",
        nargs="+",
        default=["en"],
        help="Idiomas a ingestar (en, es, fr, de, it, pt). Default: en",
    )
    target = parser.add_mutually_exclusive_group(required=True)
    target.add_argument(
        "--sets",
        nargs="+",
        metavar="SET_ID",
        help="IDs tcgdex de sets separados por espacio (p. ej. swsh3 base1)",
    )
    target.add_argument(
        "--all",
        action="store_true",
        help="Ingesta todos los sets disponibles en cada idioma",
    )
    return parser.parse_args(argv)


def main(argv: Optional[list[str]] = None) -> int:
    args = parse_args(argv)
    settings = get_settings()

    check_database(settings.db_path)
    settings.images_dir.mkdir(parents=True, exist_ok=True)

    connection = sqlite3.connect(settings.db_path)
    connection.execute("PRAGMA journal_mode = WAL")
    connection.execute("PRAGMA foreign_keys = ON")

    total_cards = 0
    total_images = 0
    try:
        with TCGdexSource() as source, httpx.Client(
            timeout=60.0,
            follow_redirects=True,
            headers={"User-Agent": "PokemonCardDetector/0.1 (ingesta local)"},
        ) as http_client:
            for lang in args.langs:
                if args.all:
                    set_ids = [
                        item.get("id")
                        for item in source.list_sets(lang)
                        if item.get("id")
                    ]
                    logger.info("Idioma %s: %s sets encontrados", lang, len(set_ids))
                else:
                    set_ids = args.sets
                for set_id in set_ids:
                    cards_done, images_done = ingest_set(
                        source, connection, http_client, settings.images_dir, set_id, lang
                    )
                    total_cards += cards_done
                    total_images += images_done
    finally:
        connection.close()

    logger.info(
        "Ingesta terminada: %s cartas, %s imagenes descargadas", total_cards, total_images
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
