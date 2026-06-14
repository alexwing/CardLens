package xyz.mappuzzle.cardlens

import android.os.Bundle
import androidx.activity.enableEdgeToEdge
import java.io.File

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // Copia modelos/indice/OCR/DB desde los assets de la APK al directorio de
    // datos de la app (el mismo que usa la API en proceso via app_data_dir).
    // Debe ocurrir ANTES de super.onCreate, que es quien arranca el lado Rust
    // (la API en proceso lee esos ficheros por ruta al cargar el reconocedor).
    // En Android los assets no son rutas de fichero: se leen con AssetManager.
    stageBundledData()
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  /**
   * Extrae los recursos pesados empaquetados en assets/cardlens/ a dataDir, una
   * sola vez (marcador por version). AssetManager solo puede abrir assets
   * grandes si estan sin comprimir (ver androidResources.noCompress en Gradle).
   */
  private fun stageBundledData() {
    val files = listOf(
      "app.db",
      "mobileclip2_s0/vision_model.onnx",
      "index/mobileclip.bin",
      "index/mobileclip_cards.json",
      "ocrs/text-detection.rten",
      "ocrs/text-recognition.rten"
    )
    val base = applicationContext.dataDir // == app_data_dir() de Tauri en Android
    val marker = File(base, ".assets_v1")
    if (marker.exists()) return // ya extraido en un arranque previo

    try {
      for (rel in files) {
        val dst = File(base, rel)
        dst.parentFile?.mkdirs()
        assets.open("cardlens/$rel").use { input ->
          dst.outputStream().use { output -> input.copyTo(output, 1 shl 20) }
        }
      }
      // Limpia WAL/SHM de una DB anterior para que la nueva app.db sea coherente.
      File(base, "app.db-wal").delete()
      File(base, "app.db-shm").delete()
      marker.writeText("1")
    } catch (e: Exception) {
      android.util.Log.e("cardlens", "fallo al extraer assets: ${e.message}", e)
    }
  }
}
