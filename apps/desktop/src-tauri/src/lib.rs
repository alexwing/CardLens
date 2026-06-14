//! Shell de escritorio/movil de CardLens.
//!
//! - **Escritorio:** al arrancar lanza la API de negocio como binario *sidecar*
//!   empaquetado y la cierra al salir, de modo que el ejecutable es autonomo.
//!   En desarrollo (`tauri dev`) el sidecar no esta empaquetado: si no se
//!   encuentra, se asume que la API se arranca aparte.
//! - **Android:** no se puede lanzar un sidecar; la API se ejecuta **en
//!   proceso** (servidor axum en un hilo con su propio runtime tokio). Antes
//!   se copian los modelos/indice/DB desde los assets de la APK al directorio
//!   de datos escribible de la app.

use tauri::Manager;

#[cfg(desktop)]
use std::path::PathBuf;
#[cfg(desktop)]
use std::process::{Child, Command};
#[cfg(desktop)]
use std::sync::Mutex;
#[cfg(desktop)]
use tauri::RunEvent;

/// Guarda el proceso de la API sidecar (escritorio) para cerrarlo al salir.
#[cfg(desktop)]
struct ApiProcess(Mutex<Option<Child>>);

/// Nombre del binario sidecar de la API (sin extension).
#[cfg(desktop)]
const API_BIN: &str = "cardlens-api";
/// Puerto en el que escucha la API (debe coincidir con VITE_API_URL del front).
const API_PORT: u16 = 8787;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(desktop)]
    let builder = builder.manage(ApiProcess(Mutex::new(None)));

    builder
        .setup(|app| {
            #[cfg(desktop)]
            {
                match start_sidecar_api(app) {
                    Ok(true) => eprintln!("[cardlens] API embebida (sidecar) arrancada"),
                    Ok(false) => {
                        eprintln!("[cardlens] sidecar no encontrado (dev: arranca la API aparte)")
                    }
                    Err(err) => eprintln!("[cardlens] no se pudo arrancar la API sidecar: {err}"),
                }
            }
            #[cfg(target_os = "android")]
            {
                let _ = app; // por si algun cfg no lo usa
                match start_inprocess_api(app) {
                    Ok(()) => eprintln!("[cardlens] API en proceso arrancada"),
                    Err(err) => eprintln!("[cardlens] no se pudo arrancar la API en proceso: {err}"),
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error al ejecutar la aplicacion Tauri")
        .run(move |_app_handle, _event| {
            #[cfg(desktop)]
            if matches!(_event, RunEvent::Exit) {
                stop_sidecar_api(_app_handle);
            }
        });
}

// ---------------------------------------------------------------------------
// Escritorio: API como binario sidecar.
// ---------------------------------------------------------------------------

/// Arranca la API sidecar con las rutas a los recursos empaquetados. Devuelve
/// Ok(false) si no hay sidecar (desarrollo), Ok(true) si se lanzo.
#[cfg(desktop)]
fn start_sidecar_api(app: &tauri::App) -> Result<bool, Box<dyn std::error::Error>> {
    // El sidecar se instala junto al ejecutable principal.
    let exe_dir = std::env::current_exe()?
        .parent()
        .map(PathBuf::from)
        .ok_or("no se pudo resolver el directorio del ejecutable")?;
    let sidecar = exe_dir.join(if cfg!(windows) {
        "cardlens-api.exe"
    } else {
        API_BIN
    });
    if !sidecar.exists() {
        return Ok(false);
    }

    // Recursos empaquetados (modelo, indices, OCR, DLL de ONNX, DB del catalogo).
    let resources = app.path().resource_dir()?.join("resources");
    let res = |rel: &str| resources.join(rel);

    // Directorio de datos del usuario (escribible): DB de trabajo, scans.
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(data_dir.join("scans"))?;
    std::fs::create_dir_all(data_dir.join("images"))?;

    // En la primera ejecucion, copia la DB del catalogo (recurso de solo
    // lectura) al directorio de datos del usuario, donde se podra escribir.
    let db_path = data_dir.join("app.db");
    if !db_path.exists() {
        let seed = res("app.db");
        if seed.exists() {
            std::fs::copy(&seed, &db_path)?;
        }
    }

    let child = Command::new(&sidecar)
        .env("API_PORT", API_PORT.to_string())
        .env("DATA_DIR", &data_dir)
        .env("DATABASE_PATH", &db_path)
        .env("ORT_DYLIB_PATH", res("onnxruntime.dll"))
        .env("MODEL_PATH", res("mobileclip2_s0/vision_model.onnx"))
        .env("INDEX_BIN_PATH", res("index/mobileclip.bin"))
        .env("INDEX_CARDS_PATH", res("index/mobileclip_cards.json"))
        .env("OCR_DET_PATH", res("ocrs/text-detection.rten"))
        .env("OCR_REC_PATH", res("ocrs/text-recognition.rten"))
        .spawn()?;

    if let Some(state) = app.try_state::<ApiProcess>() {
        *state.0.lock().unwrap() = Some(child);
    }
    Ok(true)
}

/// Cierra la API sidecar (si la hay) al salir de la app.
#[cfg(desktop)]
fn stop_sidecar_api(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<ApiProcess>() {
        if let Ok(mut guard) = state.0.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Android: API en proceso (sin sidecar).
// ---------------------------------------------------------------------------

/// Arranca el servidor axum de la API dentro del propio proceso de la app, en
/// un hilo con su runtime tokio. Antes asegura que los modelos/indice/DB estan
/// en el directorio de datos (copiados desde los assets de la APK).
#[cfg(target_os = "android")]
fn start_inprocess_api(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(data_dir.join("scans"))?;
    std::fs::create_dir_all(data_dir.join("images"))?;

    // Copia modelos/indice/OCR/DB desde los assets de la APK la primera vez.
    if let Err(err) = ensure_resources(app, &data_dir) {
        eprintln!("[cardlens] aviso: no se pudieron preparar los recursos: {err}");
        // Seguimos: la API arranca en modo degradado (sin reconocedor) y la
        // app es usable (coleccion, salud) mientras depuramos los assets.
    }

    // ort carga libonnxruntime.so (empaquetado en jniLibs) por su soname; el
    // linker de Android lo encuentra en el directorio de librerias nativas.
    std::env::set_var("ORT_DYLIB_PATH", "libonnxruntime.so");

    let config = pokemon_card_api::config::Config::embedded(data_dir, API_PORT);
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(err) => {
                eprintln!("[cardlens] no se pudo crear el runtime tokio: {err}");
                return;
            }
        };
        if let Err(err) = rt.block_on(pokemon_card_api::serve(config)) {
            eprintln!("[cardlens] la API en proceso termino con error: {err}");
        }
    });

    Ok(())
}

/// Copia los recursos pesados (modelo MobileCLIP, indice, OCR y DB del
/// catalogo) desde los assets de la APK al directorio de datos escribible, solo
/// si aun no estan. En Android los assets no son rutas de fichero: se leen con
/// el AssetManager. (Stub en la 1a fase de integracion; se completa al meter
/// los assets pesados.)
#[cfg(target_os = "android")]
fn ensure_resources(_app: &tauri::App, _data_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    // TODO(android, fase assets): copiar desde assets via AssetManager:
    //   cardlens/mobileclip2_s0/vision_model.onnx -> data_dir/...
    //   cardlens/index/mobileclip.bin, mobileclip_cards.json
    //   cardlens/ocrs/text-detection.rten, text-recognition.rten
    //   cardlens/app.db
    Ok(())
}
