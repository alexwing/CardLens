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

pub use index::{l2_normalize, CardRef, FlatIndex, Match};
pub use preprocess::{image_to_tensor, PreprocessConfig};

#[cfg(feature = "onnx")]
pub use embedder::Embedder;
