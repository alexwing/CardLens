"""Deteccion clasica de cartas con OpenCV (sin modelos entrenados).

Pipeline: redimension de trabajo -> escala de grises -> blur -> Canny ->
dilatacion -> contornos externos -> aproximacion poligonal a un cuadrilatero
de 4 puntos -> rectificacion de perspectiva a 600x825.

Si no se encuentra un cuadrilatero plausible, se devuelve la imagen completa
redimensionada como fallback para que el resto del pipeline siga funcionando.

``cv2`` se importa de forma lazy: si OpenCV no esta instalado el modulo sigue
siendo importable y la deteccion degrada devolviendo la imagen original.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Optional

import numpy as np

logger = logging.getLogger("ml.detector")

# Tamano de la carta rectificada (relacion 63x88 mm ~= 1.397).
CARD_WIDTH = 600
CARD_HEIGHT = 825

# Ancho de trabajo para la deteccion (acelera Canny/contornos).
WORK_WIDTH = 960

# Area minima del cuadrilatero: ~10% del frame de trabajo.
MIN_AREA_RATIO = 0.10

# Aspect ratio plausible de carta (alto/ancho ~1.39); se acepta el inverso
# si la carta esta girada 90 grados.
ASPECT_MIN = 1.2
ASPECT_MAX = 1.6

# Numero maximo de contornos grandes a evaluar.
MAX_CONTOURS = 10


@dataclass
class DetectionResult:
    """Resultado de la deteccion de carta."""

    found: bool
    quad: Optional[list]  # [[x, y] x 4] en coordenadas de la imagen original
    warped: np.ndarray  # imagen BGR 600x825 (o fallback)


def _try_import_cv2():
    try:
        import cv2

        return cv2
    except ImportError:
        logger.warning("opencv-python no esta instalado: deteccion deshabilitada")
        return None


def _ensure_bgr3(img: np.ndarray) -> np.ndarray:
    """Normaliza la imagen a 3 canales BGR."""
    if img.ndim == 2:
        return np.stack([img, img, img], axis=-1)
    if img.ndim == 3 and img.shape[2] == 4:
        return np.ascontiguousarray(img[:, :, :3])
    return img


def order_corners(points: np.ndarray) -> np.ndarray:
    """Ordena 4 esquinas como (tl, tr, br, bl)."""
    pts = np.asarray(points, dtype=np.float32).reshape(4, 2)
    sums = pts.sum(axis=1)
    diffs = np.diff(pts, axis=1).ravel()  # y - x
    tl = pts[np.argmin(sums)]
    br = pts[np.argmax(sums)]
    tr = pts[np.argmin(diffs)]
    bl = pts[np.argmax(diffs)]
    return np.array([tl, tr, br, bl], dtype=np.float32)


def _edge_lengths(quad: np.ndarray) -> tuple[float, float]:
    """Devuelve (ancho, alto) medios del cuadrilatero ordenado."""
    top = float(np.linalg.norm(quad[1] - quad[0]))
    right = float(np.linalg.norm(quad[2] - quad[1]))
    bottom = float(np.linalg.norm(quad[3] - quad[2]))
    left = float(np.linalg.norm(quad[0] - quad[3]))
    width = (top + bottom) / 2.0
    height = (left + right) / 2.0
    return width, height


def _plausible_aspect(width: float, height: float) -> bool:
    if width <= 1.0 or height <= 1.0:
        return False
    ratio = height / width
    if ASPECT_MIN <= ratio <= ASPECT_MAX:
        return True
    inverse = 1.0 / ratio
    return ASPECT_MIN <= inverse <= ASPECT_MAX


def _find_quad(cv2, img: np.ndarray) -> Optional[np.ndarray]:
    """Busca el mejor cuadrilatero de carta. Coordenadas de la imagen original."""
    height, width = img.shape[:2]
    scale = WORK_WIDTH / width if width > WORK_WIDTH else 1.0
    if scale != 1.0:
        work = cv2.resize(
            img, (int(width * scale), int(height * scale)), interpolation=cv2.INTER_AREA
        )
    else:
        work = img

    gray = cv2.cvtColor(work, cv2.COLOR_BGR2GRAY)
    blurred = cv2.GaussianBlur(gray, (5, 5), 0)
    edges = cv2.Canny(blurred, 50, 150)
    edges = cv2.dilate(edges, np.ones((3, 3), np.uint8), iterations=2)

    contours, _ = cv2.findContours(edges, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    if not contours:
        return None

    frame_area = work.shape[0] * work.shape[1]
    best_quad: Optional[np.ndarray] = None
    best_area = 0.0

    for contour in sorted(contours, key=cv2.contourArea, reverse=True)[:MAX_CONTOURS]:
        perimeter = cv2.arcLength(contour, True)
        approx = cv2.approxPolyDP(contour, 0.02 * perimeter, True)
        if len(approx) != 4 or not cv2.isContourConvex(approx):
            continue
        area = float(cv2.contourArea(approx))
        if area < MIN_AREA_RATIO * frame_area:
            continue
        quad = order_corners(approx.reshape(4, 2).astype(np.float32))
        quad_w, quad_h = _edge_lengths(quad)
        if not _plausible_aspect(quad_w, quad_h):
            continue
        if area > best_area:
            best_quad = quad
            best_area = area

    if best_quad is None:
        return None
    return best_quad / scale


def _warp(cv2, img: np.ndarray, quad: np.ndarray) -> np.ndarray:
    """Rectifica la perspectiva del cuadrilatero a 600x825 (retrato)."""
    quad = order_corners(quad)
    quad_w, quad_h = _edge_lengths(quad)
    if quad_w > quad_h:
        # Carta apaisada: rota el orden de esquinas para que el lado corto
        # quede arriba y el resultado salga en retrato.
        quad = np.array([quad[3], quad[0], quad[1], quad[2]], dtype=np.float32)
    destination = np.array(
        [
            [0, 0],
            [CARD_WIDTH - 1, 0],
            [CARD_WIDTH - 1, CARD_HEIGHT - 1],
            [0, CARD_HEIGHT - 1],
        ],
        dtype=np.float32,
    )
    matrix = cv2.getPerspectiveTransform(quad, destination)
    return cv2.warpPerspective(img, matrix, (CARD_WIDTH, CARD_HEIGHT))


def detect_card(img_bgr: np.ndarray) -> DetectionResult:
    """Detecta y rectifica una carta en la imagen.

    Devuelve siempre una imagen ``warped`` utilizable por el resto del
    pipeline, aunque la deteccion falle (fallback: imagen completa
    redimensionada a 600x825).
    """
    img = _ensure_bgr3(np.asarray(img_bgr))
    cv2 = _try_import_cv2()
    if cv2 is None:
        return DetectionResult(found=False, quad=None, warped=img)

    try:
        quad = _find_quad(cv2, img)
    except Exception:
        logger.exception("Fallo inesperado en la deteccion; se usa la imagen completa")
        quad = None

    if quad is None:
        fallback = cv2.resize(img, (CARD_WIDTH, CARD_HEIGHT), interpolation=cv2.INTER_AREA)
        return DetectionResult(found=False, quad=None, warped=fallback)

    warped = _warp(cv2, img, quad)
    quad_out = [[round(float(x), 2), round(float(y), 2)] for x, y in quad.tolist()]
    return DetectionResult(found=True, quad=quad_out, warped=warped)
