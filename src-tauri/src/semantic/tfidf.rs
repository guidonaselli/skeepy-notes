/// TF-IDF vectorizer for local semantic search.
///
/// This is a lightweight, pure-Rust embedding model that represents each note
/// as a bag-of-words TF-IDF vector.  It is used as the default "semantic"
/// backend before the user downloads an ONNX model.
///
/// The vector space is built from the vocabulary of ALL notes in the database,
/// capped at `MAX_VOCAB` terms to keep memory and cosine-similarity computation
/// tractable (< 1ms per query on a 10k-note corpus).
use std::collections::HashMap;

const MAX_VOCAB: usize = 2048;
const MIN_DF: usize = 2; // min document frequency to include a term

// ─── Tokenization ─────────────────────────────────────────────────────────────

pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 3 && t.len() <= 30)
        .map(|t| t.to_lowercase())
        .filter(|t| !is_stopword(t))
        .collect()
}

fn is_stopword(word: &str) -> bool {
    const STOPS: &[&str] = &[
        "the", "and", "for", "are", "but", "not", "you", "all", "can", "had",
        "her", "was", "one", "our", "out", "day", "get", "has", "him", "his",
        "how", "its", "may", "new", "now", "old", "own", "see", "two", "way",
        "who", "did", "que", "con", "del", "las", "los", "una", "por", "para",
        "como", "mas", "pero", "sin", "sobre", "este", "esta", "esto",
    ];
    STOPS.contains(&word)
}

// ─── Corpus vocabulary builder ────────────────────────────────────────────────

/// Builds the vocabulary (term → column index) from a collection of documents.
/// Terms are sorted by document frequency descending, capped at `MAX_VOCAB`.
pub fn build_vocab(documents: &[Vec<String>]) -> HashMap<String, usize> {
    let mut df: HashMap<String, usize> = HashMap::new();

    for doc in documents {
        let unique: std::collections::HashSet<_> = doc.iter().collect();
        for term in unique {
            *df.entry(term.clone()).or_insert(0) += 1;
        }
    }

    // Keep terms with df >= MIN_DF, sorted by df descending, capped at MAX_VOCAB
    let mut terms: Vec<(String, usize)> = df
        .into_iter()
        .filter(|(_, count)| *count >= MIN_DF)
        .collect();

    terms.sort_by(|a, b| b.1.cmp(&a.1));
    terms.truncate(MAX_VOCAB);

    terms
        .into_iter()
        .enumerate()
        .map(|(i, (term, _))| (term, i))
        .collect()
}

// ─── TF-IDF vectorization ─────────────────────────────────────────────────────

/// Compute a TF-IDF vector for `tokens` given a prebuilt `vocab` and `idf` table.
///
/// Returns a unit-normalized vector (L2 norm = 1) of length `vocab.len()`.
pub fn vectorize(
    tokens: &[String],
    vocab: &HashMap<String, usize>,
    idf: &[f32],
) -> Vec<f32> {
    let dim = vocab.len();
    let mut tf: HashMap<usize, f32> = HashMap::new();
    let total = tokens.len() as f32;

    if total == 0.0 || dim == 0 {
        return vec![0.0; dim];
    }

    for token in tokens {
        if let Some(&idx) = vocab.get(token) {
            *tf.entry(idx).or_insert(0.0) += 1.0 / total;
        }
    }

    let mut vec: Vec<f32> = vec![0.0; dim];
    for (idx, tf_val) in tf {
        vec[idx] = tf_val * idf[idx];
    }

    // L2 normalize
    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 1e-9 {
        for v in &mut vec {
            *v /= norm;
        }
    }

    vec
}

/// Compute IDF values for each vocab term given `num_docs` and per-term df.
pub fn compute_idf(vocab: &HashMap<String, usize>, df: &HashMap<String, usize>, num_docs: usize) -> Vec<f32> {
    let n = num_docs as f32;
    let mut idf = vec![1.0f32; vocab.len()];
    for (term, &idx) in vocab {
        let df_val = *df.get(term).unwrap_or(&1) as f32;
        idf[idx] = (1.0 + n / (1.0 + df_val)).ln() + 1.0;
    }
    idf
}

// ─── Cosine similarity ────────────────────────────────────────────────────────

/// Cosine similarity between two unit-normalized vectors. Both must have equal length.
pub fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_splits_and_lowercases() {
        let tokens = tokenize("Hello World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(!tokens.contains(&"is".to_string())); // too short
    }

    #[test]
    fn vectorize_produces_unit_vector() {
        let docs = vec![
            tokenize("rust programming language systems"),
            tokenize("rust memory safety ownership"),
            tokenize("python scripting language easy"),
        ];
        let vocab = build_vocab(&docs);
        let df: HashMap<String, usize> = {
            let mut m = HashMap::new();
            for doc in &docs {
                let unique: std::collections::HashSet<_> = doc.iter().collect();
                for t in unique { *m.entry(t.clone()).or_insert(0) += 1; }
            }
            m
        };
        let idf = compute_idf(&vocab, &df, docs.len());
        let v = vectorize(&docs[0], &vocab, &idf);

        // L2 norm ≈ 1.0 (or 0 if empty, but shouldn't be)
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01 || norm < 1e-9);
    }

    #[test]
    fn similar_docs_have_higher_cosine() {
        let docs = vec![
            tokenize("rust programming memory safety"),
            tokenize("rust ownership borrowing memory"),
            tokenize("python javascript web development"),
        ];
        let vocab = build_vocab(&docs);
        let df: HashMap<String, usize> = {
            let mut m = HashMap::new();
            for doc in &docs {
                let unique: std::collections::HashSet<_> = doc.iter().collect();
                for t in unique { *m.entry(t.clone()).or_insert(0) += 1; }
            }
            m
        };
        let idf = compute_idf(&vocab, &df, docs.len());
        let v0 = vectorize(&docs[0], &vocab, &idf);
        let v1 = vectorize(&docs[1], &vocab, &idf);
        let v2 = vectorize(&docs[2], &vocab, &idf);

        let sim_01 = cosine_sim(&v0, &v1);
        let sim_02 = cosine_sim(&v0, &v2);

        // doc0 and doc1 share "rust" and "memory" — should be more similar than doc0 vs doc2
        assert!(sim_01 > sim_02, "sim_01={sim_01}, sim_02={sim_02}");
    }
}
