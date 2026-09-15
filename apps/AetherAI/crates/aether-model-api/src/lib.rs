use aether_core::{GenerationSettings, Message, NetworkPolicy};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, mpsc};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelBackend {
    AetherGguf,
    AetherNative,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFormat {
    Gguf,
    SafeTensors,
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ModelLoadState {
    #[default]
    Registered,
    Loading,
    Ready,
    Unavailable(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub text_generation: bool,
    pub embeddings: bool,
    pub tool_calling: bool,
    pub vision: bool,
    pub requires_network: bool,
    pub requires_paid_service: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub id: String,
    pub display_name: String,
    pub backend: ModelBackend,
    pub format: ModelFormat,
    pub path: String,
    pub context_tokens: u32,
    pub local: bool,
    #[serde(default)]
    pub load_state: ModelLoadState,
}
#[derive(Debug, Clone)]
pub struct GenerationRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub settings: GenerationSettings,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationChunk {
    Text(String),
    Reasoning(String),
}
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub input: Vec<String>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingResponse {
    pub vectors: Vec<Vec<f32>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokenization {
    pub token_count: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderHealth {
    Ready,
    Degraded(String),
    Unavailable(String),
}
pub type GenerationStream = mpsc::Receiver<Result<GenerationChunk, ModelError>>;
#[derive(Debug, Error, Clone)]
pub enum ModelError {
    #[error("model provider error: {0}")]
    Provider(String),
    #[error("embeddings are unsupported by this provider")]
    EmbeddingsUnsupported,
    #[error("network permission is required")]
    NetworkPermissionRequired,
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("production runtime rejects mock model ids")]
    MockModelRejected,
}
#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn capabilities(&self) -> ProviderCapabilities;
    async fn models(&self) -> Result<Vec<ModelDescriptor>, ModelError>;
    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationStream, ModelError>;
    async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse, ModelError> {
        Err(ModelError::EmbeddingsUnsupported)
    }
    async fn tokenize(&self, _input: &str) -> Result<Tokenization, ModelError> {
        Err(ModelError::Provider(
            "runtime tokenization is not exposed by this provider".into(),
        ))
    }
    async fn health(&self) -> ProviderHealth;
}
#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn ModelProvider>>,
    models: HashMap<String, ModelDescriptor>,
}
impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn bind_provider(&mut self, model_id: impl Into<String>, provider: Arc<dyn ModelProvider>) {
        self.providers.insert(model_id.into(), provider);
    }
    pub fn register_model(&mut self, model: ModelDescriptor) -> Result<(), ModelError> {
        if model.id == "aetherai/mock" {
            return Err(ModelError::MockModelRejected);
        }
        self.models.insert(model.id.clone(), model);
        Ok(())
    }
    pub fn model(&self, id: &str) -> Option<&ModelDescriptor> {
        self.models.get(id)
    }
    pub fn models(&self) -> Vec<ModelDescriptor> {
        let mut v: Vec<_> = self.models.values().cloned().collect();
        v.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        v
    }
    pub fn provider_for_model(&self, id: &str) -> Result<Arc<dyn ModelProvider>, ModelError> {
        self.models
            .get(id)
            .ok_or_else(|| ModelError::ModelNotFound(id.into()))?;
        self.providers
            .get(id)
            .cloned()
            .ok_or_else(|| ModelError::Provider(format!("provider for {id} not loaded")))
    }
}
pub fn require_network(policy: NetworkPolicy) -> Result<(), ModelError> {
    match policy {
        NetworkPolicy::On => Ok(()),
        NetworkPolicy::Off | NetworkPolicy::Ask => Err(ModelError::NetworkPermissionRequired),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn desc(id: &str) -> ModelDescriptor {
        ModelDescriptor {
            id: id.into(),
            display_name: "x".into(),
            backend: ModelBackend::AetherGguf,
            format: ModelFormat::Gguf,
            path: "x".into(),
            context_tokens: 1,
            local: true,
            load_state: ModelLoadState::Registered,
        }
    }
    #[test]
    fn registry_rejects_mock_id() {
        let mut r = ProviderRegistry::new();
        assert!(matches!(
            r.register_model(desc("aetherai/mock")),
            Err(ModelError::MockModelRejected)
        ));
    }
    #[test]
    fn network_ask_blocks_io() {
        assert!(require_network(NetworkPolicy::Ask).is_err());
        assert!(require_network(NetworkPolicy::On).is_ok());
    }
    #[test]
    fn old_descriptor_without_load_state_deserializes() {
        let json = r#"{"id":"local","display_name":"Local","backend":"AetherGguf","format":"Gguf","path":"/m","context_tokens":4096,"local":true}"#;
        let d: ModelDescriptor = serde_json::from_str(json).unwrap();
        assert_eq!(d.load_state, ModelLoadState::Registered);
    }
}
