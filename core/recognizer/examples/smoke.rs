//! Prueba de humo en escritorio del pipeline on-device completo.
//!
//! Carga MobileCLIP2-S0, embebe un punado de imagenes reales del catalogo
//! (data/images), construye el indice coseno en memoria y comprueba que una
//! carta se identifica a si misma por encima del resto. Valida modelo +
//! preprocesado + busqueda antes de tocar el cross-compile a Android.
//!
//! Ejecutar (desde core/recognizer):
//!   cargo run --release --example smoke --features "onnx download-binaries"

use std::path::{Path, PathBuf};
use std::time::Instant;

use recognizer::{CardRef, Embedder, FlatIndex, PreprocessConfig};

fn main() -> anyhow::Result<()> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .unwrap()
        .to_path_buf();
    let model_path = repo_root.join("models/mobileclip2_s0/vision_model.onnx");
    let images_dir = repo_root.join("data/images");

    anyhow::ensure!(model_path.exists(), "falta el modelo en {:?}", model_path);
    anyhow::ensure!(images_dir.exists(), "falta data/images en {:?}", images_dir);

    // Tomamos hasta 12 imagenes del catalogo para la prueba.
    let mut pngs: Vec<PathBuf> = std::fs::read_dir(&images_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "png").unwrap_or(false))
        .collect();
    pngs.sort();
    pngs.truncate(12);
    anyhow::ensure!(pngs.len() >= 3, "se necesitan >=3 imagenes; hay {}", pngs.len());

    println!("Cargando MobileCLIP2-S0 desde {:?}", model_path);
    let model_bytes = std::fs::read(&model_path)?;
    let t = Instant::now();
    let mut embedder = Embedder::from_bytes(&model_bytes, PreprocessConfig::mobileclip2_s0())?;
    println!("Modelo cargado en {} ms", t.elapsed().as_millis());

    // Embebemos cada carta y medimos latencia.
    let mut vectors: Vec<f32> = Vec::new();
    let mut cards: Vec<CardRef> = Vec::new();
    let mut dim = 0usize;
    for p in &pngs {
        let bytes = std::fs::read(p)?;
        let t = Instant::now();
        let emb = embedder.embed_bytes(&bytes)?;
        let ms = t.elapsed().as_millis();
        dim = emb.len();
        let id = p.file_stem().unwrap().to_string_lossy().to_string();
        println!("  embed {id}: {dim}d, {ms} ms");
        vectors.extend_from_slice(&emb);
        cards.push(CardRef {
            card_id: id.clone(),
            name: id,
            number: String::new(),
            set_id: String::new(),
            lang: String::new(),
        });
    }
    println!("Embedding dim = {dim} (esperado 512)");
    anyhow::ensure!(dim == 512, "dimension inesperada: {dim}");

    let index = FlatIndex::new(dim, vectors, cards)?;

    // Consultamos con la 3a carta (re-embebida): debe ser su propio top-1.
    let query_path = &pngs[2];
    let query_id = query_path.file_stem().unwrap().to_string_lossy().to_string();
    let query_emb = embedder.embed_bytes(&std::fs::read(query_path)?)?;

    let t = Instant::now();
    let results = index.search(&query_emb, 5)?;
    println!(
        "\nConsulta = {query_id}  (busqueda en {} us)",
        t.elapsed().as_micros()
    );
    for (rank, m) in results.iter().enumerate() {
        println!("  {}. {:<22} score={:.4}", rank + 1, m.card.card_id, m.score);
    }

    let top = &results[0];
    anyhow::ensure!(
        top.card.card_id == query_id,
        "FALLO: top-1 = {} pero se esperaba {}",
        top.card.card_id,
        query_id
    );
    anyhow::ensure!(top.score > 0.99, "FALLO: auto-similitud baja {:.4}", top.score);
    println!("\nOK: la carta se identifica a si misma (score {:.4}) por encima del resto.", top.score);
    Ok(())
}
