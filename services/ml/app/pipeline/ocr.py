"""Wrapper de RapidOCR (modelos PaddleOCR preentrenados, CPU).

Import lazy: si ``rapidocr-onnxruntime`` no esta instalado, ``run_ocr``
devuelve un resultado vacio y el servicio sigue funcionando.

Heuristicas sobre la carta rectificada (600x825):
- ``name_guess``: linea de mayor confianza cuyo centro vertical cae en el
  25% superior de la imagen (zona del nombre en cartas Pokemon).
- ``number_guess``: primera coincidencia del patron ``N/N`` (1-3 digitos,
  barra, 1-3 digitos, espacios opcionales); se conserva el numerador, que
  es el numero de la carta dentro del set.
"""

from __future__ import annotations

import logging
import re
from typing import Any, Optional

import numpy as np

logger = logging.getLogger("ml.ocr")

NUMBER_PATTERN = re.compile(r"(\d{1,3})\s*/\s*(\d{1,3})")
NAME_REGION_RATIO = 0.25

_engine: Any = None
_engine_failed = False


def get_engine() -> Any:
    """Singleton lazy del motor RapidOCR. Devuelve None si no esta disponible."""
    global _engine, _engine_failed
    if _engine is not None:
        return _engine
    if _engine_failed:
        return None
    try:
        from rapidocr_onnxruntime import RapidOCR
    except ImportError:
        logger.warning("rapidocr-onnxruntime no esta instalado: OCR deshabilitado")
        _engine_failed = True
        return None
    try:
        _engine = RapidOCR()
    except Exception:
        logger.exception("No se pudo inicializar RapidOCR: OCR deshabilitado")
        _engine_failed = True
        return None
    return _engine


def _box_center_y(box: Any) -> Optional[float]:
    try:
        points = np.asarray(box, dtype=np.float32).reshape(-1, 2)
        return float(points[:, 1].mean())
    except Exception:
        return None


def run_ocr(img_bgr: np.ndarray) -> dict:
    """Ejecuta OCR sobre la carta rectificada.

    Devuelve ``{"lines": [{"text", "confidence"}], "name_guess", "number_guess"}``.
    Nunca lanza: ante cualquier fallo devuelve un resultado vacio.
    """
    empty = {"lines": [], "name_guess": None, "number_guess": None}
    engine = get_engine()
    if engine is None:
        return empty

    try:
        result, _elapse = engine(img_bgr)
    except Exception:
        logger.exception("Fallo la ejecucion de OCR")
        return empty
    if not result:
        return empty

    img_height = float(img_bgr.shape[0])
    name_limit = img_height * NAME_REGION_RATIO

    lines: list[dict] = []
    name_guess: Optional[str] = None
    best_name_confidence = -1.0
    number_guess: Optional[str] = None

    for item in result:
        try:
            box, text, confidence = item[0], item[1], item[2]
        except (IndexError, TypeError, ValueError):
            continue
        text = str(text).strip()
        if not text:
            continue
        try:
            confidence = float(confidence)
        except (TypeError, ValueError):
            confidence = 0.0

        lines.append({"text": text, "confidence": round(confidence, 4)})

        center_y = _box_center_y(box)
        if (
            center_y is not None
            and center_y <= name_limit
            and confidence > best_name_confidence
        ):
            name_guess = text
            best_name_confidence = confidence

        if number_guess is None:
            match = NUMBER_PATTERN.search(text)
            if match:
                number_guess = match.group(1)

    return {"lines": lines, "name_guess": name_guess, "number_guess": number_guess}
