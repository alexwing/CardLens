//! OCR de la carta en Rust puro (crate `ocrs`, runtime `rten`, sin nativos).
//!
//! Lee el texto visible de la carta (idealmente ya recortada por el detector)
//! para reforzar el reconocimiento visual: el nombre y el numero permiten
//! desambiguar entre artes parecidos. Solo se compila con la feature `ocr`.

use std::path::Path;

use image::DynamicImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

/// Lector OCR: envuelve un `OcrEngine` con sus modelos de deteccion y
/// reconocimiento de texto.
pub struct OcrReader {
    engine: OcrEngine,
}

impl OcrReader {
    /// Crea el lector a partir de los dos modelos `.rten` (deteccion y
    /// reconocimiento de texto).
    pub fn from_files(detection: &Path, recognition: &Path) -> anyhow::Result<Self> {
        let detection_model =
            Model::load_file(detection).map_err(|e| anyhow::anyhow!("modelo OCR deteccion: {e}"))?;
        let recognition_model = Model::load_file(recognition)
            .map_err(|e| anyhow::anyhow!("modelo OCR reconocimiento: {e}"))?;
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(detection_model),
            recognition_model: Some(recognition_model),
            ..Default::default()
        })
        .map_err(|e| anyhow::anyhow!("creando OcrEngine: {e}"))?;
        Ok(Self { engine })
    }

    /// Lee solo la banda superior de la carta (donde esta el nombre). Mucho
    /// mas rapido que OCR de toda la carta: evita reconocer ataques, habilidad
    /// y flavor text, que no aportan a la identificacion.
    pub fn read_name_region(&self, img: &DynamicImage) -> anyhow::Result<String> {
        let (w, h) = (img.width(), img.height());
        let band_h = ((h as f32) * 0.24).round().max(1.0) as u32;
        let region = img.crop_imm(0, 0, w, band_h);
        self.read_text(&region)
    }

    /// Devuelve todo el texto reconocido en la imagen, en orden de lectura
    /// (lineas separadas por salto de linea).
    pub fn read_text(&self, img: &DynamicImage) -> anyhow::Result<String> {
        let rgb = img.to_rgb8();
        let (w, h) = (rgb.width(), rgb.height());
        let source = ImageSource::from_bytes(rgb.as_raw(), (w, h))
            .map_err(|e| anyhow::anyhow!("ImageSource: {e}"))?;
        let input = self
            .engine
            .prepare_input(source)
            .map_err(|e| anyhow::anyhow!("prepare_input OCR: {e}"))?;
        let text = self
            .engine
            .get_text(&input)
            .map_err(|e| anyhow::anyhow!("get_text OCR: {e}"))?;
        Ok(text)
    }
}
