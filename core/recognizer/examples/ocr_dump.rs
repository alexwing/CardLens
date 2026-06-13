//! Lee el texto de una imagen (detectando+recortando la carta primero).
//! cargo run --example ocr_dump --features ocr -- <entrada> <det.rten> <rec.rten>

use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).expect("uso: ocr_dump <entrada> <det.rten> <rec.rten>");
    let det = args.get(2).expect("falta modelo de deteccion");
    let rec = args.get(3).expect("falta modelo de reconocimiento");

    let img = image::open(input)?;
    let card = recognizer::detector::detect_and_crop(&img).unwrap_or(img);
    println!("carta {}x{}", card.width(), card.height());

    let reader = recognizer::ocr::OcrReader::from_files(Path::new(det), Path::new(rec))?;
    let text = reader.read_text(&card)?;
    println!("--- TEXTO OCR ---");
    println!("{text}");
    println!("-----------------");
    Ok(())
}
