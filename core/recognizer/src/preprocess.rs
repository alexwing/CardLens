//! Preprocesado de imagen para el encoder visual.
//!
//! Replica la transformacion estandar de CLIP/MobileCLIP: redimension al
//! lado del modelo, recorte central, conversion a tensor CHW en `f32` y
//! normalizacion canal a canal con la media y desviacion de OpenAI CLIP.
//!
//! NOTA: la media/desviacion y el tamano de entrada deben coincidir con los
//! que espera el ONNX concreto que se embarque. Los valores por defecto son
//! los de OpenAI CLIP, que MobileCLIP reutiliza; el tamano se pasa como
//! parametro porque varia por variante (MobileCLIP2-S0 usa 256).

use image::{imageops::FilterType, DynamicImage};
use ndarray::Array4;

/// Media de normalizacion de OpenAI CLIP (RGB).
pub const CLIP_MEAN: [f32; 3] = [0.481_454_66, 0.457_827_5, 0.408_210_73];
/// Desviacion tipica de normalizacion de OpenAI CLIP (RGB).
pub const CLIP_STD: [f32; 3] = [0.268_629_54, 0.261_302_58, 0.275_777_11];

/// Configuracion del preprocesado, dependiente de la variante del modelo.
#[derive(Debug, Clone)]
pub struct PreprocessConfig {
    /// Lado (px) de la imagen cuadrada que espera el modelo.
    pub size: u32,
    pub mean: [f32; 3],
    pub std: [f32; 3],
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self::mobileclip2_s0()
    }
}

impl PreprocessConfig {
    /// Preprocesado de MobileCLIP2-S0 segun su `preprocessor_config.json`:
    /// entrada 256x256, recorte central, y SIN normalizacion CLIP
    /// (mean=0, std=1, es decir solo escala a [0,1]). Importante: este modelo
    /// NO usa la media/desviacion de OpenAI CLIP.
    pub fn mobileclip2_s0() -> Self {
        Self {
            size: 256,
            mean: [0.0, 0.0, 0.0],
            std: [1.0, 1.0, 1.0],
        }
    }

    /// Preprocesado clasico de OpenAI CLIP (ViT-B-32 y similares): tamano
    /// configurable y normalizacion con la media/desviacion de CLIP.
    pub fn openai_clip(size: u32) -> Self {
        Self {
            size,
            mean: CLIP_MEAN,
            std: CLIP_STD,
        }
    }
}

/// Convierte una imagen ya decodificada en el tensor de entrada del modelo:
/// forma `[1, 3, size, size]`, RGB, normalizado.
///
/// Redimensiona el lado menor a `size` (preservando proporcion) y recorta el
/// centro, igual que la transform `Resize + CenterCrop` de CLIP.
pub fn image_to_tensor(img: &DynamicImage, cfg: &PreprocessConfig) -> Array4<f32> {
    let size = cfg.size;
    let rgb = img.to_rgb8();
    let (w, h) = (rgb.width(), rgb.height());

    // Resize del lado menor a `size`, preservando proporcion.
    let (rw, rh) = if w <= h {
        (size, ((h as f32) * (size as f32 / w as f32)).round() as u32)
    } else {
        (((w as f32) * (size as f32 / h as f32)).round() as u32, size)
    };
    let resized = DynamicImage::ImageRgb8(rgb).resize_exact(rw, rh, FilterType::CatmullRom);

    // Recorte central a size x size.
    let x0 = (rw.saturating_sub(size)) / 2;
    let y0 = (rh.saturating_sub(size)) / 2;
    let cropped = resized.crop_imm(x0, y0, size, size).to_rgb8();

    // A tensor CHW normalizado.
    let mut tensor = Array4::<f32>::zeros((1, 3, size as usize, size as usize));
    for (x, y, px) in cropped.enumerate_pixels() {
        let (xi, yi) = (x as usize, y as usize);
        for c in 0..3 {
            let v = px[c] as f32 / 255.0;
            tensor[[0, c, yi, xi]] = (v - cfg.mean[c]) / cfg.std[c];
        }
    }
    tensor
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn produces_expected_shape() {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(400, 600, Rgb([128, 64, 32])));
        let cfg = PreprocessConfig {
            size: 224,
            ..Default::default()
        };
        let t = image_to_tensor(&img, &cfg);
        assert_eq!(t.shape(), &[1, 3, 224, 224]);
    }

    #[test]
    fn normalization_is_applied() {
        // Imagen gris uniforme: cada canal debe dar el mismo valor normalizado
        // en todos los pixeles (v - mean) / std.
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(300, 300, Rgb([255, 0, 0])));
        let cfg = PreprocessConfig::default();
        let t = image_to_tensor(&img, &cfg);
        let expected_r = (1.0 - cfg.mean[0]) / cfg.std[0];
        let expected_g = (0.0 - cfg.mean[1]) / cfg.std[1];
        approx::assert_abs_diff_eq!(t[[0, 0, 10, 10]], expected_r, epsilon = 1e-5);
        approx::assert_abs_diff_eq!(t[[0, 1, 10, 10]], expected_g, epsilon = 1e-5);
    }
}
