# services/ml — Servicio de inferencia (FastAPI)

Servicio interno de PokemonCardDetector: detecta la carta en una foto
(OpenCV clasico), lee texto con RapidOCR, calcula embeddings con OpenCLIP
preentrenado y busca candidatos en un indice FAISS. Sin entrenamiento de
modelos: todo es preentrenado u algoritmico.

## Requisitos

- Python 3.12
- ~5 GB de disco para dependencias (torch) y modelos descargados al primer uso

## Instalacion

```powershell
cd services/ml
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -r requirements.txt
```

## Arranque

```powershell
uvicorn app.main:app --host 127.0.0.1 --port 8001
```

- `GET http://127.0.0.1:8001/health` — estado del servicio, indice y modelos.
- `POST http://127.0.0.1:8001/analyze` — multipart con campo `image`.

Los modelos se cargan lazy en el primer uso. Si falta una dependencia o el
indice FAISS, el servicio arranca igual y `/analyze` responde con
`candidates=[]` y `low_confidence=true` (degradacion elegante, nunca 500).

## Ingesta del catalogo (orden obligatorio)

1. **Crear el esquema**: arranca antes la API Rust (`cargo run` en
   `services/api`); sus migraciones crean `data/app.db`.
2. **Catalogo e imagenes** (API TCGdex, gratuita y sin clave):

   ```powershell
   python -m ingest.ingest_catalog --langs en es --sets swsh3 base1
   # o todos los sets de un idioma:
   python -m ingest.ingest_catalog --langs en --all
   ```

3. **Indice FAISS** (embeddings OpenCLIP de las imagenes descargadas):

   ```powershell
   python -m ingest.build_index
   ```

   Genera `data/index/faiss.index` y `data/index/cards.json`.

## Configuracion (variables de entorno)

El servicio (y la ingesta) leen el `.env` de la **raiz del repo** — el mismo
archivo que consume la API Rust — ademas de las variables de entorno del
proceso. Las rutas relativas (`DATA_DIR`) se anclan **siempre a la raiz del
repo**, nunca al cwd ni a `services/ml`: `DATA_DIR=./data` apunta a
`<repo>/data` en ambos servicios.

| Variable          | Default              | Descripcion                          |
|-------------------|----------------------|--------------------------------------|
| `ML_PORT`         | `8001`               | Puerto del servicio                  |
| `DATA_DIR`        | `./data`             | Directorio de datos (relativo a la raiz del repo) |
| `MODEL_NAME`      | `ViT-B-32`           | Modelo OpenCLIP                      |
| `PRETRAINED`      | `laion2b_s34b_b79k`  | Pesos preentrenados                  |
| `TOP_K`           | `5`                  | Candidatos maximos                   |
| `W_VISUAL`        | `0.65`               | Peso visual en la fusion             |
| `W_OCR`           | `0.35`               | Peso OCR en la fusion                |
| `CONF_THRESHOLD`  | `0.80`               | Umbral de confianza del top1         |
| `MARGIN_THRESHOLD`| `0.05`               | Margen minimo top1 - top2            |

## Docker

```powershell
docker build -t pcd-ml .
docker run -p 8001:8001 -v ${PWD}/../../data:/data pcd-ml
```

## Mejoras futuras

- Detector YOLO fine-tuned como alternativa opcional al detector OpenCV.
