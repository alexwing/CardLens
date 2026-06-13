"""Construye el indice FAISS a partir de las imagenes locales del catalogo.

Uso (desde services/ml, con el venv activado):
    python -m ingest.build_index [--batch-size 32]

Lee de data/app.db las cartas con imagen local existente, calcula embeddings
por lotes con OpenCLIP (app.pipeline.embedder) y escribe:
- data/index/faiss.index  (IndexFlatIP, vectores L2-normalizados = coseno)
- data/index/cards.json   (lista alineada fila a fila:
                           {card_id, name, number, set_id, lang})
"""

from __future__ import annotations

import argparse
import json
import logging
import sqlite3
import sys
from pathlib import Path
from typing import Optional

import numpy as np

SERVICE_ROOT = Path(__file__).resolve().parents[1]
if str(SERVICE_ROOT) not in sys.path:
    sys.path.insert(0, str(SERVICE_ROOT))

from app.config import get_settings  # noqa: E402
from app.pipeline.embedder import get_embedder  # noqa: E402

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
logger = logging.getLogger("ingest.build_index")


def read_image_bgr(path: Path) -> Optional[np.ndarray]:
    """Lee una imagen como BGR; usa cv2 si esta disponible, si no PIL."""
    try:
        import cv2

        img = cv2.imread(str(path), cv2.IMREAD_COLOR)
        if img is not None:
            return img
    except ImportError:
        pass
    try:
        from PIL import Image

        with Image.open(path) as pil_img:
            rgb = np.asarray(pil_img.convert("RGB"))
        return rgb[:, :, ::-1].copy()
    except Exception:
        return None


def load_cards_with_images(db_path: Path, images_dir: Path) -> list[dict]:
    """Cartas del catalogo cuya imagen local existe en disco."""
    if not db_path.exists():
        logger.error(
            "No existe %s. Arranca antes la API Rust (cargo run en services/api) "
            "para crear el esquema y ejecuta ingest_catalog.",
            db_path,
        )
        sys.exit(1)
    connection = sqlite3.connect(db_path)
    try:
        rows = connection.execute(
            """
            SELECT id, name, number, set_id, lang
            FROM cards
            WHERE image_local IS NOT NULL
            ORDER BY id
            """
        ).fetchall()
    finally:
        connection.close()

    cards: list[dict] = []
    for card_id, name, number, set_id, lang in rows:
        image_path = images_dir / f"{card_id}.png"
        if not image_path.exists():
            continue
        cards.append(
            {
                "card_id": card_id,
                "name": name,
                "number": number,
                "set_id": set_id,
                "lang": lang,
                "_image_path": image_path,
            }
        )
    return cards


def iter_batches(items: list, batch_size: int):
    for start in range(0, len(items), batch_size):
        yield items[start : start + batch_size]


def parse_args(argv: Optional[list[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Construye data/index/faiss.index y data/index/cards.json."
    )
    parser.add_argument(
        "--batch-size",
        type=int,
        default=32,
        help="Tamano de lote para el calculo de embeddings (default: 32)",
    )
    return parser.parse_args(argv)


def main(argv: Optional[list[str]] = None) -> int:
    args = parse_args(argv)
    settings = get_settings()

    cards = load_cards_with_images(settings.db_path, settings.images_dir)
    if not cards:
        logger.error(
            "No hay cartas con imagen local. Ejecuta antes ingest_catalog "
            "(python -m ingest.ingest_catalog --langs en --sets <ids>)."
        )
        sys.exit(1)
    logger.info("Cartas con imagen local: %s", len(cards))

    embedder = get_embedder()
    if embedder is None:
        logger.error(
            "Faltan dependencias de embeddings (torch / open_clip_torch). "
            "Instala requirements.txt antes de construir el indice."
        )
        sys.exit(1)

    try:
        import faiss
    except ImportError:
        logger.error("faiss-cpu no esta instalado. Instala requirements.txt.")
        sys.exit(1)

    try:
        from tqdm import tqdm
    except ImportError:  # tqdm es solo cosmetico
        def tqdm(iterable, **_kwargs):
            return iterable

    indexed_cards: list[dict] = []
    vectors: list[np.ndarray] = []
    skipped = 0

    batches = list(iter_batches(cards, max(1, args.batch_size)))
    for batch in tqdm(batches, desc="Embeddings", unit="lote"):
        images = []
        metas = []
        for card in batch:
            img = read_image_bgr(card["_image_path"])
            if img is None:
                skipped += 1
                continue
            images.append(img)
            metas.append(card)
        if not images:
            continue
        embeddings = embedder.embed_batch(images)  # ya L2-normalizados
        vectors.append(embeddings)
        indexed_cards.extend(metas)

    if not vectors:
        logger.error("No se pudo calcular ningun embedding; indice no generado.")
        sys.exit(1)

    matrix = np.vstack(vectors).astype(np.float32)
    index = faiss.IndexFlatIP(matrix.shape[1])
    index.add(matrix)

    settings.index_dir.mkdir(parents=True, exist_ok=True)
    faiss.write_index(index, str(settings.index_path))

    rows = [
        {
            "card_id": card["card_id"],
            "name": card["name"],
            "number": card["number"],
            "set_id": card["set_id"],
            "lang": card["lang"],
        }
        for card in indexed_cards
    ]
    settings.cards_json_path.write_text(
        json.dumps(rows, ensure_ascii=False), encoding="utf-8"
    )

    logger.info(
        "Indice generado: %s vectores (dim %s), %s imagenes ilegibles omitidas",
        index.ntotal,
        matrix.shape[1],
        skipped,
    )
    logger.info("Escrito: %s y %s", settings.index_path, settings.cards_json_path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
