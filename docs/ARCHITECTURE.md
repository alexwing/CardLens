# Arquitectura de PokemonCardDetector

Documento de referencia del sistema. Audiencia: cualquier persona que vaya a desarrollar, operar o ampliar el proyecto.

---

## 1. Vision y objetivos

PokemonCardDetector permite a un coleccionista apuntar con la camara (o subir una foto) a una carta Pokemon y obtener en segundos: la identificacion de la carta, sus metadatos de catalogo (set, numero, rareza, ilustrador), un precio estimado opcional y la posibilidad de anadirla a su coleccion personal.

Objetivos de diseno:

- **Local-first y privado**: toda la inferencia corre en la maquina del usuario. Las fotos nunca salen del equipo.
- **Cero entrenamiento de modelos en el MVP** (restriccion dura): solo modelos preentrenados (RapidOCR, OpenCLIP) y vision clasica (OpenCV).
- **Sin servicios de pago**: catalogo y precios via la API gratuita de TCGdex; sin API keys.
- **Modular y reemplazable**: cada pieza (deteccion, OCR, embeddings, catalogo, precios, persistencia) esta detras de una interfaz para poder sustituirla sin tocar el resto.
- **Multiplataforma con un solo codigo base**: web (Vite/React) y escritorio/Android (Tauri 2) consumen la misma API.

No-objetivos del MVP: autenticacion, multiusuario, sincronizacion en nube, deteccion de falsificaciones, grading de condicion.

---

## 2. Diagrama de componentes y responsabilidades

```mermaid
flowchart TB
    subgraph Captura
        WEB[apps/web<br/>Vite + React :5173]
        DESK[apps/desktop<br/>Tauri 2 escritorio/Android]
    end

    subgraph Persistencia["Persistencia y API publica"]
        API[services/api<br/>Rust axum :8787]
        DB[(SQLite data/app.db<br/>WAL)]
        STATIC[Estaticos<br/>/images/* /scans/*]
        PRICE[PriceProvider<br/>null | tcgdex]
    end

    subgraph Inferencia["Inferencia (interna)"]
        ML[services/ml<br/>FastAPI :8001]
        DET[Detector OpenCV<br/>Canny + contornos]
        OCRM[RapidOCR<br/>PaddleOCR preentrenado]
        EMB[OpenCLIP ViT-B-32<br/>laion2b_s34b_b79k]
        IDX[FAISS IndexFlatIP<br/>data/index/faiss.index]
    end

    subgraph Ingesta["Ingesta (batch, offline)"]
        ING[ingest_catalog + build_index]
        SRC[CatalogSource<br/>TCGdexSource]
    end

    TCGDEX[(TCGdex API v2<br/>catalogo, imagenes, precios)]

    WEB --> API
    DESK --> API
    API -->|POST /analyze| ML
    API --> DB
    API --> STATIC
    API --> PRICE
    PRICE -.-> TCGDEX
    ML --> DET --> OCRM
    ML --> EMB --> IDX
    ING --> SRC -.-> TCGDEX
    ING --> DB
    ING -->|imagenes + indice| IDX
```

Responsabilidades, claramente separadas:

| Modulo | Responsabilidad | NO hace |
|---|---|---|
| `apps/web` | Captura (camara/subida), UI de resultados, coleccion | Logica de negocio, acceso a DB |
| `apps/desktop` | Empaquetar el frontend web como app nativa (Tauri 2) | Logica propia mas alla del shell |
| `services/api` | API publica, persistencia (unico escritor de SQLite en runtime), servir estaticos, precios via `PriceProvider`, orquestar el scan | Inferencia ML |
| `services/ml` (app) | Inferencia: deteccion, OCR, embedding, busqueda FAISS, fusion de scores | Persistir en DB, servir a clientes finales |
| `services/ml` (ingest) | Descargar catalogo e imagenes (via `CatalogSource`), poblar SQLite, construir el indice FAISS | Servir trafico |
| `data/` | Estado compartido: DB, imagenes, scans, indice | — |

El esquema SQL tiene un unico propietario: `services/api/migrations/0001_init.sql`. La ingesta Python escribe contra ese esquema pero nunca lo define ni lo migra.

---

## 3. Flujo de datos: de la camara al resultado

