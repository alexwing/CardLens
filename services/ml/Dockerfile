# Servicio ML de PokemonCardDetector (FastAPI + OpenCV + OpenCLIP + FAISS)
FROM python:3.12-slim

# Dependencias nativas minimas para opencv-python.
RUN apt-get update \
    && apt-get install -y --no-install-recommends libgl1 libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY app ./app
COPY ingest ./ingest

# Monta el directorio de datos del repo en /data (db, imagenes, indice).
ENV DATA_DIR=/data
ENV ML_PORT=8001

EXPOSE 8001

# Forma shell para que se expanda ML_PORT.
CMD uvicorn app.main:app --host 0.0.0.0 --port ${ML_PORT}
