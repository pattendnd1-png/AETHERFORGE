#![forbid(unsafe_code)]

use std::sync::Arc;

use aether_model_api::{EmbeddingRequest, ModelProvider};
use aether_retrieval::{EmbeddingError, EmbeddingProvider};
use async_trait::async_trait;

pub struct ModelEmbeddingProvider {
    provider: Arc<dyn ModelProvider>,
    model_id: String,
    dimensions: usize,
}

impl ModelEmbeddingProvider {
    pub fn new(
        provider: Arc<dyn ModelProvider>,
        model_id: impl Into<String>,
        dimensions: usize,
    ) -> Result<Self, EmbeddingError> {
        if dimensions == 0 {
            return Err(EmbeddingError::EmptyVector);
        }
        Ok(Self {
            provider,
            model_id: model_id.into(),
            dimensions,
        })
    }
}

#[async_trait]
impl EmbeddingProvider for ModelEmbeddingProvider {
    async fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let response = self
            .provider
            .embed(EmbeddingRequest {
                input: inputs.to_vec(),
            })
            .await
            .map_err(|error| EmbeddingError::Provider(error.to_string()))?;
        if response.vectors.len() != inputs.len() {
            return Err(EmbeddingError::BatchSizeMismatch {
                expected: inputs.len(),
                actual: response.vectors.len(),
            });
        }
        for vector in &response.vectors {
            if vector.len() != self.dimensions {
                return Err(EmbeddingError::DimensionMismatch {
                    left: vector.len(),
                    right: self.dimensions,
                });
            }
            if vector.is_empty() {
                return Err(EmbeddingError::EmptyVector);
            }
            if vector.iter().any(|value| !value.is_finite()) {
                return Err(EmbeddingError::NonFinite);
            }
        }
        Ok(response.vectors)
    }
    fn dimensions(&self) -> usize {
        self.dimensions
    }
    fn model_id(&self) -> &str {
        &self.model_id
    }
}
