use std::sync::{Arc, mpsc};

use aether_core::NetworkPolicy;
use aether_model_api::{
    EmbeddingRequest, EmbeddingResponse, GenerationRequest, GenerationStream, ModelBackend,
    ModelDescriptor, ModelError, ModelFormat, ModelLoadState, ModelProvider, ProviderCapabilities,
    ProviderHealth, ProviderRegistry,
};
use aether_model_runtime::{
    embeddings::ModelEmbeddingProvider, registry::embedding_provider_for_model,
};
use aether_retrieval::EmbeddingProvider;
use async_trait::async_trait;

struct FakeProvider {
    embeddings: bool,
    requires_network: bool,
}

#[async_trait]
impl ModelProvider for FakeProvider {
    async fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            text_generation: false,
            embeddings: self.embeddings,
            tool_calling: false,
            vision: false,
            requires_network: self.requires_network,
            requires_paid_service: false,
        }
    }
    async fn models(&self) -> Result<Vec<ModelDescriptor>, ModelError> {
        Ok(Vec::new())
    }
    async fn generate_stream(
        &self,
        _request: GenerationRequest,
    ) -> Result<GenerationStream, ModelError> {
        let (_tx, rx) = mpsc::channel();
        Ok(rx)
    }
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, ModelError> {
        Ok(EmbeddingResponse {
            vectors: request.input.iter().map(|_| vec![1.0, 0.0]).collect(),
        })
    }
    async fn health(&self) -> ProviderHealth {
        ProviderHealth::Ready
    }
}

fn descriptor(id: &str) -> ModelDescriptor {
    ModelDescriptor {
        id: id.into(),
        display_name: id.into(),
        backend: ModelBackend::AetherGguf,
        format: ModelFormat::Gguf,
        path: "/models/embed.gguf".into(),
        context_tokens: 2048,
        local: true,
        load_state: ModelLoadState::Ready,
    }
}

#[tokio::test]
async fn model_embedding_provider_batches_through_model_provider() {
    let provider: Arc<dyn ModelProvider> = Arc::new(FakeProvider {
        embeddings: true,
        requires_network: false,
    });
    let adapter = ModelEmbeddingProvider::new(provider, "local-embed", 2).unwrap();
    let vectors = adapter
        .embed_batch(&["one".into(), "two".into()])
        .await
        .unwrap();
    assert_eq!(vectors, vec![vec![1.0, 0.0], vec![1.0, 0.0]]);
    assert_eq!(adapter.dimensions(), 2);
    assert_eq!(adapter.model_id(), "local-embed");
}

#[tokio::test]
async fn registry_adapter_rejects_unsupported_or_disallowed_network_provider() {
    let mut registry = ProviderRegistry::new();
    registry.register_model(descriptor("no-embed")).unwrap();
    registry.bind_provider(
        "no-embed",
        Arc::new(FakeProvider {
            embeddings: false,
            requires_network: false,
        }),
    );
    assert!(
        embedding_provider_for_model(&registry, "no-embed", 2, NetworkPolicy::Off)
            .await
            .is_err()
    );

    let mut registry = ProviderRegistry::new();
    registry
        .register_model(descriptor("network-embed"))
        .unwrap();
    registry.bind_provider(
        "network-embed",
        Arc::new(FakeProvider {
            embeddings: true,
            requires_network: true,
        }),
    );
    assert!(
        embedding_provider_for_model(&registry, "network-embed", 2, NetworkPolicy::Off)
            .await
            .is_err()
    );
}
