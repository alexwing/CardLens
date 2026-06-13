"""Servicio ML de PokemonCardDetector (FastAPI, interno).

Endpoints:
- GET  /health  -> estado del servicio, indice y modelos.
- POST /analyze -> deteccion + OCR + embedding + busqueda + fusion.

Diseno: los modelos (OCR, OpenCLIP, FAISS) se cargan lazy en el primer uso y
se cachean como singletons. Si falta una dependencia o el indice, el servicio
arranca igual y /analyze degrada a ``candidates=[]`` y ``low_confidence=true``
(nunca 500 por modelo ausente).
"""

from __future__ import annotations

import logging
import time
from typing import Optional

import numpy as np
from fastapi import FastAPI, File, HTTPException, UploadFile

from app.config import get_settings
from app.pipeline import detector, fusion, ocr
from app.pipeline.embedder import get_embedder
from app.pipeline.index import get_index

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(name)s: %(message)s",
)
logger = logging.getLogger("ml.main")

app = FastAPI(
    title="PokemonCardDetector ML",
    description="Servicio interno de deteccion, OCR y busqueda visual de cartas.",
    version="0.1.0",
)


def _decode_image(data: bytes) -> Optional[np.ndarray]:
    """Decodifica bytes de imagen a BGR. Usa cv2 si esta disponible, si no PIL."""
    try:
        import cv2

        array = np.frombuffer(data, dtype=np.uint8)
        img = cv2.imdecode(array, cv2.IMREAD_COLOR)
        if img is not None:
            return img
    except ImportError:
        pass
    except Exception:
        logger.exception("Fallo cv2.imdecode; se intenta con PIL")
    try:
        from io import BytesIO

        from PIL import Image

        pil_img = Image.open(BytesIO(data)).convert("RGB")
        return np.asarray(pil_img)[:, :, ::-1].copy()
    except Exception:
        return None


@app.get("/health")
def health() -> dict:
    settings = get_settings()
    index = get_index()
    return {
        "status": "ok",
        "index": {"loaded": index.loaded(), "size": index.size()},
        "models": {
            "detector": "opencv-quad",
            "ocr": "rapidocr",
            "embedder": f"{settings.MODEL_NAME}/{settings.PRETRAINED}",
        },
    }


@app.post("/analyze")
async def analyze(image: UploadFile = File(...)) -> dict:
    data = await image.read()
    if not data:
        raise HTTPException(status_code=400, detail="El campo 'image' esta vacio")
    img = _decode_image(data)
    if img is None:
        raise HTTPException(
            status_code=400, detail="No se pudo decodificar la imagen recibida"
        )

    settings = get_settings()
    timing_ms: dict[str, float] = {}

    # 1. Deteccion y rectificacion de la carta.
    started = time.perf_counter()
    detection = detector.detect_card(img)
    timing_ms["detect"] = round((time.perf_counter() - started) * 1000.0, 2)

    # 2. OCR sobre la carta rectificada.
    started = time.perf_counter()
    ocr_result = ocr.run_ocr(detection.warped)
    timing_ms["ocr"] = round((time.perf_counter() - started) * 1000.0, 2)

    # 3. Embedding visual (degrada a None si faltan torch/open_clip).
    started = time.perf_counter()
    embedding: Optional[np.ndarray] = None
    embedder = get_embedder()
    if embedder is not None:
        try:
            embedding = embedder.embed_image(detection.warped)
        except Exception:
            logger.exception("Fallo el calculo del embedding; se degrada sin candidatos")
    timing_ms["embed"] = round((time.perf_counter() - started) * 1000.0, 2)

    # 4. Busqueda en el indice FAISS (degrada a [] si no hay indice).
    started = time.perf_counter()
    visual_results: list = []
    index = get_index()
    if embedding is not None and index.loaded():
        try:
            visual_results = index.search(embedding, settings.TOP_K)
        except Exception:
            logger.exception("Fallo la busqueda FAISS; se degrada sin candidatos")
    timing_ms["search"] = round((time.perf_counter() - started) * 1000.0, 2)

    # 5. Fusion visual + OCR y umbrales de confianza.
    candidates, low_confidence = fusion.fuse(
        visual_results,
        ocr_result["name_guess"],
        ocr_result["number_guess"],
        settings,
    )

    return {
        "detection": {
            "found": detection.found,
            "method": "opencv-quad",
            "quad": detection.quad,
        },
        "ocr": {
            "lines": ocr_result["lines"],
            "name_guess": ocr_result["name_guess"],
            "number_guess": ocr_result["number_guess"],
        },
        "candidates": candidates,
        "low_confidence": low_confidence,
        "timing_ms": timing_ms,
    }


if __name__ == "__main__":
    import uvicorn

    uvicorn.run("app.main:app", host="127.0.0.1", port=get_settings().ML_PORT)
