# Arquitectura Android on-device para CardLens (repo: PokemonCardDetector)

Documento de diseño. Fecha: 2026-06-13. Autor: arquitectura.

Pregunta del usuario: ¿es posible una ÚNICA app Android que incluya cliente + parte servidor + modelo ML, idealmente offline (sin PC ni internet para identificar)?

---

## (a) Veredicto directo

Sí, es posible y recomendable, pero NO empaquetando el servicio Python actual. La identificación 100% offline en una sola app Android es viable reescribiendo la capa de inferencia en runtimes nativos (ONNX Runtime vía el crate Rust `ort`, OpenCV nativo, índice vectorial en Rust) dentro del core Rust de Tauri 2, y exponiéndola al frontend React por comandos `invoke` en lugar de `fetch` a `localhost`. El catálogo (metadata + embeddings) viaja dentro del APK; solo las imágenes grandes de cartas se descargan/cachean bajo demanda. El stack Python (torch + faiss-cpu + open_clip + opencv-python + rapidocr-onnxruntime) se ABANDONA en el dispositivo: no se puede bundlear de forma realista.

---

## (b) Por qué el servicio Python actual no se puede bundlear tal cual

El bloqueante más severo y representativo es **torch vía pip**:

- PyPI no publica wheels de torch para Android. A 2026-05-13, `torch` 2.12.0 solo distribuye wheels para `win_amd64`, `manylinux_2_28_x86_64`, `manylinux_2_28_aarch64` (Linux glibc, NO Android Bionic) y `macosx_14_0_arm64`. Cero tags `android`. La wheel `aarch64` es de Linux glibc y NO es compatible con el ABI Bionic de Android. [confirmado]
- Chaquopy (la única vía real de pip-en-Android) solo soporta `torch` 1.8.1 sobre Python 3.8 (declaración del mantenedor en los issues #1215 abierto/2024 y #1376/2025). El `open_clip_torch` moderno requiere torch >= 2.6, así que aunque se forzara torch 1.8.1, open_clip no arrancaría: brecha de ~5 años de versión. Además hay fallos arquitecturales en runtime (p. ej. `torch_shm_manager` ausente en el sandbox de ficheros de Android, issue #247). [confirmado]

**Matización verificada (importante):** torch es el bloqueante más severo, pero NO el único. `faiss-cpu` tampoco tiene wheel/AAR oficial Android (los ports comunitarios usan FAISS 1.5.3 / NDK r19c de 2019, abandonados). `opencv-python` y `rapidocr-onnxruntime` (los paquetes pip) tampoco tienen vía Android: la vía correcta es OpenCV nativo (NDK) y los modelos PaddleOCR ONNX corriendo sobre ONNX Runtime. Es decir, casi todo el stack pip habría que sustituirlo por equivalentes nativos, no solo torch.

**Lo que SÍ es portable son los MODELOS, no el paquete Python.** OpenCLIP ViT-B/32 y PaddleOCR se exportan a ONNX y corren en Android sobre ONNX Runtime (hay demos públicas funcionando offline). La inviabilidad es del servicio Python on-device, no de la funcionalidad de escaneo.

Confirmación local del stack actual: `E:/projects/PokemonCardDetector/services/ml/requirements.txt` lista `opencv-python`, `rapidocr-onnxruntime`, `open_clip_torch`, `torch`, `faiss-cpu`.

---

## (c) Arquitectura on-device recomendada (mapeo componente a componente)

Runtime único de ML en el dispositivo: **ONNX Runtime 1.26.0** [confirmado, Maven Central] accedido desde Rust vía el crate **`ort` 2.0.0-rc.12** (publicado 2026-03-05) [confirmado, github.com/pykeio/ort/releases]. El crate distribuye binario prebuilt oficial para `aarch64-linux-android` (fila confirmada en `ort-sys/build/download/dist.txt`, apuntando a `cdn.pyke.io/.../ms@1.26.0/aarch64-linux-android.tar.lzma2`), y el bug bloqueante de toolchain #514 (lib compilada con GCC de Ubuntu en vez de NDK Clang) está confirmado arreglado.

| Componente actual | Tecnología hoy | Reemplazo móvil-nativo | Runtime / crate Rust | ¿Reutiliza o reescribe? |
|---|---|---|---|---|
| Detección de cuadrilátero | opencv-python (`findContours`, `approxPolyDP`) | OpenCV nativo C++ vía NDK/JNI, o el crate Rust `opencv` ligado a OpenCV4Android 4.12.0 | OpenCV4Android (C++/NDK) | Reescribe el binding (la lógica del algoritmo se reutiliza tal cual; es trivial) |
| OCR de texto | rapidocr-onnxruntime (PaddleOCR ONNX) | Modelos PaddleOCR PP-OCRv4/v5 mobile ONNX (det+rec+cls) sobre ORT; opción alternativa Google ML Kit Latin bundled | `ort` (XNNPACK EP) o ML Kit | Reescribe el wrapper; reutiliza los modelos ONNX |
| Embedding visual | open_clip ViT-B/32 (torch, 512d) | MobileCLIP2-S0 ONNX (vision encoder, 512d) o ViT-B/32 INT8 ONNX | `ort` (CPU EP) | Reescribe la inferencia; reutiliza/reexporta el modelo a ONNX |
| Búsqueda de similitud | faiss-cpu `IndexFlatIP` (coseno sobre embeddings normalizados) | Brute-force dot-product en Rust puro (ndarray + NEON), o `usearch` 2.25.2 si crece el catálogo | Rust puro / crate `usearch` | Reescribe (es un dot-product + argmax; trivial para ~20-40k vectores) |
| Metadata del catálogo | SQLite vía sqlx (en API axum) | SQLite vía sqlx en el core Rust de Tauri | `sqlx` (sqlite) | Reutiliza casi tal cual (mismo crate, mismas tablas) |
| API de negocio | axum :8787 (servidor HTTP) | Funciones Rust llamadas por comandos `invoke` de Tauri | `tauri::command` + `serde` | Reescribe la capa de transporte (de HTTP a invoke); reutiliza la lógica de negocio y queries |
| Frontend | React en webview Tauri | El MISMO React, pero llamando `invoke()` en vez de `fetch` | Tauri JS API (`@tauri-apps/api`) | Reutiliza casi tal cual (solo cambia la capa de llamadas) |

Notas técnicas confirmadas que condicionan el diseño:
- ViT/transformers corren en **CPU EP**, no acelerados por XNNPACK (XNNPACK no cubre los operadores de atención/layer-norm). La aceleración NPU real exige QNN EP (solo Qualcomm Snapdragon, plugin separado `onnxruntime-android-qnn`). [confirmado]
- **NNAPI está deprecado en Android 15** (agosto 2024); no apoyarse en él para desarrollo nuevo. CPU EP y XNNPACK EP siguen disponibles y son suficientes. [confirmado]
- `ort` permite cargar modelos desde bytes en RAM con `Session::builder()?.commit_from_memory(&bytes)`, ideal para leer assets de Android sin escribir a disco. [confirmado]

---

## (d) Cómo encaja en Tauri 2 Android

Tauri 2 es estable (v2.11.2 a junio 2026) y Android es target oficial desde v2.0 (2 oct 2024) [confirmado, v2.tauri.app/release]. El patrón es: toda la inferencia y la lógica de negocio viven en el **core Rust** (la `app_lib` ya declarada en `apps/desktop/src-tauri/Cargo.toml` con `crate-type = ["staticlib","cdylib","rlib"]`, exactamente el patrón móvil de Tauri). El frontend React llama por `invoke`, NO por `fetch` a localhost.

NO usar `tauri-plugin-localhost`: la doc oficial lo marca como riesgo de seguridad considerable y añade complejidad innecesaria. El mecanismo correcto es `invoke` + comandos. [confirmado]

Ejemplo mínimo de comando Tauri (Rust):

```rust
// src-tauri/src/scan.rs
use serde::Serialize;

#[derive(Serialize)]
pub struct CardMatch {
    pub card_id: String,
    pub name: String,
    pub score: f32,
}

/// Recibe los bytes de la foto (JPEG/PNG) tomada en el frontend,
/// detecta el cuadrilatero, recorta, calcula el embedding CLIP,
/// busca el top-k por coseno y devuelve los candidatos.
#[tauri::command]
async fn identify_card(image: Vec<u8>) -> Result<Vec<CardMatch>, String> {
    // La inferencia es bloqueante y pesada: ejecutarla fuera del hilo
    // principal para no provocar ANR en Android.
    tokio::task::spawn_blocking(move || {
        let cropped = detect_and_crop(&image).map_err(|e| e.to_string())?;
        let embedding = embed_clip(&cropped).map_err(|e| e.to_string())?; // ort
        let matches = search_topk(&embedding, 5);                          // dot-product
        Ok(matches)
    })
    .await
    .map_err(|e| e.to_string())?
}
```

Registro del comando (en `lib.rs`):

```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![identify_card])
    .run(tauri::generate_context!())
    .expect("error al arrancar la app");
```

Llamada desde el cliente React (TypeScript):

```ts
import { invoke } from "@tauri-apps/api/core";

async function scan(photoBytes: Uint8Array) {
  const matches = await invoke<CardMatch[]>("identify_card", {
    image: Array.from(photoBytes),
  });
  return matches; // [{ card_id, name, score }, ...]
}
```

Cámara: `getUserMedia()` funciona en el WebView de Android pero NO out-of-the-box. Requiere (1) un `WebChromeClient` personalizado en `MainActivity.kt` (implementar `onPermissionRequest`), (2) permisos `CAMERA` / `RECORD_AUDIO` / `MODIFY_AUDIO_SETTINGS` en `AndroidManifest.xml`, y (3) `https://tauri.localhost` en la CSP. Documentado en la discusión oficial #12732. [confirmado]

---

## (e) Presupuesto de tamaño de APK y rendimiento esperado

Modelos y datos embarcados (cifras por componente):

| Pieza | Tamaño | Marca |
|---|---|---|
| Vision encoder MobileCLIP2-S0 ONNX (512d) | 43 MB | [confirmado, plhery/mobileclip2-onnx] |
| Alternativa: ViT-B/32 image encoder INT8 ONNX | 91.2 MB | [confirmado, greyovo/CLIP-android-demo] |
| Modelos PaddleOCR PP-OCRv4 mobile EN (det+rec+cls) | ~10.7 MB | [confirmado, suma HuggingFace SWHL/RapidOCR + breezedeus] |
| Alternativa OCR: ML Kit Latin bundled | ~4 MB | [confirmado, Google Developers] |
| Índice de embeddings catálogo (20k cartas x 512d FP32) | ~40 MB | [estimado, cálculo 20000·512·4 bytes = 40.96 MB] |
| Índice catálogo completo (40k x 512d FP32) | ~82 MB | [confirmado por cálculo 40000·512·4 = 81.92 MB] |
| Índice INT8 (40k x 512d) | ~20 MB | [confirmado por cálculo 40000·512·1 = 20.48 MB] |
| Runtime ONNX (binario nativo en APK) | ~10-15 MB | [estimado, overhead típico ORT Mobile] |
| APK Tauri base (sin assets) | ~45 MB | [estimado, de issue beta #8893; puede haber variado] |
| Metadata SQLite (~20k cartas, texto) | ~10-30 MB | [estimado, depende de campos guardados] |

**APK total estimado (MVP, MobileCLIP2-S0 + OCR ML Kit + índice 20k FP32):** ~45 (base) + 43 (CLIP) + 4 (OCR) + 40 (índice) + ~15 (ORT) + ~20 (DB) ≈ **~165 MB** [estimado, suma de las cifras anteriores]. Cabe holgadamente en el módulo base AAB (límite 500 MB comprimido) [confirmado, Google Play].

Rendimiento de inferencia (latencia por imagen, CPU, single-thread):
- ViT-B/32 INT8 en Snapdragon 8+ Gen 1 (Xiaomi 12S): **35-45 ms/imagen** [confirmado, greyovo/CLIP-android-demo].
- ViT-B/32 FP32: **~108-124 ms/imagen** [estimado, ratio 2-3x sobre INT8].
- MobileCLIP2-S0 en Android ONNX CPU: **~5-15 ms/imagen** [estimado, ratio frente a ViT-B/32; las cifras del paper de MobileCLIP (1.5-10 ms) son iOS CoreML y NO extrapolables a Android CPU].
- Búsqueda brute-force coseno sobre 40k x 512d FP32 en ARM mid-range: **~5-20 ms/query** [estimado, memory-bound; ~41 MFLOP/query, lectura de ~80 MB RAM].

En gama media/baja, multiplicar la latencia de CLIP por 2-4x respecto a la gama alta [estimado, sin fuente directa].

---

## (f) Estrategia para las imágenes de cartas offline

Imágenes de TCGdex: high 600x825 y low 245x337, ambas WebP, patrón `https://assets.tcgdex.net/{lang}/{set}/{setId}/{cardNum}/{quality}.webp` [confirmado, tcgdex.dev/assets].

**Recomendación elegida (MVP):** NO bundlear las imágenes en el APK. Embarcar solo metadata + embeddings precomputados. Las miniaturas WebP 245x337 se descargan lazy en el primer uso y se cachean en disco local (Room/cache de imágenes). Tamaño estimado ~10-25 KB por miniatura; 40k miniaturas ≈ ~800 MB en caché en disco [estimado, TCGdex no publica tamaños de archivo]. Esto mantiene el APK pequeño y solo necesita red la primera vez que se muestra cada carta (la identificación en sí ya es 100% offline porque usa embeddings, no imágenes).

**Alternativas:**
- **Play Asset Delivery (install-time pack):** empaquetar el modelo y/o un subconjunto de miniaturas como asset pack si el AAB base supera 500 MB. Límite por pack hasta ~512 MB / 1.5 GB según tipo; acumulado install-time 4 GB; on-demand 30 GB [confirmado, Google Play]. Útil si se quiere "expansión X totalmente offline" descargable bajo demanda.
- **Bundlear todo en el APK:** inviable (cientos de MB a GB de imágenes). Descartado.
- **Resolución high bajo demanda:** servir 245x337 por defecto y descargar 600x825 solo al abrir el detalle de una carta.

Identificación pura offline garantizada: solo necesita los embeddings del catálogo (embarcados) + el modelo (embarcado). Las imágenes son únicamente para mostrar resultados, no para identificar.

---

## (g) Tres opciones de producto comparadas

| Criterio | A) Android 100% on-device offline | B) Híbrido (cliente Tauri Android + servidor Python remoto PC/VPS) | C) MVP móvil (MobileCLIP cuantizado + catálogo acotado) |
|---|---|---|---|
| Privacidad / local-first | Máxima (todo en el dispositivo) | Baja (las fotos salen del dispositivo) | Alta (todo on-device) |
| Funciona sin internet para identificar | Sí | No (requiere servidor) | Sí |
| Reutiliza el servicio Python actual | No (reescribe en Rust) | Sí (se mantiene tal cual) | No (reescribe parcial) |
| Esfuerzo de ingeniería | Alto | Bajo-medio | Medio |
| Tamaño APK | ~165 MB [estimado] | Pequeño (~45-60 MB, sin modelos) | ~100-130 MB (catálogo acotado) [estimado] |
| Coste de operación | Cero (sin backend) | Recurrente (hosting servidor) | Cero |
| Precisión | Alta (ViT-B/32) o buena (MobileCLIP2) | Alta (stack actual completo) | Buena (cuantizado + catálogo parcial) |
| Cumple restricciones del usuario (privacidad, sin pago obligatorio, offline) | Sí, totalmente | No (rompe privacidad y/o introduce coste) | Sí |
| Riesgo técnico | Medio-alto (ort Android joven; cross-compile) | Bajo | Medio |

