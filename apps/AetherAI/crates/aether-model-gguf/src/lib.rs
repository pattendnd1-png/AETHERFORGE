use aether_model_api::{
    EmbeddingRequest, EmbeddingResponse, GenerationRequest, GenerationStream, ModelDescriptor,
    ModelError, ModelProvider, ProviderCapabilities, ProviderHealth, Tokenization,
};
use aether_model_runtime::{LocalEndpoint, ManagedRuntime, OpenAiLocalClient};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct GgufProvider {
    model: ModelDescriptor,
    client: OpenAiLocalClient,
    runtime: Option<Arc<ManagedRuntime>>,
}
impl GgufProvider {
    pub fn external(model: ModelDescriptor, port: u16) -> Self {
        Self {
            model,
            client: OpenAiLocalClient::new(LocalEndpoint::new(port)),
            runtime: None,
        }
    }
    pub fn managed(
        model: ModelDescriptor,
        runtime_path: PathBuf,
        port: u16,
        threads: Option<u16>,
        gpu_layers: Option<i16>,
    ) -> Self {
        let args = managed_args(&model, port, threads, gpu_layers);
        let rt = ManagedRuntime::new(
            runtime_path.to_string_lossy(),
            args,
            LocalEndpoint::new(port),
        );
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
            Err(ModelError::Provider("GGUF runtime is not reachable".into()))
        }
    }
}
fn managed_args(
    model: &ModelDescriptor,
    port: u16,
    threads: Option<u16>,
    gpu_layers: Option<i16>,
) -> Vec<String> {
    let mut args = vec![
        "--model".into(),
        model.path.clone(),
        "--host".into(),
        "127.0.0.1".into(),
        "--port".into(),
        port.to_string(),
        "--ctx-size".into(),
        model.context_tokens.to_string(),
    ];
    if let Some(t) = threads {
        args.extend(["--threads".into(), t.to_string()]);
    }
    if let Some(g) = gpu_layers {
        args.extend(["--n-gpu-layers".into(), g.to_string()]);
    }
    args
}
#[async_trait]
impl ModelProvider for GgufProvider {
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
    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationStream, ModelError> {
        self.ready()?;
        self.client.generate_stream(request)
    }
    async fn embed(&self, _: EmbeddingRequest) -> Result<EmbeddingResponse, ModelError> {
        Err(ModelError::EmbeddingsUnsupported)
    }
    async fn tokenize(&self, input: &str) -> Result<Tokenization, ModelError> {
        self.ready()?;
        self.client.tokenize(input)
    }
    async fn health(&self) -> ProviderHealth {
        if self.client.is_ready() {
            ProviderHealth::Ready
        } else {
            ProviderHealth::Unavailable("GGUF runtime not reachable".into())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use aether_model_api::{ModelBackend, ModelFormat, ModelLoadState};
    fn model() -> ModelDescriptor {
        ModelDescriptor {
            id: "q".into(),
            display_name: "Q".into(),
            backend: ModelBackend::AetherGguf,
            format: ModelFormat::Gguf,
            path: "/tmp/q.gguf".into(),
            context_tokens: 4096,
            local: true,
            load_state: ModelLoadState::Registered,
        }
    }
    #[tokio::test]
    async fn gguf_is_free_local_provider() {
        let p = GgufProvider::external(model(), 9);
        let c = p.capabilities().await;
        assert!(!c.requires_network);
        assert!(!c.requires_paid_service);
    }
    #[test]
    fn managed_command_contains_real_model_controls() {
        let a = managed_args(&model(), 8080, Some(8), Some(-1));
        assert!(a.windows(2).any(|w| w == ["--model", "/tmp/q.gguf"]));
        assert!(a.windows(2).any(|w| w == ["--ctx-size", "4096"]));
        assert!(a.windows(2).any(|w| w == ["--threads", "8"]));
        assert!(a.windows(2).any(|w| w == ["--n-gpu-layers", "-1"]));
    }
    #[tokio::test]
    async fn tokenize_does_not_fake_whitespace_counts() {
        let p = GgufProvider::external(model(), 9);
        assert!(p.tokenize("one two three").await.is_err());
    }
}
