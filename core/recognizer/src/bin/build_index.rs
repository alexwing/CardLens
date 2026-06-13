//! Construye el indice MobileCLIP del catalogo para el reconocimiento on-device.
//!
//! Lee la lista de cartas de `data/index/cards.json` (la que ya genera la
//! ingesta, con {card_id, name, number, set_id, lang}), embebe cada imagen
//! `data/images/{card_id}.png` con MobileCLIP2-S0 y escribe el indice propio
//! del reconocedor: `data/index/mobileclip.bin` + `data/index/mobileclip_cards.json`.
//!
//! Ejecutar (desde core/recognizer):
//!   $env:ORT_DYLIB_PATH = "...\runtime\ort\onnxruntime.dll"
//!   cargo run --release --bin build_index --features "onnx desktop-dynamic" -- [limite]

use std::fs::File;
use std::path::Path;
use std::time::Instant;

use recognizer::{CardRef, Embedder, FlatIndex, PreprocessConfig};

fn main() -> anyhow::Result<()> {
    let limit: Option<usize> = std::env::args().nth(1).and_then(|s| s.parse().ok());

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf();
    let model_path = repo_root.join("models/mobileclip2_s0/vision_model.onnx");
    let images_dir = repo_root.join("data/images");
    let src_cards_path = repo_root.join("data/index/cards.json");
    let out_bin = repo_root.join("data/index/mobileclip.bin");
    let out_cards = repo_root.join("data/index/mobileclip_cards.json");

    anyhow::ensure!(model_path.exists(), "falta el modelo en {:?}", model_path);
    anyhow::ensure!(
        src_cards_path.exists(),
        "falta {:?}: ejecuta antes la ingesta (build_index de Python crea cards.json)",
        src_cards_path
    );

    let mut src: Vec<CardRef> = serde_json::from_reader(File::open(&src_cards_path)?)?;
    if let Some(n) = limit {
        src.truncate(n);
    }
    let total = src.len();
    println!("Cartas a indexar: {total}");

    let model_bytes = std::fs::read(&model_path)?;
    let mut embedder = Embedder::from_bytes(&model_bytes, PreprocessConfig::mobileclip2_s0())?;

    let mut vectors: Vec<f32> = Vec::new();
    let mut cards: Vec<CardRef> = Vec::new();
    let mut skipped = 0usize;
    let start = Instant::now();

    for (i, card) in src.into_iter().enumerate() {
        let image_path = images_dir.join(format!("{}.png", card.card_id));
        if !image_path.exists() {
            skipped += 1;
            continue;
        }
        match std::fs::read(&image_path).map_err(anyhow::Error::from) {
            Ok(bytes) => match embedder.embed_bytes(&bytes) {
                Ok(embedding) => {
                    vectors.extend_from_slice(&embedding);
                    cards.push(card);
                }
                Err(err) => {
                    eprintln!("  embedding fallido {}: {err}", card.card_id);
                    skipped += 1;
                }
            },
            Err(err) => {
                eprintln!("  lectura fallida {}: {err}", card.card_id);
                skipped += 1;
            }
        }
        if (i + 1) % 500 == 0 {
            let rate = (i + 1) as f64 / start.elapsed().as_secs_f64();
            println!("  {}/{} ({:.0} img/s)", i + 1, total, rate);
        }
    }

    anyhow::ensure!(!cards.is_empty(), "no se indexo ninguna carta");
    let dim = vectors.len() / cards.len();
    let index = FlatIndex::new(dim, vectors, cards)?;
    index.save(&out_bin, &out_cards)?;

    println!(
        "Indice MobileCLIP escrito: {} cartas, dim {}, {} omitidas, {:.0}s",
        index.len(),
        dim,
        skipped,
        start.elapsed().as_secs_f64()
    );
    println!("  {:?}", out_bin);
    println!("  {:?}", out_cards);
    Ok(())
}
