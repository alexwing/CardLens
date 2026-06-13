"""Configuracion del servicio ML basada en pydantic-settings.

Todos los valores son sobreescribibles via variables de entorno
(por ejemplo ``ML_PORT=8002`` o ``DATA_DIR=D:/datos``).
"""

from __future__ import annotations

from functools import lru_cache
from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict

# Raiz del servicio: services/ml (este archivo vive en services/ml/app/config.py).
SERVICE_ROOT = Path(__file__).resolve().parents[1]

# Raiz del repo: dos niveles por encima de services/ml. Las rutas relativas de
# configuracion (DATA_DIR) y el .env compartido se anclan aqui, igual que en la
# API Rust, para que ambos servicios consuman exactamente la misma configuracion.
REPO_ROOT = SERVICE_ROOT.parents[1]


class Settings(BaseSettings):
    """Parametros del servicio de inferencia."""

    # .env unico en la raiz del repo, compartido con la API Rust.
    model_config = SettingsConfigDict(env_file=REPO_ROOT / ".env", extra="ignore")

    ML_PORT: int = 8001

    # Relativa a la raiz del repo -> E:/projects/PokemonCardDetector/data
    DATA_DIR: str = "./data"

    # Ruta de la base SQLite; si no se define, se usa DATA_DIR/app.db.
    DATABASE_PATH: str | None = None

    # Modelo de embeddings (OpenCLIP preentrenado, sin entrenamiento propio).
    MODEL_NAME: str = "ViT-B-32"
    PRETRAINED: str = "laion2b_s34b_b79k"

    # Busqueda y fusion de puntuaciones.
    TOP_K: int = 5
    W_VISUAL: float = 0.65
    W_OCR: float = 0.35
    CONF_THRESHOLD: float = 0.80
    MARGIN_THRESHOLD: float = 0.05

    @property
    def data_dir(self) -> Path:
        """DATA_DIR resuelto a ruta absoluta respecto a la raiz del repo."""
        path = Path(self.DATA_DIR)
        if not path.is_absolute():
            path = (REPO_ROOT / path).resolve()
        return path

    @property
    def db_path(self) -> Path:
        """DATABASE_PATH resuelto respecto a la raiz del repo, o DATA_DIR/app.db."""
        if self.DATABASE_PATH:
            path = Path(self.DATABASE_PATH)
            if not path.is_absolute():
                path = (REPO_ROOT / path).resolve()
            return path
        return self.data_dir / "app.db"

    @property
    def images_dir(self) -> Path:
        return self.data_dir / "images"

    @property
    def index_dir(self) -> Path:
        return self.data_dir / "index"

    @property
    def index_path(self) -> Path:
        return self.index_dir / "faiss.index"

    @property
    def cards_json_path(self) -> Path:
        return self.index_dir / "cards.json"


@lru_cache
def get_settings() -> Settings:
    """Singleton de configuracion."""
    return Settings()
