# services/api — API de negocio (Rust: axum + sqlx + SQLite)

API publica del detector de cartas Pokemon. Expone el catalogo, el escaneo de
fotos (delegando el analisis al servicio ML), la coleccion del usuario y los
precios con cache de 24 horas.

## Ejecutar en desarrollo

```bash
cd services/api
cargo run
```

Al arrancar:

- Crea los directorios `data/`, `data/scans/` y `data/images/` si no existen.
- Crea `data/app.db` (SQLite, modo WAL) y aplica las migraciones de
  `migrations/` automaticamente. Este modulo es el unico propietario del
  esquema; la ingesta Python rellena el catalogo despues.
- Escucha en `http://0.0.0.0:8787` (prefijo `/api`), sirve `data/images` en
  `/images/*` y `data/scans` en `/scans/*`.

## Variables de entorno

| Variable        | Default                  | Descripcion                                   |
|-----------------|--------------------------|-----------------------------------------------|
| `API_PORT`      | `8787`                   | Puerto HTTP de la API                         |
| `DATABASE_PATH` | `<raiz del repo>/data/app.db` | Fichero SQLite                           |
| `DATA_DIR`      | `<raiz del repo>/data`   | Directorio de datos (imagenes y escaneos)     |
| `ML_SERVICE_URL`| `http://127.0.0.1:8001`  | URL base del servicio ML (FastAPI)            |
| `PRICE_PROVIDER`| `null`                   | Proveedor de precios: `null` o `tcgdex`       |

Se carga `.env` si existe (dotenvy busca en el cwd y sus padres, asi que el
`.env` de la raiz del repo se aplica aunque arranques desde `services/api`).
Las rutas relativas de `DATABASE_PATH` y `DATA_DIR` se resuelven contra la
**raiz del repo** (no contra el cwd), de modo que `DATA_DIR=./data` siempre
apunta a `<repo>/data`, el mismo arbol que usan la ingesta y el servicio ML.
En despliegues fuera del repo (p. ej. Docker) usa rutas absolutas.

## Endpoints principales

- `GET  /api/health` — estado de la API y alcance del servicio ML.
- `POST /api/scan` — multipart con campo `image`; identifica la carta.
- `GET  /api/cards?q=&set_id=&page=&page_size=` — catalogo paginado.
- `GET  /api/cards/{id}` / `GET /api/sets` — detalle y sets.
- `GET  /api/scans?page=` — historial de escaneos.
- `GET  /api/collection`, `POST /api/collection/items`,
  `DELETE /api/collection/items/{id}` — coleccion.
- `GET  /api/prices/{card_id}` — precios con cache de 24h en la tabla `prices`.

## Docker

```bash
docker build -t pcd-api services/api
```

La imagen espera un volumen montado en `/data` compartido con la ingesta y el
servicio ML.
