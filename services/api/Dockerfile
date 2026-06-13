# Imagen de la API de negocio (axum + SQLite)

FROM rust:1 AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/pokemon-card-api /usr/local/bin/pokemon-card-api
COPY migrations ./migrations

# En contenedor los datos viven en /data (montar volumen compartido del repo).
ENV API_PORT=8787 \
    DATA_DIR=/data \
    DATABASE_PATH=/data/app.db \
    ML_SERVICE_URL=http://ml:8001 \
    PRICE_PROVIDER=null

EXPOSE 8787
CMD ["pokemon-card-api"]
