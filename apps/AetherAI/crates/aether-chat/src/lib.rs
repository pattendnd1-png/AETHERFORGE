#![forbid(unsafe_code)]

pub mod context;

pub use context::{ChatContextError, ChatContextProvider, PreparedContext};

use aether_core::{Conversation, LocalCitation, MessageId, Role};
use aether_model_api::{GenerationChunk, GenerationRequest, ModelProvider};
use aether_retrieval::{RetrievalTrace, RetrievedContext};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEvent {
    UserCommitted(MessageId),
    RetrievalPrepared {
        citations: Vec<LocalCitation>,
        trace: RetrievalTrace,
    },
    AssistantDelta(String),
    AssistantCommitted(MessageId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatTurnResult {
    pub assistant_text: String,
    pub events: Vec<ChatEvent>,
    pub citations: Vec<LocalCitation>,
    pub retrieval: Option<RetrievedContext>,
}

#[derive(Debug, Error)]
pub enum ChatError {
    #[error(transparent)]
    InvalidSettings(#[from] aether_core::AetherCoreError),
    #[error(transparent)]
    Model(#[from] aether_model_api::ModelError),
    #[error(transparent)]
    Context(#[from] ChatContextError),
    #[error("no live model configured")]
    NoLiveModelConfigured,
    #[error("provider returned no assistant text")]
    EmptyResponse,
    #[error(
        "generation request exceeds live model context limit even with all retrieved context removed"
    )]
    ContextWindowExceeded,
}

pub struct ChatService {
    provider: Arc<dyn ModelProvider>,
    context_provider: Option<Arc<dyn ChatContextProvider>>,
}

impl ChatService {
    pub fn new(provider: Arc<dyn ModelProvider>) -> Self {
        Self {
            provider,
            context_provider: None,
        }
    }

    pub fn with_context_provider(
        provider: Arc<dyn ModelProvider>,
        context_provider: Arc<dyn ChatContextProvider>,
    ) -> Self {
        Self {
            provider,
            context_provider: Some(context_provider),
        }
    }

    pub async fn send_user_message(
        &self,
        conversation: &mut Conversation,
        content: impl Into<String>,
    ) -> Result<ChatTurnResult, ChatError> {
        self.send_user_message_with(conversation, content, |_| {})
            .await
    }

    pub async fn send_user_message_with<F>(
        &self,
        conversation: &mut Conversation,
        content: impl Into<String>,
        mut on_event: F,
    ) -> Result<ChatTurnResult, ChatError>
    where
        F: FnMut(&ChatEvent),
    {
        conversation.settings.generation.validate()?;
        let model = conversation
            .settings
            .model
            .clone()
            .ok_or(ChatError::NoLiveModelConfigured)?;
        let content = content.into();

        let user_id = conversation.push_message(Role::User, content.clone());
        let user_event = ChatEvent::UserCommitted(user_id);
        on_event(&user_event);
        let mut events = vec![user_event];

        let mut prepared = match &self.context_provider {
            Some(provider) => Some(provider.prepare(conversation, &content).await?),
            None => None,
        };

        if let Some(context) = &prepared {
            let retrieval_event = ChatEvent::RetrievalPrepared {
                citations: context.citations.clone(),
                trace: context.retrieval.trace.clone(),
            };
            on_event(&retrieval_event);
            events.push(retrieval_event);
        }

        let context_limit = self.live_context_limit(&model, conversation).await?;

        let request = loop {
            let request = build_generation_request(conversation, model.clone(), prepared.as_ref());
            let token_count = self.request_token_count(&request).await?;
            if token_count <= context_limit {
                break request;
            }

            let Some(context) = prepared.as_mut() else {
                return Err(ChatError::ContextWindowExceeded);
            };
            if context.retrieval.items.pop().is_none() {
                return Err(ChatError::ContextWindowExceeded);
            }
            context.retrieval.estimated_tokens = context
                .retrieval
                .items
                .iter()
                .map(|item| item.estimated_tokens)
                .sum();
            context.retrieval.trace.estimated_tokens = context.retrieval.estimated_tokens;
            context.rebuild_after_trim();
        };

        let stream = self.provider.generate_stream(request).await?;
        let mut assistant = String::new();

        for chunk in stream {
            match chunk? {
                GenerationChunk::Text(text) => {
                    assistant.push_str(&text);
                    let event = ChatEvent::AssistantDelta(text);
                    on_event(&event);
                    events.push(event);
                }
                GenerationChunk::Reasoning(_) => {}
            }
        }

        if assistant.is_empty() {
            return Err(ChatError::EmptyResponse);
        }

        let id = conversation.push_message(Role::Assistant, assistant.clone());
        let event = ChatEvent::AssistantCommitted(id);
        on_event(&event);
        events.push(event);

        let citations = prepared
            .as_ref()
            .map(|context| context.citations.clone())
            .unwrap_or_default();
        let retrieval = prepared.map(|context| context.retrieval);

        Ok(ChatTurnResult {
            assistant_text: assistant,
            events,
            citations,
            retrieval,
        })
    }

    async fn live_context_limit(
        &self,
        model: &str,
        conversation: &Conversation,
    ) -> Result<usize, ChatError> {
        let configured =
            usize::try_from(conversation.settings.generation.context_tokens).unwrap_or(usize::MAX);

        let descriptors = self.provider.models().await?;
        let live = descriptors
            .iter()
            .find(|descriptor| descriptor.id == model)
            .map(|descriptor| usize::try_from(descriptor.context_tokens).unwrap_or(usize::MAX));

        Ok(live.map_or(configured, |limit| limit.min(configured)))
    }

    async fn request_token_count(&self, request: &GenerationRequest) -> Result<usize, ChatError> {
        let serialized = request
            .messages
            .iter()
            .map(|message| format!("{:?}: {}", message.role, message.content))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(self.provider.tokenize(&serialized).await?.token_count)
    }
}

fn build_generation_request(
    conversation: &Conversation,
    model: String,
    prepared: Option<&PreparedContext>,
) -> GenerationRequest {
    let mut messages = conversation.messages.clone();

    if let Some(context) = prepared
        && !context.injected_messages.is_empty()
    {
        let insert_at = messages.len().saturating_sub(1);
        for (offset, message) in context.injected_messages.iter().cloned().enumerate() {
            messages.insert(insert_at + offset, message);
        }
    }

    GenerationRequest {
        model,
        messages,
        settings: conversation.settings.generation.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_model_api::{
        EmbeddingRequest, EmbeddingResponse, GenerationStream, ModelDescriptor, ModelError,
        ProviderCapabilities, ProviderHealth, Tokenization,
    };
    use async_trait::async_trait;
    use std::sync::mpsc;

    struct StreamingProvider {
        fail: bool,
        reasoning: bool,
    }

    #[async_trait]
    impl ModelProvider for StreamingProvider {
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
            Ok(vec![])
        }

        async fn generate_stream(
            &self,
            _: GenerationRequest,
        ) -> Result<GenerationStream, ModelError> {
            if self.fail {
                return Err(ModelError::Provider("forced".into()));
            }
            let (tx, rx) = mpsc::channel();
            if self.reasoning {
                tx.send(Ok(GenerationChunk::Reasoning("thinking".into())))
                    .unwrap();
            }
            tx.send(Ok(GenerationChunk::Text("Hel".into()))).unwrap();
            tx.send(Ok(GenerationChunk::Text("lo".into()))).unwrap();
            Ok(rx)
        }

        async fn embed(&self, _: EmbeddingRequest) -> Result<EmbeddingResponse, ModelError> {
            Err(ModelError::EmbeddingsUnsupported)
        }

        async fn tokenize(&self, _: &str) -> Result<Tokenization, ModelError> {
            Ok(Tokenization { token_count: 1 })
        }

        async fn health(&self) -> ProviderHealth {
            ProviderHealth::Ready
        }
    }

    #[tokio::test]
    async fn streamed_deltas_commit_one_assistant_message() {
        let provider = Arc::new(StreamingProvider {
            fail: false,
            reasoning: false,
        });
        let service = ChatService::new(provider);
        let mut conversation = Conversation::new("live");
        conversation.settings.model = Some("test/live".into());
        let mut seen = String::new();
        let result = service
            .send_user_message_with(&mut conversation, "ping", |event| {
                if let ChatEvent::AssistantDelta(delta) = event {
                    seen.push_str(delta);
                }
            })
            .await
            .unwrap();

        assert_eq!(seen, "Hello");
        assert_eq!(result.assistant_text, "Hello");
        assert!(result.citations.is_empty());
        assert!(result.retrieval.is_none());
        assert_eq!(conversation.messages.len(), 2);
        assert_eq!(conversation.messages[1].content, "Hello");
    }

    #[tokio::test]
    async fn reasoning_is_not_committed_as_final_assistant_text() {
        let service = ChatService::new(Arc::new(StreamingProvider {
            fail: false,
            reasoning: true,
        }));
        let mut conversation = Conversation::new("reasoning");
        conversation.settings.model = Some("test/live".into());
        let result = service
            .send_user_message(&mut conversation, "ping")
            .await
            .unwrap();
        assert_eq!(result.assistant_text, "Hello");
        assert_eq!(conversation.messages[1].content, "Hello");
    }

    #[tokio::test]
    async fn failure_preserves_user_without_assistant() {
        let service = ChatService::new(Arc::new(StreamingProvider {
            fail: true,
            reasoning: false,
        }));
        let mut conversation = Conversation::new("fail");
        conversation.settings.model = Some("test/live".into());
        assert!(
            service
                .send_user_message(&mut conversation, "ping")
                .await
                .is_err()
        );
        assert_eq!(conversation.messages.len(), 1);
        assert_eq!(conversation.messages[0].role, Role::User);
    }

    #[tokio::test]
    async fn no_model_does_not_fabricate_turn() {
        let service = ChatService::new(Arc::new(StreamingProvider {
            fail: false,
            reasoning: false,
        }));
        let mut conversation = Conversation::new("none");
        assert!(matches!(
            service.send_user_message(&mut conversation, "ping").await,
            Err(ChatError::NoLiveModelConfigured)
        ));
        assert!(conversation.messages.is_empty());
    }
}
