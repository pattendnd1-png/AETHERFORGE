#![forbid(unsafe_code)]

use aether_core::NetworkPolicy;
use aether_model_api::{ModelError, ProviderRegistry, require_network};

use crate::embeddings::ModelEmbeddingProvider;

pub async fn embedding_provider_for_model(
    registry: &ProviderRegistry,
    model_id: &str,
    dimensions: usize,
    network_policy: NetworkPolicy,
) -> Result<ModelEmbeddingProvider, ModelError> {
    if dimensions == 0 {
        return Err(ModelError::Provider(
            "embedding dimensions must be non-zero".into(),
        ));
    }

    let descriptor = registry
        .model(model_id)
        .ok_or_else(|| ModelError::ModelNotFound(model_id.into()))?;
    if !descriptor.local {
        return Err(ModelError::Provider(
            "embedding model must be installed locally".into(),
        ));
    }

    let provider = registry.provider_for_model(model_id)?;
    let capabilities = provider.capabilities().await;
    if !capabilities.embeddings {
        return Err(ModelError::EmbeddingsUnsupported);
    }
    if capabilities.requires_paid_service {
        return Err(ModelError::Provider(
            "paid embedding providers are not eligible for local semantic retrieval".into(),
        ));
    }
    if capabilities.requires_network {
        require_network(network_policy)?;
    }

    ModelEmbeddingProvider::new(provider, model_id, dimensions)
        .map_err(|error| ModelError::Provider(error.to_string()))
}
