//! Indice de busqueda por similitud coseno en Rust puro.
//!
//! Reemplaza a FAISS `IndexFlatIP`: los embeddings se almacenan
//! L2-normalizados, de modo que el producto escalar entre dos vectores es
//! directamente su similitud coseno. La busqueda es exhaustiva (fuerza
//! bruta): para catalogos de decenas de miles de cartas es de sobra rapida
//! en movil (unidades de milisegundos) y no requiere ninguna dependencia
//! nativa, lo que la hace trivial de cross-compilar a Android.

use serde::{Deserialize, Serialize};

/// Normaliza un vector a longitud unidad (L2) in situ.
///
/// Si el vector es nulo (norma 0) se deja intacto para evitar NaN.
pub fn l2_normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Metadatos minimos de cada fila del indice, alineados por posicion con la
/// matriz de embeddings. Coincide con `cards.json` del pipeline de ingesta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardRef {
    pub card_id: String,
    pub name: String,
    pub number: String,
    pub set_id: String,
    pub lang: String,
}

/// Resultado de una busqueda: referencia a la carta y su puntuacion coseno.
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    pub card: CardRef,
    pub score: f32,
}

/// Indice plano de embeddings + metadatos.
///
/// `vectors` es una matriz fila-mayor de `len * dim` floats ya
/// L2-normalizados; `cards[i]` describe la fila `i`.
#[derive(Debug, Clone)]
pub struct FlatIndex {
    dim: usize,
    vectors: Vec<f32>,
    cards: Vec<CardRef>,
}

impl FlatIndex {
    /// Crea un indice a partir de una matriz aplanada fila-mayor.
    ///
    /// Los vectores se normalizan al construir, de modo que el llamante no
    /// necesita preocuparse por ello. Falla si las dimensiones no cuadran.
    pub fn new(dim: usize, mut vectors: Vec<f32>, cards: Vec<CardRef>) -> anyhow::Result<Self> {
        anyhow::ensure!(dim > 0, "la dimension del embedding debe ser > 0");
        anyhow::ensure!(
            vectors.len() == dim * cards.len(),
            "longitud de la matriz ({}) != dim ({}) * num_cartas ({})",
            vectors.len(),
            dim,
            cards.len()
        );
        for row in vectors.chunks_mut(dim) {
            l2_normalize(row);
        }
        Ok(Self {
            dim,
            vectors,
            cards,
        })
    }

    /// Numero de cartas indexadas.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// `true` si el indice no contiene cartas.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Dimension de los embeddings.
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Devuelve las `k` cartas mas parecidas al `query`, de mayor a menor
    /// similitud coseno. El `query` se normaliza internamente, asi que puede
    /// pasarse sin normalizar. Falla si la dimension no coincide.
    pub fn search(&self, query: &[f32], k: usize) -> anyhow::Result<Vec<Match>> {
        anyhow::ensure!(
            query.len() == self.dim,
            "dimension del query ({}) != dim del indice ({})",
            query.len(),
            self.dim
        );
        if self.is_empty() || k == 0 {
            return Ok(Vec::new());
        }
        let mut q = query.to_vec();
        l2_normalize(&mut q);

        let mut scored: Vec<(usize, f32)> = self
            .vectors
            .chunks(self.dim)
            .enumerate()
            .map(|(i, row)| {
                let dot = row.iter().zip(&q).map(|(a, b)| a * b).sum::<f32>();
                (i, dot)
            })
            .collect();

        let k = k.min(scored.len());
        // Ordenacion parcial: solo nos interesan los k mejores.
        scored.select_nth_unstable_by(k - 1, |a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(k);
        scored.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored
            .into_iter()
            .map(|(i, score)| Match {
                card: self.cards[i].clone(),
                score,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str) -> CardRef {
        CardRef {
            card_id: id.to_string(),
            name: id.to_string(),
            number: "1".to_string(),
            set_id: "set".to_string(),
            lang: "en".to_string(),
        }
    }

    #[test]
    fn normalize_unit_length() {
        let mut v = vec![3.0_f32, 4.0];
        l2_normalize(&mut v);
        approx::assert_abs_diff_eq!(v[0], 0.6, epsilon = 1e-6);
        approx::assert_abs_diff_eq!(v[1], 0.8, epsilon = 1e-6);
    }

    #[test]
    fn normalize_zero_vector_is_safe() {
        let mut v = vec![0.0_f32, 0.0];
        l2_normalize(&mut v);
        assert_eq!(v, vec![0.0, 0.0]);
    }

    #[test]
    fn search_ranks_by_cosine() {
        // Tres vectores en 2D apuntando a direcciones distintas.
        let vectors = vec![
            1.0, 0.0, // este -> card a
            0.0, 1.0, // norte -> card b
            -1.0, 0.0, // oeste -> card c
        ];
        let idx = FlatIndex::new(2, vectors, vec![card("a"), card("b"), card("c")]).unwrap();

        // Query casi-este: el mejor debe ser "a", el peor "c".
        let res = idx.search(&[0.9, 0.1], 3).unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0].card.card_id, "a");
        assert_eq!(res[2].card.card_id, "c");
        // El mejor score debe ser practicamente 1.0 tras normalizar el query.
        assert!(res[0].score > 0.98);
        // Orden monotono decreciente.
        assert!(res[0].score >= res[1].score && res[1].score >= res[2].score);
    }

    #[test]
    fn search_respects_k() {
        let vectors = vec![1.0, 0.0, 0.0, 1.0, -1.0, 0.0];
        let idx = FlatIndex::new(2, vectors, vec![card("a"), card("b"), card("c")]).unwrap();
        let res = idx.search(&[1.0, 0.0], 2).unwrap();
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn dimension_mismatch_errors() {
        let idx = FlatIndex::new(2, vec![1.0, 0.0], vec![card("a")]).unwrap();
        assert!(idx.search(&[1.0, 0.0, 0.0], 1).is_err());
    }

    #[test]
    fn construction_validates_shape() {
        // 3 floats no son multiplo de dim=2 para 1 carta (necesitaria 2).
        assert!(FlatIndex::new(2, vec![1.0, 0.0, 3.0], vec![card("a")]).is_err());
    }
}
