use aether_model_api::{
    GenerationRequest, GenerationStream, ModelDescriptor, ModelError, ModelProvider,
    ProviderCapabilities, ProviderHealth,
};
use aether_model_runtime::{LocalEndpoint, ManagedRuntime, OpenAiLocalClient};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Clone)]
pub struct NativeProvider {
    model: ModelDescriptor,
    client: OpenAiLocalClient,
    runtime: Option<Arc<ManagedRuntime>>,
}
impl NativeProvider {
    pub fn external(model: ModelDescriptor, port: u16) -> Self {
        Self {
            model,
            client: OpenAiLocalClient::new(LocalEndpoint::new(port)),
            runtime: None,
        }
    }
    pub fn managed_mistralrs(
        model: ModelDescriptor,
        command: impl Into<String>,
        port: u16,
    ) -> Self {
        let args = vec![
            "serve".into(),
            "--host".into(),
            "127.0.0.1".into(),
            "--port".into(),
            port.to_string(),
            "plain".into(),
            "-m".into(),
            model.path.clone(),
        ];
        let rt = ManagedRuntime::new(command, args, LocalEndpoint::new(port));
        Self {
            model,
            client: OpenAiLocalClient::new(LocalEndpoint::new(port)),
            runtime: Some(Arc::new(rt)),
        }
    }
    fn ready(&self) -> Result<(), ModelError> {
        if self.client.is_ready() {
            return Ok(());
        }
        if let Some(rt) = &self.runtime {
            rt.ensure_started()
                .map_err(|e| ModelError::Provider(e.to_string()))
        } else {
            Err(ModelError::Provider(
                "native Rust runtime is not reachable".into(),
            ))
        }
    }
}
#[async_trait]
impl ModelProvider for NativeProvider {
    async fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            text_generation: true,
            embeddings: false,
            tool_calling: false,
            vision: false,
            requires_network: false,
            requires_paid_service: false,
        }
    }
    async fn models(&self) -> Result<Vec<ModelDescriptor>, ModelError> {
        Ok(vec![self.model.clone()])
    }
    async fn generate_stream(&self, r: GenerationRequest) -> Result<GenerationStream, ModelError> {
        self.ready()?;
        self.client.generate_stream(r)
    }
    async fn health(&self) -> ProviderHealth {
        if self.client.is_ready() {
            ProviderHealth::Ready
        } else {
            ProviderHealth::Unavailable("Rust-native runtime not reachable".into())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use aether_model_api::{ModelBackend, ModelFormat};
    #[tokio::test]
    async fn native_is_free_local_provider() {
        let m = ModelDescriptor {
            id: "n".into(),
            display_name: "N".into(),
            backend: ModelBackend::AetherNative,
            format: ModelFormat::SafeTensors,
            path: "/tmp/n".into(),
            context_tokens: 4096,
            local: true,
            load_state: aether_model_api::ModelLoadState::Registered,
        };
        let p = NativeProvider::external(m, 9);
        let c = p.capabilities().await;
        assert!(!c.requires_network);
        assert!(!c.requires_paid_service);
    }
}
