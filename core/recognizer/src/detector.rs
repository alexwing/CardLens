//! Deteccion y recorte de la carta dentro de la foto, sin OpenCV nativo.
//!
//! Para fotos casi frontales (el caso de un escaneo deliberado) basta con
//! aislar la carta por su caja delimitadora: bordes Canny -> contornos -> el
//! contorno mayor con proporcion de carta -> recorte de su bounding box. Es
//! mucho mas robusto que rectificar un cuadrilatero por homografia (que se
//! distorsiona con bordes sueltos del fondo). El embedder reescala/recorta
//! despues igual que las imagenes del catalogo, asi que la comparacion es
//! consistente.
//!
//! Si no encuentra una carta plausible, devuelve `None` y el llamante usa la
//! imagen completa como fallback.

use image::{DynamicImage, GenericImageView};
use imageproc::contours::find_contours;
use imageproc::distance_transform::Norm;
use imageproc::edges::canny;
use imageproc::filter::gaussian_blur_f32;
use imageproc::morphology::dilate;

/// Lado mayor de la imagen de trabajo para la deteccion (px).
const WORK_MAX: u32 = 700;
/// Area minima/maxima de la caja como fraccion del encuadre de trabajo.
const MIN_AREA_FRAC: f64 = 0.10;
const MAX_AREA_FRAC: f64 = 0.95;
/// Margen que se anade alrededor de la caja (fraccion), para no comer el borde.
const MARGIN_FRAC: f32 = 0.015;

/// Detecta la carta y devuelve su recorte (bounding box), o `None`.
pub fn detect_and_crop(img: &DynamicImage) -> Option<DynamicImage> {
    let (orig_w, orig_h) = img.dimensions();
    if orig_w < 32 || orig_h < 32 {
        return None;
    }

    let scale = (WORK_MAX as f32 / orig_w.max(orig_h) as f32).min(1.0);
    let work_w = ((orig_w as f32) * scale).round().max(1.0) as u32;
    let work_h = ((orig_h as f32) * scale).round().max(1.0) as u32;
    let work = img.resize_exact(work_w, work_h, image::imageops::FilterType::Triangle);

    let gray = work.to_luma8();
    let blurred = gaussian_blur_f32(&gray, 1.6);
    let edges = canny(&blurred, 30.0, 90.0);
    let dilated = dilate(&edges, Norm::LInf, 1);
    let contours = find_contours::<i32>(&dilated);

    let frame_area = (work_w as f64) * (work_h as f64);
    // (area, minx, miny, maxx, maxy) en coordenadas de trabajo.
    let mut best: Option<(f64, i32, i32, i32, i32)> = None;

    for contour in &contours {
        if contour.points.len() < 8 {
            continue;
        }
        let (mut minx, mut miny) = (i32::MAX, i32::MAX);
        let (mut maxx, mut maxy) = (i32::MIN, i32::MIN);
        for p in &contour.points {
            minx = minx.min(p.x);
            miny = miny.min(p.y);
            maxx = maxx.max(p.x);
            maxy = maxy.max(p.y);
        }
        let bw = (maxx - minx) as f64;
        let bh = (maxy - miny) as f64;
        if bw < 1.0 || bh < 1.0 {
            continue;
        }
        let area = bw * bh;
        if area < frame_area * MIN_AREA_FRAC || area > frame_area * MAX_AREA_FRAC {
            continue;
        }
        let ratio = bw / bh;
        if !(0.45..=0.95).contains(&ratio) {
            continue;
        }
        if best.as_ref().map(|(a, ..)| area > *a).unwrap_or(true) {
            best = Some((area, minx, miny, maxx, maxy));
        }
    }

    let (_, minx, miny, maxx, maxy) = best?;

    // A coordenadas originales + margen, recortado al tamano de la imagen.
    let inv = 1.0 / scale;
    let bw = (maxx - minx) as f32 * inv;
    let bh = (maxy - miny) as f32 * inv;
    let mx = bw * MARGIN_FRAC;
    let my = bh * MARGIN_FRAC;
    let x0 = (((minx as f32) * inv) - mx).round().max(0.0) as u32;
    let y0 = (((miny as f32) * inv) - my).round().max(0.0) as u32;
    let x1 = ((((maxx as f32) * inv) + mx).round() as u32).min(orig_w);
    let y1 = ((((maxy as f32) * inv) + my).round() as u32).min(orig_h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some(img.crop_imm(x0, y0, x1 - x0, y1 - y0))
}
