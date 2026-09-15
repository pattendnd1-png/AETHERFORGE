#![forbid(unsafe_code)]

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EmbeddingError {
    #[error("embedding provider error: {0}")]
    Provider(String),
    #[error("embedding permission is blocked")]
    BlockedByPermission,
    #[error("embedding vector is empty")]
    EmptyVector,
    #[error("embedding vector dimensions mismatch: left={left}, right={right}")]
    DimensionMismatch { left: usize, right: usize },
    #[error("embedding vector contains a non-finite value")]
    NonFinite,
    #[error("embedding provider returned {actual} vectors for {expected} inputs")]
    BatchSizeMismatch { expected: usize, actual: usize },
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError>;
    fn dimensions(&self) -> usize;
    fn model_id(&self) -> &str;
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32, EmbeddingError> {
    if a.is_empty() || b.is_empty() {
        return Err(EmbeddingError::EmptyVector);
    }
    if a.len() != b.len() {
        return Err(EmbeddingError::DimensionMismatch {
            left: a.len(),
            right: b.len(),
        });
    }
    if a.iter().chain(b).any(|value| !value.is_finite()) {
        return Err(EmbeddingError::NonFinite);
    }

    let mut dot = 0.0_f64;
    let mut left_norm = 0.0_f64;
    let mut right_norm = 0.0_f64;
    for (left, right) in a.iter().zip(b) {
        let left = f64::from(*left);
        let right = f64::from(*right);
        dot += left * right;
        left_norm += left * left;
        right_norm += right * right;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return Err(EmbeddingError::EmptyVector);
    }
    Ok((dot / (left_norm.sqrt() * right_norm.sqrt())).clamp(-1.0, 1.0) as f32)
}

pub(crate) fn semantic_score_milli(similarity: f32) -> i32 {
    ((similarity.clamp(-1.0, 1.0) + 1.0) * 300.0).round() as i32
}

pub(crate) fn validate_batch(
    vectors: &[Vec<f32>],
    expected_count: usize,
    dimensions: usize,
) -> Result<(), EmbeddingError> {
    if vectors.len() != expected_count {
        return Err(EmbeddingError::BatchSizeMismatch {
            expected: expected_count,
            actual: vectors.len(),
        });
    }
    for vector in vectors {
        if vector.len() != dimensions {
            return Err(EmbeddingError::DimensionMismatch {
                left: vector.len(),
                right: dimensions,
            });
        }
        if vector.is_empty() {
            return Err(EmbeddingError::EmptyVector);
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(EmbeddingError::NonFinite);
        }
    }
    Ok(())
}
