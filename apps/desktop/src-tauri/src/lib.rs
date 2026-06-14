//! Shell de escritorio/movil de CardLens.
//!
//! En escritorio, al arrancar lanza la API de negocio (binario sidecar
//! empaquetado) y la cierra al salir, de modo que el ejecutable es autonomo
//! (no hay que arrancar nada a mano). En desarrollo (`tauri dev`) el sidecar
//! no esta empaquetado: si no se encuentra, se asume que la API se arranca
//! aparte y la app sigue igualmente.

use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;

use tauri::{Manager, RunEvent};

/// Guarda el proceso de la API embebida para poder cerrarlo al salir.
struct ApiProcess(Mutex<Option<Child>>);

/// Nombre del binario sidecar de la API (sin extension).
const API_BIN: &str = "cardlens-api";
/// Puerto en el que escucha la API (debe coincidir con VITE_API_URL del front).
const API_PORT: &str = "8787";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ApiProcess(Mutex::new(None)))
        .setup(|app| {
            match start_embedded_api(app) {
                Ok(true) => eprintln!("[cardlens] API embebida arrancada"),
                Ok(false) => {
                    eprintln!("[cardlens] sidecar no encontrado (modo dev: arranca la API aparte)")
                }
                Err(err) => eprintln!("[cardlens] no se pudo arrancar la API embebida: {err}"),
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error al ejecutar la aplicacion Tauri")
        .run(|app_handle, event| {
            if matches!(event, RunEvent::Exit) {
                stop_embedded_api(app_handle);
            }
        });
}

/// Arranca la API sidecar con las rutas a los recursos empaquetados. Devuelve
/// Ok(false) si no hay sidecar (desarrollo), Ok(true) si se lanzo.
fn start_embedded_api(app: &tauri::App) -> Result<bool, Box<dyn std::error::Error>> {
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
    // lectura) al directorio de datos del usuario, donde se podra escribir
    // (colecciones, scans, precios).
    let db_path = data_dir.join("app.db");
    if !db_path.exists() {
        let seed = res("app.db");
        if seed.exists() {
            std::fs::copy(&seed, &db_path)?;
        }
    }

    let child = Command::new(&sidecar)
        .env("API_PORT", API_PORT)
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

/// Cierra la API embebida (si la hay) al salir de la app.
fn stop_embedded_api(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<ApiProcess>() {
        if let Ok(mut guard) = state.0.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
            }
        }
    }
}
