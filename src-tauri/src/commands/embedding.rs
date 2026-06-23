use crate::crypto::decrypt_saved_api_key;
use serde_json::Value;

const MAX_INPUT_CHARS: usize = 8000;
const MAX_BATCH_SIZE: usize = 20;
const EMBEDDING_TIMEOUT_SECS: u64 = 120;

/// Validate an embedding vector: non-empty, all finite, dimension matches expected.
pub fn validate_embedding(vec: &[f64], expected_dim: Option<usize>) -> Result<(), String> {
    if vec.is_empty() {
        return Err("Embedding vector is empty".to_string());
    }
    if let Some(dim) = expected_dim {
        if vec.len() != dim {
            return Err(crate::models::sanitize_error(format!(
                "Embedding dimension mismatch: expected {}, got {}", dim, vec.len()
            )));
        }
    }
    for (i, &v) in vec.iter().enumerate() {
        if !v.is_finite() {
            return Err(crate::models::sanitize_error(format!(
                "Embedding value at index {} is not finite: {}", i, v
            )));
        }
    }
    Ok(())
}

/// Embed text inputs using the configured embedding API.
/// Returns (vectors, model_used, total_tokens).
pub async fn embed_batch(
    base_url: &str, api_key: &str, model: &str, inputs: &[String],
) -> Result<(Vec<Vec<f64>>, String, u32), String> {
    if inputs.is_empty() {
        return Err("No inputs for embedding".to_string());
    }
    let batch = inputs.len().clamp(1, MAX_BATCH_SIZE);
    if batch != inputs.len() {
        return Err(format!("Batch size {} exceeds max {}", inputs.len(), MAX_BATCH_SIZE));
    }
    for input in inputs.iter().take(batch) {
        if input.len() > MAX_INPUT_CHARS {
            return Err(format!("Input exceeds {} chars", MAX_INPUT_CHARS));
        }
    }

    let url = format!("{}/v1/embeddings", base_url.trim_end_matches('/'));
    let body = serde_json::json!({ "model": model, "input": &inputs[..batch] });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(EMBEDDING_TIMEOUT_SECS))
        .build().map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let resp = client.post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json").json(&body).send().await
        .map_err(|e| crate::models::sanitize_error(format!("Embedding request failed: {}", e)))?;
    if !resp.status().is_success() {
        return Err(format!("Embedding HTTP {}", resp.status().as_u16()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| crate::models::sanitize_error(e.to_string()))?;
    let data = json["data"].as_array().ok_or("Unexpected embedding response")?;
    let emb_model = json["model"].as_str().unwrap_or(model).to_string();
    let tokens = json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32;
    let mut vectors = Vec::new();
    for item in data {
        let arr = item["embedding"].as_array().ok_or("Missing embedding array")?;
        let vec: Vec<f64> = arr.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect();
        validate_embedding(&vec, None)?;
        vectors.push(vec);
    }
    // Validate same-batch dimension consistency
    if let Some(first) = vectors.first() {
        let dim = first.len();
        for v in &vectors[1..] {
            if v.len() != dim {
                return Err(crate::models::sanitize_error(format!(
                    "Inconsistent embedding dimensions in batch: {} vs {}", dim, v.len()
                )));
            }
        }
    }
    Ok((vectors, emb_model, tokens))
}

/// Get a single embedding for a query (for hybrid search).
pub async fn embed_query(
    base_url: &str, api_key: &str, model: &str, query: &str,
) -> Result<Vec<f64>, String> {
    let input = if query.len() > MAX_INPUT_CHARS { query[..MAX_INPUT_CHARS].to_string() } else { query.to_string() };
    let (vecs, _, _) = embed_batch(base_url, api_key, model, &[input]).await?;
    vecs.into_iter().next().ok_or_else(|| "No embedding returned".to_string())
}

/// Cosine similarity between two vectors (must be same length).
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let (dot, na, nb) = a.iter().zip(b.iter())
        .fold((0.0, 0.0, 0.0), |(d, na, nb), (&x, &y)| {
            (d + x * y, na + x * x, nb + y * y)
        });
    let denom = (na * nb).sqrt();
    if denom == 0.0 { 0.0 } else { (dot / denom).clamp(-1.0, 1.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_empty_vector_fails() {
        assert!(validate_embedding(&[], None).is_err());
    }

    #[test]
    fn validate_nan_fails() {
        assert!(validate_embedding(&[1.0, f64::NAN, 3.0], None).is_err());
    }

    #[test]
    fn validate_inf_fails() {
        assert!(validate_embedding(&[1.0, f64::INFINITY], None).is_err());
    }

    #[test]
    fn validate_good_vector_passes() {
        assert!(validate_embedding(&[0.1, 0.2, 0.3], Some(3)).is_ok());
    }

    #[test]
    fn validate_dimension_mismatch_fails() {
        assert!(validate_embedding(&[0.1, 0.2, 0.3], Some(4)).is_err());
    }

    #[test]
    fn cosine_same_vectors_is_one() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 0.001);
    }

    #[test]
    fn cosine_different_lengths_is_zero() {
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn cosine_empty_is_zero() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }
}