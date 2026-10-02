use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const EMBEDDING_DIM: usize = 384;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChunk {
    pub message_id: Uuid,
    pub role: String,
    pub author: String,
    pub content: String,
    pub turn_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredMemory {
    pub chunk: MemoryChunk,
    pub score: f32,
}

/// Generates a normalized high-dimensional semantic embedding vector from text.
/// Uses a combination of subword hashing, token frequency, and position weighting.
/// The resulting vector is L2-normalized so dot-product equals cosine similarity.
pub fn compute_embedding(text: &str) -> [f32; EMBEDDING_DIM] {
    let mut vec = [0.0f32; EMBEDDING_DIM];
    if text.is_empty() {
        return vec;
    }

    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '\'' && c != '-')
        .filter(|s| s.len() >= 2)
        .collect();

    if words.is_empty() {
        return vec;
    }

    // 1. Unigram and Bigram Feature Hashing into vector dimensions
    for (i, word) in words.iter().enumerate() {
        let hash1 = hash_str(word) as usize;
        let dim1 = hash1 % EMBEDDING_DIM;
        let sign1 = if ((hash1 >> 16) & 1) == 0 { 1.0 } else { -1.0 };
        vec[dim1] += sign1 * (1.0 + (word.len() as f32).ln());

        // Bigram features for semantic context
        if i + 1 < words.len() {
            let next_word = words[i + 1];
            let hash2 = hash_str(&format!("{word}_{next_word}")) as usize;
            let dim2 = hash2 % EMBEDDING_DIM;
            let sign2 = if ((hash2 >> 16) & 1) == 0 { 1.0 } else { -1.0 };
            vec[dim2] += sign2 * 1.5;
        }

        // Subword 3-gram character shingles
        let chars: Vec<char> = word.chars().collect();
        if chars.len() >= 3 {
            for window in chars.windows(3) {
                let shingle: String = window.iter().collect();
                let hash3 = hash_str(&shingle) as usize;
                let dim3 = hash3 % EMBEDDING_DIM;
                let sign3 = if ((hash3 >> 16) & 1) == 0 { 0.8 } else { -0.8 };
                vec[dim3] += sign3;
            }
        }
    }

    // 2. L2 Normalization
    let norm_sq: f32 = vec.iter().map(|v| v * v).sum();
    if norm_sq > 0.0 {
        let inv_norm = 1.0 / norm_sq.sqrt();
        for v in &mut vec {
            *v *= inv_norm;
        }
    }

    vec
}

fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Calculates cosine similarity between two normalized vectors (dot product).
pub fn cosine_similarity(a: &[f32; EMBEDDING_DIM], b: &[f32; EMBEDDING_DIM]) -> f32 {
    let mut dot = 0.0f32;
    for i in 0..EMBEDDING_DIM {
        dot += a[i] * b[i];
    }
    dot.clamp(-1.0, 1.0)
}

/// Retrieves the top-K most relevant past memories matching the query from candidate older messages.
pub fn retrieve_relevant_memories(
    query: &str,
    candidates: &[MemoryChunk],
    top_k: usize,
    threshold: f32,
) -> Vec<ScoredMemory> {
    if query.trim().is_empty() || candidates.is_empty() || top_k == 0 {
        return Vec::new();
    }

    let query_vec = compute_embedding(query);
    let mut scored: Vec<ScoredMemory> = candidates
        .iter()
        .map(|chunk| {
            let chunk_vec = compute_embedding(&chunk.content);
            let score = cosine_similarity(&query_vec, &chunk_vec);
            ScoredMemory {
                chunk: chunk.clone(),
                score,
            }
        })
        .filter(|sm| sm.score >= threshold)
        .collect();

    // Sort by relevance score descending
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(top_k);
    scored
}

/// Formats retrieved memory chunks for inclusion in the system prompt.
pub fn format_retrieved_memories(memories: &[ScoredMemory]) -> String {
    if memories.is_empty() {
        return String::new();
    }

    let mut out = String::from("[Relevant Past Memories & Lore from earlier conversation]\n");
    for m in memories {
        out.push_str(&format!(
            "- [Turn {}] {}: {}\n",
            m.chunk.turn_index + 1,
            m.chunk.author,
            m.chunk.content.trim()
        ));
    }
    out
}
