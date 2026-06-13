"""Embeddings visuales con OpenCLIP preentrenado (CPU, sin entrenamiento).

Import lazy de ``torch`` y ``open_clip``: el servicio arranca aunque falten
y degrada devolviendo ``None`` desde :func:`get_embedder`.
"""

from __future__ import annotations

import logging
from typing import Optional, Sequence

import numpy as np

from app.config import get_settings

logger = logging.getLogger("ml.embedder")

_embedder: Optional["ClipEmbedder"] = None
_embedder_failed = False


class ClipEmbedder:
    """Singleton de OpenCLIP en CPU, modo eval, sin gradientes."""

    def __init__(self, model_name: str, pretrained: str) -> None:
        import open_clip
        import torch
        from PIL import Image

        self._torch = torch
        self._Image = Image
        self._device = "cpu"
        self.model_name = model_name
        self.pretrained = pretrained

        model, _, preprocess = open_clip.create_model_and_transforms(
            model_name, pretrained=pretrained
        )
        model.eval()
        model.to(self._device)
        self._model = model
        self._preprocess = preprocess

    def embed_batch(self, imgs_bgr: Sequence[np.ndarray]) -> np.ndarray:
        """Embeddings L2-normalizados de un lote de imagenes BGR.

        Devuelve un array float32 de forma (N, D).
        """
        torch = self._torch
        tensors = []
        for img in imgs_bgr:
            img = np.asarray(img)
            if img.ndim == 2:
                img = np.stack([img, img, img], axis=-1)
            rgb = np.ascontiguousarray(img[:, :, :3][:, :, ::-1])
            pil_img = self._Image.fromarray(rgb)
            tensors.append(self._preprocess(pil_img))
        batch = torch.stack(tensors).to(self._device)
        with torch.no_grad():
            features = self._model.encode_image(batch)
            features = features / features.norm(dim=-1, keepdim=True)
        return features.cpu().numpy().astype(np.float32)

    def embed_image(self, img_bgr: np.ndarray) -> np.ndarray:
        """Embedding L2-normalizado (float32, forma (D,)) de una imagen BGR."""
        return self.embed_batch([img_bgr])[0]


def get_embedder() -> Optional[ClipEmbedder]:
    """Singleton lazy. Devuelve None si faltan dependencias o falla la carga."""
    global _embedder, _embedder_failed
    if _embedder is not None:
        return _embedder
    if _embedder_failed:
        return None
    settings = get_settings()
    try:
        logger.info(
            "Cargando OpenCLIP %s (%s) en CPU...", settings.MODEL_NAME, settings.PRETRAINED
        )
        _embedder = ClipEmbedder(settings.MODEL_NAME, settings.PRETRAINED)
        logger.info("OpenCLIP cargado")
    except ImportError:
        logger.warning(
            "torch/open_clip_torch no estan instalados: embeddings deshabilitados"
        )
        _embedder_failed = True
        return None
    except Exception:
        logger.exception("No se pudo cargar el modelo OpenCLIP: embeddings deshabilitados")
        _embedder_failed = True
        return None
    return _embedder
