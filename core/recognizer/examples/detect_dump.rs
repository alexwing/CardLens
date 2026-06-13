//! Volcado de depuracion del detector: guarda el mapa de bordes y el recorte.
//! cargo run --example detect_dump -- <entrada> <salida_base>

use imageproc::distance_transform::Norm;
use imageproc::edges::canny;
use imageproc::filter::gaussian_blur_f32;
use imageproc::morphology::dilate;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).expect("uso: detect_dump <entrada> <base_salida>");
    let base = args.get(2).expect("uso: detect_dump <entrada> <base_salida>");
    let img = image::open(input)?;
    println!("entrada {}x{}", img.width(), img.height());

    // Replica el preproceso del detector para inspeccionar los bordes.
    let scale = (700.0_f32 / img.width().max(img.height()) as f32).min(1.0);
    let ww = ((img.width() as f32) * scale).round() as u32;
    let wh = ((img.height() as f32) * scale).round() as u32;
    let work = img.resize_exact(ww, wh, image::imageops::FilterType::Triangle);
    let gray = work.to_luma8();
    let blurred = gaussian_blur_f32(&gray, 1.6);
    let edges = canny(&blurred, 30.0, 90.0);
    let dilated = dilate(&edges, Norm::LInf, 1);
    dilated.save(format!("{base}_edges.png"))?;
    println!("bordes guardados en {base}_edges.png ({ww}x{wh})");

    match recognizer::detector::detect_and_crop(&img) {
        Some(card) => {
            card.save(format!("{base}_crop.png"))?;
            println!("recorte en {base}_crop.png");
        }
        None => println!("SIN DETECCION"),
    }
    Ok(())
}