1. **Captura**: el cliente toma una foto (getUserMedia o `<input capture>`) y la envia como multipart (campo `image`) a `POST /api/scan`.
2. **Persistencia inicial**: la API genera un `scan_id` (UUID v4), guarda la imagen en `data/scans/{scan_id}.jpg` y reenvia los bytes a `POST /analyze` del servicio ML.
3. **Deteccion y rectificacion (OpenCV)**: Canny + contornos sobre la imagen reducida; se busca el cuadrilatero convexo mas grande con aspecto compatible con una carta y se rectifica la perspectiva a 600x825 px. Si no hay cuadrilatero, se usa la imagen completa (`detection.found=false`).
4. **OCR (RapidOCR)**: sobre la imagen rectificada, se extraen lineas con confianza; heuristicas derivan `name_guess` (banda superior) y `number_guess` (primera coincidencia del patron `\d{1,3}/\d{1,3}` en cualquier linea OCR).
5. **Embedding (OpenCLIP)**: la imagen rectificada pasa por ViT-B-32 (`laion2b_s34b_b79k`); el vector se normaliza L2.
6. **Busqueda (FAISS)**: `IndexFlatIP` devuelve el top-5 por similitud coseno; `data/index/cards.json` mapea fila -> `card_id`.
7. **Fusion de scores**: por candidato, `ocr_score` = WRatio(rapidfuzz)/100 entre `name_guess` y el nombre del candidato, con bonus +0.15 (tope 1.0) si `number_guess` coincide con su `number` normalizado. Si hay OCR utilizable: `final = 0.65*visual + 0.35*ocr`; si no: `final = visual`. `low_confidence = true` si top1 < 0.80 o (top1 − top2) < 0.05.
8. **Enriquecimiento**: la API hace JOIN `cards`+`sets` para construir objetos `Card` completos (nombre del set, imagenes, rareza...).
9. **Persistencia final**: la API inserta la fila en `scans` (con `raw_json` de la respuesta ML) y los candidatos en `scan_candidates`.
10. **Respuesta**: `{scan_id, low_confidence, best, candidates}`; la UI muestra el mejor candidato y alternativas clicables si `low_confidence`.

Latencias esperadas en CPU moderna (orientativas, imagen de movil ~12 MP):

| Etapa | Latencia tipica |
|---|---|
| Deteccion + rectificacion (OpenCV) | 20–80 ms |
| OCR (RapidOCR, CPU) | 300–900 ms |
| Embedding (OpenCLIP ViT-B-32, CPU) | 150–500 ms |
| Busqueda FAISS (<100k vectores, flat) | < 5 ms |
| Persistencia + JOIN (SQLite) | < 10 ms |
| **Total percibido** | **~0.5–1.5 s** |

---

## 4. Justificacion del stack

- **Deteccion clasica OpenCV, no YOLO (en el MVP)**. La restriccion del proyecto es no entrenar nada. Un YOLO generico no tiene la clase "carta Pokemon", asi que requeriria fine-tuning. Y no hace falta: una carta es un cuadrilatero de bordes rectos y alto contraste sobre el fondo; Canny + contornos + rectificacion de perspectiva resuelve el caso comun con coste casi nulo y cero dependencias de modelos. YOLO fine-tuned queda documentado como Fase B opcional (seccion 7) solo si la deteccion clasica falla en condiciones reales.
- **RapidOCR** (`rapidocr-onnxruntime`): modelos PaddleOCR preentrenados multilingues sobre ONNX Runtime CPU. Sin GPU, sin entrenamiento, sin servicios externos, y cubre latin (en/es/fr/de/it/pt) de serie.
- **OpenCLIP + FAISS**: CLIP preentrenado (ViT-B-32, `laion2b_s34b_b79k`) da embeddings visualmente discriminativos sin entrenar; las ilustraciones de cartas son lo bastante distintivas para que la similitud coseno funcione como identificador. FAISS `IndexFlatIP` con vectores L2-normalizados es busqueda exacta, trivial de construir y mas que suficiente para el tamano del catalogo (decenas de miles de cartas).
- **axum + sqlx (Rust)**: API ligera, tipada y sin runtime pesado; sqlx valida queries y trae migraciones; cambiar SQLite -> PostgreSQL es principalmente cambiar el driver y la cadena de conexion.
- **Tauri 2**: un solo codigo base web que empaqueta escritorio (Windows/macOS/Linux) y Android, con binarios pequenos frente a Electron.
- **SQLite -> PostgreSQL**: SQLite en WAL es ideal para single-user local (cero operacion); el esquema y sqlx estan pensados para migrar a PostgreSQL cuando haya multiusuario (seccion 8).

---

