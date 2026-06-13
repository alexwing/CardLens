"""Fusion de puntuaciones visual + OCR segun el contrato del proyecto.

- ``final = W_VISUAL * visual + W_OCR * ocr`` si hay OCR utilizable
  (hay ``name_guess`` y rapidfuzz esta disponible); si no, ``final = visual``.
- ``ocr_score`` por candidato: ratio difuso (rapidfuzz WRatio / 100) entre
  ``name_guess`` y el nombre del candidato, con bonus +0.15 (tope 1.0) si
  ``number_guess`` coincide con el ``number`` del candidato normalizado.
- ``low_confidence`` = True si el final_score top1 < CONF_THRESHOLD o
  (top1 - top2) < MARGIN_THRESHOLD.

``rapidfuzz`` se importa de forma lazy y su ausencia no rompe el servicio.
"""

from __future__ import annotations

import logging
from typing import Any, Optional, Sequence

logger = logging.getLogger("ml.fusion")

NUMBER_BONUS = 0.15

_rapidfuzz_failed = False


def _get_fuzz() -> Any:
    """Modulo ``rapidfuzz.fuzz`` o None si no esta instalado."""
    global _rapidfuzz_failed
    if _rapidfuzz_failed:
        return None
    try:
        from rapidfuzz import fuzz

        return fuzz
    except ImportError:
        logger.warning("rapidfuzz no esta instalado: puntuacion OCR deshabilitada")
        _rapidfuzz_failed = True
        return None


def normalize_number(value: Optional[str]) -> Optional[str]:
    """Normaliza un numero de carta: '036/189' -> '36', ' 04 ' -> '4'."""
    if value is None:
        return None
    text = str(value).strip().lower()
    if not text:
        return None
    if "/" in text:
        text = text.split("/", 1)[0].strip()
    stripped = text.lstrip("0")
    return stripped if stripped else "0"


def ocr_score(
    name_guess: Optional[str], number_guess: Optional[str], card_meta: dict
) -> float:
    """Puntuacion OCR [0, 1] de un candidato del indice."""
    score = 0.0
    fuzz = _get_fuzz()
    card_name = card_meta.get("name")
    if name_guess and card_name and fuzz is not None:
        try:
            score = float(fuzz.WRatio(name_guess, str(card_name))) / 100.0
        except Exception:
            logger.exception("Fallo el calculo de WRatio")
            score = 0.0
    guess_number = normalize_number(number_guess)
    card_number = normalize_number(card_meta.get("number"))
    if guess_number is not None and card_number is not None and guess_number == card_number:
        score = min(1.0, score + NUMBER_BONUS)
    return score


def _clamp01(value: float) -> float:
    return max(0.0, min(1.0, float(value)))


def is_low_confidence(candidates: Sequence[dict], settings: Any) -> bool:
    """Aplica los umbrales de confianza del contrato sobre candidatos ordenados."""
    if not candidates:
        return True
    top1 = candidates[0]["final_score"]
    if top1 < settings.CONF_THRESHOLD:
        return True
    if len(candidates) >= 2 and (top1 - candidates[1]["final_score"]) < settings.MARGIN_THRESHOLD:
        return True
    return False


def fuse(
    visual_results: Sequence[tuple[dict, float]],
    name_guess: Optional[str],
    number_guess: Optional[str],
    settings: Any,
) -> tuple[list[dict], bool]:
    """Combina resultados visuales y OCR.

    ``visual_results``: lista de ``(card_meta, visual_score)`` del indice.
    Devuelve ``(candidates, low_confidence)`` con candidatos ordenados de
    forma descendente por ``final_score`` (maximo ``settings.TOP_K``).
    """
    usable_ocr = bool(name_guess) and _get_fuzz() is not None

    candidates: list[dict] = []
    for card_meta, visual in visual_results:
        visual = _clamp01(visual)
        ocr = ocr_score(name_guess, number_guess, card_meta)
        if usable_ocr:
            final = settings.W_VISUAL * visual + settings.W_OCR * ocr
        else:
            final = visual
        candidates.append(
            {
                "card_id": card_meta.get("card_id"),
                "visual_score": round(visual, 4),
                "ocr_score": round(ocr, 4),
                "final_score": round(final, 4),
            }
        )

    candidates.sort(key=lambda c: c["final_score"], reverse=True)
    candidates = candidates[: settings.TOP_K]
    return candidates, is_low_confidence(candidates, settings)
