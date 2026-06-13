/// Punto de entrada compartido entre escritorio y movil.
///
/// En plataformas moviles, `tauri::mobile_entry_point` genera el glue code
/// necesario (JNI en Android, bindings en iOS) para arrancar la app.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicacion Tauri");
}