## 5. Modelo de datos

7 tablas en SQLite (WAL). Propietario unico del esquema: `services/api/migrations/0001_init.sql`.

| Tabla | Proposito | Notas |
|---|---|---|
| `sets` | Sets del catalogo | `id = tcgdexSetId + '_' + lang` (p. ej. `swsh3_en`); un set por idioma |
| `cards` | Cartas del catalogo | `id = tcgdexId + '_' + lang` (p. ej. `swsh3-136_en`); `image_url` (remota) e `image_local` (en `data/images/`); `updated_at` para refrescos de ingesta |
| `scans` | Un escaneo de usuario | `id` UUID v4; `image_path` apunta a `data/scans/`; `raw_json` conserva la respuesta integra del ML para depuracion y re-puntuacion futura |
| `scan_candidates` | Top-5 candidatos por scan | Conserva `visual_score`, `ocr_score` y `final_score` por separado para poder auditar y re-ponderar la fusion |
| `users` | Usuarios | **Sin auth en el MVP**: existe ya para que `collection_items.user_id` no requiera una migracion destructiva al activar autenticacion (seccion 8); mientras tanto `user_id` es NULL |
| `collection_items` | Coleccion personal | `id` UUID v4; referencia opcional al `scan_id` de origen; `quantity`, `condition`, `lang`, `notes` |
| `prices` | Cache de precios | PK compuesta `(card_id, source)`; `updated_at` implementa el TTL de 24 h; varias fuentes pueden convivir |

Indices: `cards(name)`, `cards(set_id)`, `scan_candidates(scan_id)`, `collection_items(card_id)`.

Decisiones de IDs:

- `card_id`/`set_id` llevan el idioma sufijado con guion bajo (seguro como nombre de archivo en Windows y particionable por el ultimo `_` para recuperar `tcgdex_id` + `lang` al llamar a fuentes externas).
- `scan_id` y `collection_items.id` son UUID v4 en texto: generables por cualquier componente sin coordinarse con la DB.

---

## 6. Endpoints

### API publica (Rust axum, :8787, prefijo `/api`, JSON, CORS permisivo)

| Metodo | Ruta | Descripcion |
|---|---|---|
| GET | `/api/health` | `{"status":"ok","ml":{"reachable":bool}}` |
| POST | `/api/scan` | Multipart campo `image`. Guarda en `data/scans`, llama al ML, enriquece con catalogo, persiste. Respuesta: `{"scan_id","low_confidence","best":{"card":Card,"confidence"}|null,"candidates":[{"card":Card,"confidence","visual_score","ocr_score"}]}`. Si el ML devuelve 0 candidatos: `best=null`, `candidates=[]` |
| GET | `/api/cards?q=&set_id=&page=&page_size=` | Busqueda de catalogo (`q` filtra por nombre con LIKE). `{"items":[Card],"total","page"}` |
| GET | `/api/cards/{id}` | Detalle de carta -> `Card` |
| GET | `/api/sets` | `[{"id","name","series","code","release_date","total","lang"}]` |
| GET | `/api/scans?page=` | Historial: `{"items":[{"id","created_at","best_card_id","confidence","low_confidence"}],"total","page"}` |
| GET | `/api/collection` | `{"items":[{"id","card":Card,"quantity","condition","lang","notes","created_at"}]}` |
| POST | `/api/collection/items` | Body `{"card_id","scan_id"|null,"quantity","condition"|null,"lang"|null,"notes"|null}` -> 201 con el item creado |
| DELETE | `/api/collection/items/{id}` | 204 |
| GET | `/api/prices/{card_id}` | `{"card_id","prices":[{"source","currency","market","low","high","trend","updated_at"}]}`; cache 24 h en tabla `prices`; `NullPriceProvider` -> lista vacia |
| GET | `/images/*` | Estaticos: imagenes oficiales (`data/images`) |
| GET | `/scans/*` | Estaticos: fotos de usuario (`data/scans`) |

`Card = {"id","set_id","name","number","rarity","supertype","lang","image_url","image_local","set_name"}`

### API interna del servicio ML (FastAPI, :8001)

| Metodo | Ruta | Descripcion |
|---|---|---|
| GET | `/health` | `{"status":"ok","index":{"loaded":bool,"size":int},"models":{"detector":"opencv-quad","ocr":"rapidocr","embedder":"<nombre modelo>"}}` |
| POST | `/analyze` | Multipart campo `image`. Respuesta: `detection` (`found`, `method`, `quad` o null), `ocr` (`lines`, `name_guess`, `number_guess`), `candidates` (max 5, descendente por `final_score`), `low_confidence`, `timing_ms` (`detect`, `ocr`, `embed`, `search`) |