**Recomendación: empezar por C como camino hacia A.** La opción **C (MVP móvil con MobileCLIP2-S0 cuantizado y catálogo acotado a un idioma / un subconjunto de expansiones)** es la primera entrega; converge naturalmente en **A (100% on-device offline con catálogo completo)** ampliando el índice. **Razón:** A es el objetivo correcto según las restricciones firmes del usuario (privacidad y local-first, sin servicios de pago, offline), pero su catálogo completo (~20k/idioma) y el endurecimiento del cross-compile de `ort` para Android son los puntos de mayor riesgo. C entrega valor offline real con menor superficie de riesgo (catálogo pequeño = índice pequeño + descarga de imágenes acotada) y valida el pipeline nativo completo antes de escalar. **B se descarta como destino** porque rompe la restricción firme de privacidad/offline e introduce coste recurrente; solo es aceptable como andamiaje temporal de desarrollo (reusar el servicio Python para generar/validar embeddings del catálogo en PC), nunca como producto final.

---

## (h) Plan de migración por fases (esfuerzo relativo)

Punto de partida: app de escritorio cliente-servidor (React + axum :8787 + Python FastAPI :8001), índice de prueba de 504 cartas.

| Fase | Objetivo | Esfuerzo relativo |
|---|---|---|
| 0. Preparación de modelos | Exportar/obtener ONNX: MobileCLIP2-S0 vision (o ViT-B/32 INT8) + PaddleOCR mobile. Recomputar embeddings del catálogo con el modelo elegido (en PC, reusando el servicio Python como herramienta offline de build, NO como servidor de producción). | Bajo |
| 1. Núcleo Rust de inferencia (en desktop) | Integrar `ort` 2.0.0-rc.12 + OpenCV + búsqueda brute-force en el core Rust. Validar paridad de resultados contra el servicio Python en escritorio (mismo hardware, fácil de depurar). | Alto |
| 2. Mover negocio a comandos invoke | Reescribir la capa axum HTTP como `tauri::command`; el frontend React pasa de `fetch` a `invoke`. SQLite/sqlx se reutiliza casi tal cual. Apagar el servidor Python en desktop. | Medio |
| 3. Bring-up Android | `tauri android init`, cargo-ndk + NDK, cross-compile de `ort` para `aarch64-linux-android` (estrategia `download-binaries`, verificar fix #514). Empaquetar modelos como assets y cargarlos con `commit_from_memory`. | Alto |
| 4. Cámara + permisos | `WebChromeClient` en `MainActivity.kt`, permisos en `AndroidManifest.xml`, CSP `tauri.localhost`. Pipeline foto -> recorte -> embedding -> top-k end-to-end en dispositivo. | Medio |
| 5. MVP offline (Opción C) | Catálogo acotado (un idioma / subconjunto), índice embarcado, miniaturas lazy + cache. Medir latencia en gama media/baja real. | Medio |
| 6. Escalar a A | Ampliar a catálogo completo (~20k/idioma), evaluar `usearch` si la búsqueda brute-force se queda corta (>100k vectores), Play Asset Delivery si el AAB supera 500 MB. | Medio-alto |

Las fases 1 y 2 se hacen en escritorio (depuración fácil) antes de tocar Android. La fase 3 es la de mayor incertidumbre.

---

## (i) Riesgos técnicos específicos de móvil y mitigaciones

| Riesgo | Detalle | Mitigación |
|---|---|---|
| Tamaño de APK | Modelos FP32 (335 MB el image encoder ViT-B/32) [confirmado] inflan el APK | Usar MobileCLIP2-S0 (43 MB) o INT8 (91 MB); índice INT8 (20 MB); imágenes fuera del APK; Play Asset Delivery si >500 MB |
| Latencia en gama baja | En mid/low-end la latencia de CLIP puede ser 2-4x la de gama alta [estimado] | Preferir MobileCLIP2-S0; inferencia en `spawn_blocking`; mostrar spinner; cuantizar; permitir cancelar |
| Precisión tras cuantización | INT8 dinámico degrada retrieval típicamente 1-5% (4.6% reportado ViT-B/32 CIFAR100) [confirmado] | Devolver top-3/top-5 en vez de top-1; combinar con OCR (nombre/número de carta) para desempatar; medir recall@k en el dominio real de cartas |
| Calor / batería | Inferencia CPU sostenida calienta y consume; ráfagas de escaneo continuo | Inferencia bajo demanda (no continua); throttling entre escaneos; modelo ligero (MobileCLIP2-S0); evitar FP32 |
| Permisos de cámara en WebView | `getUserMedia` no funciona out-of-the-box; doble capa de permisos Android | `WebChromeClient` personalizado (#12732) + permisos en manifest + CSP `tauri.localhost`; fallback `input[type=file capture=environment]` (no confirmado en Tauri, pero es del WebView) |
| `ort` Android joven | Soporte anunciado en rc.11 (ene 2026), rc.12 no es API-stable; bug #514 reciente | Fijar versión exacta de `ort` y de ONNX Runtime; verificar fix #514 en el binario usado; tener `tract` (Rust puro) como fallback para modelos simples; tests de regresión vs desktop |
| `resource_dir()` en Android | Bug #11823: devuelve `asset://localhost`, no ruta real; puede colgarse | Copiar assets a data/cache dir en primer arranque vía fs plugin, o cargar con `commit_from_memory` desde bytes del asset |
| ANR por bloqueo de UI | Comandos nativos Android corren en hilo principal por defecto; cargar modelo / inferir bloquea | Ejecutar inferencia y carga de modelo en `tokio::task::spawn_blocking` / coroutines (`Dispatchers.IO`) |
| Track record de Tauri Android | Poca evidencia pública de apps Tauri Android a gran escala en producción | Validar en dispositivos reales pronto (fase 3-5); no asumir paridad de DX con desktop |

---

## (j) Fuentes clave

ONNX Runtime / crate `ort`:
- https://github.com/pykeio/ort/releases
- https://github.com/pykeio/ort/blob/main/ort-sys/build/download/dist.txt
- https://github.com/pykeio/ort/issues/514
- https://docs.rs/crate/ort/latest/features
- https://onnxruntime.ai/docs/build/android.html
- https://onnxruntime.ai/docs/execution-providers/Xnnpack-ExecutionProvider.html
- https://onnxruntime.ai/docs/execution-providers/NNAPI-ExecutionProvider.html
- https://central.sonatype.com/artifact/com.microsoft.onnxruntime/onnxruntime-android
- https://developer.android.com/ndk/guides/neuralnetworks/migration-guide

Inviabilidad del stack Python en Android:
- https://pypi.org/project/torch/#files
- https://github.com/chaquo/chaquopy/issues/1215
- https://github.com/chaquo/chaquopy/issues/1376
- https://github.com/chaquo/chaquopy/issues/247
- https://pypi.org/project/open-clip-torch/
- https://github.com/faiss-wheels/faiss-wheels
- https://docs.pytorch.org/executorch/0.7/using-executorch-android.html

Modelos (CLIP / MobileCLIP / OCR) en Android:
- https://github.com/greyovo/CLIP-android-demo
- https://huggingface.co/plhery/mobileclip2-onnx
- https://huggingface.co/Xenova/mobileclip_blt/blob/main/onnx/vision_model_quantized.onnx
- https://huggingface.co/apple/MobileCLIP-S0/tree/main
- https://machinelearning.apple.com/research/mobileclip2
- https://github.com/RapidAI/RapidOCR
- https://github.com/RapidAI/RapidOcrAndroidOnnx
- https://huggingface.co/SWHL/RapidOCR
- https://developers.google.com/ml-kit/vision/text-recognition/v2/android

Búsqueda vectorial e imágenes:
- https://github.com/unum-cloud/usearch
- https://crates.io/crates/usearch
- https://tcgdex.dev/assets

Tauri 2 Android:
- https://v2.tauri.app/release/
- https://v2.tauri.app/blog/tauri-20/
- https://github.com/tauri-apps/tauri/discussions/12732
- https://github.com/tauri-apps/tauri/issues/11823
- https://v2.tauri.app/develop/resources/
- https://v2.tauri.app/develop/plugins/develop-mobile/
- https://v2.tauri.app/plugin/localhost/
- https://v2.tauri.app/distribute/google-play/
- https://support.google.com/googleplay/android-developer/answer/9859372
- https://developer.android.com/guide/playcore/asset-delivery
