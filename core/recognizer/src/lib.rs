//! Nucleo de reconocimiento de cartas Pokemon, compartido entre la app de
//! escritorio y la de Android.
//!
//! Pipeline on-device (sin servidor, sin internet para identificar):
//! 1. `preprocess`: imagen de camara/archivo -> tensor de entrada del modelo.
//! 2. `embedder` (feature `onnx`): tensor -> embedding visual con ONNX Runtime.
//! 3. `index`: embedding -> top-k cartas por similitud coseno (reemplaza FAISS).
//!
//! La metadata de catalogo y el OCR viven en capas superiores; este crate se
//! concentra en la parte portatil de la inferencia, que es identica en ambos
//! objetivos y se desarrolla/prueba primero en escritorio.

pub mod detector;
pub mod index;
pub mod preprocess;

#[cfg(feature = "onnx")]
pub mod embedder;

#[cfg(feature = "ocr")]
pub mod ocr;

pub use index::{l2_normalize, CardRef, FlatIndex, Match};
pub use preprocess::{image_to_tensor, PreprocessConfig};

/// Decodifica los bytes de una imagen y devuelve la carta recortada (detector);
/// si no se detecta carta, devuelve la imagen completa. Asi embedding y OCR
/// trabajan sobre la misma vista de la carta.
pub fn prepare_card(image_bytes: &[u8]) -> anyhow::Result<image::DynamicImage> {
    let img = image::load_from_memory(image_bytes)?;
    Ok(detector::detect_and_crop(&img).unwrap_or(img))
}

#[cfg(feature = "onnx")]
pub use embedder::Embedder;
