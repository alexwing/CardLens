//! Encoder visual on-device basado en ONNX Runtime (crate `ort`).
//!
//! Carga un modelo ONNX (p. ej. MobileCLIP2-S0, encoder de imagen) y produce
//! un embedding L2-normalizado de 512 dimensiones a partir de una imagen.
//! El mismo codigo corre en escritorio (binarios prebuilt de ORT) y en
//! Android (ORT compilado con el NDK).
//!
//! Solo se compila con la feature `onnx`.

use crate::index::l2_normalize;
use crate::preprocess::{image_to_tensor, PreprocessConfig};
use anyhow::Context;
use image::DynamicImage;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;

/// `ort::Error` no implementa `std::error::Error`, asi que no encaja con
/// `?`/`.context()` de anyhow; lo convertimos por su mensaje.
fn oe(e: ort::Error) -> anyhow::Error {
    anyhow::anyhow!("ONNX Runtime: {e}")
}

/// Encoder visual: envuelve una sesion de ONNX Runtime.
pub struct Embedder {
    session: Session,
    cfg: PreprocessConfig,
    input_name: String,
}

impl Embedder {
    /// Crea el encoder a partir de los bytes del modelo ONNX (assets de la
    /// app en Android, o un archivo en escritorio) y la configuracion de
    /// preprocesado de la variante embarcada.
    pub fn from_bytes(model: &[u8], cfg: PreprocessConfig) -> anyhow::Result<Self> {
        let session = Session::builder()
            .map_err(oe)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(oe)?
            .with_intra_threads(num_threads())
            .map_err(oe)?
            .commit_from_memory(model)
            .map_err(oe)
            .context("cargando el modelo ONNX desde memoria")?;

        let input_name = session
            .inputs
            .first()
            .map(|i| i.name.clone())
            .context("el modelo ONNX no declara entradas")?;

        Ok(Self {
            session,
            cfg,
            input_name,
        })
    }

    /// Genera el embedding L2-normalizado de una imagen ya decodificada.
    pub fn embed(&mut self, img: &DynamicImage) -> anyhow::Result<Vec<f32>> {
        let tensor = image_to_tensor(img, &self.cfg);
        let value = Value::from_array(tensor)
            .map_err(oe)
            .context("construyendo el tensor de entrada")?;

        let outputs = self
            .session
            .run(ort::inputs![self.input_name.as_str() => value])
            .map_err(oe)
            .context("ejecutando la inferencia ONNX")?;

        let (_shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(oe)
            .context("extrayendo el embedding de salida")?;

        let mut embedding = data.to_vec();
        l2_normalize(&mut embedding);
        Ok(embedding)
    }

    /// Genera el embedding a partir de bytes de imagen sin decodificar
    /// (JPEG/PNG), tal como llegan de la camara o de un archivo.
    pub fn embed_bytes(&mut self, image_bytes: &[u8]) -> anyhow::Result<Vec<f32>> {
        let img = image::load_from_memory(image_bytes).context("decodificando la imagen")?;
        self.embed(&img)
    }
}

/// Numero de hilos de inferencia: limitado para no saturar moviles de gama
/// baja, pero aprovechando varios nucleos en gama alta y escritorio.
fn num_threads() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(2)
        .clamp(1, 4)
}