Degradacion controlada: si el indice FAISS no existe o falta una dependencia de modelo, el servicio arranca igualmente y `/analyze` responde `candidates=[]` y `low_confidence=true`. Nunca un 500 por modelo ausente.

---

## 7. Plan de datos y modelos

- **Fase A (actual)** — solo preentrenados. El indice FAISS se construye desde las **imagenes oficiales** de TCGdex (`data/images/{card_id}.png`): un embedding OpenCLIP por carta. Cero etiquetado, cero entrenamiento. Riesgo asumido: gap de dominio entre imagen oficial limpia y foto real (brillo, perspectiva); lo mitiga la rectificacion previa y la fusion con OCR.
- **Fase B (opcional, solo si la deteccion clasica falla en condiciones reales)** — fine-tuning de un detector pequeno (YOLOv8n/YOLO11n) con **dataset sintetico**: composicion de las imagenes oficiales sobre fondos aleatorios con augmentations de perspectiva, iluminacion, blur y oclusion parcial. 10–20k muestras generadas por script; las cajas se conocen por construccion, asi que **sin etiquetado manual**. El detector sustituiria al modulo OpenCV detras de la misma interfaz.
- **Fase C (opcional)** — export de OpenCLIP a ONNX (y cuantizacion) para reducir la huella de torch y acercar la inferencia al dispositivo (escritorio sin Python, eventualmente movil).

---

## 8. Escalado MVP -> produccion

| Dimension | MVP | Produccion |
|---|---|---|
| Base de datos | SQLite WAL | PostgreSQL via sqlx (cambio de driver y connection string; el esquema ya es portable) |
| Busqueda vectorial | FAISS flat en proceso | FAISS como servicio propio, o indice IVF/HNSW si el catalogo y el QPS crecen |
| Autenticacion | Ninguna | JWT; la tabla `users` ya existe y `collection_items.user_id` ya esta en el esquema: activar auth no requiere migracion destructiva |
| Ingesta | Script batch manual | Colas (p. ej. trabajos programados + cola de tareas) con refrescos incrementales por `updated_at` |
| Observabilidad | Logs | `tracing` estructurado en la API, metricas de latencia por etapa del pipeline (el ML ya reporta `timing_ms`) |
| Tenancy | Single-user local | Multiusuario: scoping por `user_id` en scans y coleccion, cuotas y rate-limiting |

---

## 9. Riesgos y mitigaciones

| Riesgo | Mitigacion |
|---|---|
| Brillos, holograficas y reverse-holo rompen el OCR | El peso dominante de la fusion es visual (0.65); si no hay OCR utilizable, `final = visual`. La UI siempre ofrece los 5 candidatos como alternativas clicables y marca `low_confidence` |
| Cartas en otros idiomas | Ingesta multilenguaje (`--langs en es ...`), un `card_id` por idioma, y OCR PaddleOCR multilingue |
| `getUserMedia` exige HTTPS fuera de localhost | Fallback a `<input type="file" capture>` (siempre funciona) y mkcert para servir la web con TLS en LAN al probar desde el movil |
| torch pesa varios GB en CPU | Documentado: Fase C exporta CLIP a ONNX y elimina torch del runtime de inferencia |
| Limites y ToS de fuentes de precios | Conector `PriceProvider` desacoplado (default `null` = sin llamadas externas) + cache de 24 h en la tabla `prices`; cambiar de fuente es una implementacion nueva, no un refactor |
| Concurrencia en SQLite | Modo WAL; en runtime solo escribe la API; la ingesta es batch offline (no convive con trafico de escritura) |

---

## 10. Plan por fases

- **F0 — Scaffolding (este repo)**: estructura del monorepo, contrato de API, esquema SQL, esqueletos funcionales de API, ML, ingesta y web.
- **F1 — MVP identificar + coleccion**: subida de imagen, pipeline completo de identificacion, catalogo navegable, anadir/quitar de la coleccion.
- **F2 — Camara en vivo + PWA + Android + precios**: captura continua con getUserMedia, instalable como PWA, build Android con Tauri 2, `PRICE_PROVIDER=tcgdex`.
- **F3 — Produccion**: autenticacion JWT, PostgreSQL, multiusuario, telemetria/observabilidad y despliegue.
