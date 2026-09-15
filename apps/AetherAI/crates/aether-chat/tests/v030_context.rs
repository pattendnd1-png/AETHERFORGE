use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
};

use aether_chat::{ChatContextError, ChatContextProvider, ChatEvent, ChatService, PreparedContext};
use aether_core::{Conversation, LocalCitation, Role, SourceRange};
use aether_model_api::{
    EmbeddingRequest, EmbeddingResponse, GenerationChunk, GenerationRequest, GenerationStream,
    ModelBackend, ModelDescriptor, ModelError, ModelFormat, ModelLoadState, ModelProvider,
    ProviderCapabilities, ProviderHealth, Tokenization,
};
use aether_retrieval::{
    ContextItem, ContextSource, RetrievalModes, RetrievalSignal, RetrievalTrace, RetrievedContext,
    SemanticAvailability,
};
use async_trait::async_trait;
use uuid::Uuid;

struct RecordingTokenProvider {
    limit: u32,
    last_request: Mutex<Option<GenerationRequest>>,
    last_prompt_tokens: Mutex<usize>,
}

impl RecordingTokenProvider {
    fn new(limit: u32) -> Self {
        Self {
            limit,
            last_request: Mutex::new(None),
            last_prompt_tokens: Mutex::new(0),
        }
    }

    fn last_prompt_tokens(&self) -> usize {
        *self.last_prompt_tokens.lock().unwrap()
    }

    fn last_request(&self) -> GenerationRequest {
        self.last_request.lock().unwrap().clone().unwrap()
    }
}

#[async_trait]
impl ModelProvider for RecordingTokenProvider {
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
        Ok(vec![ModelDescriptor {
            id: "test/live".into(),
            display_name: "Test Live".into(),
            backend: ModelBackend::AetherGguf,
            format: ModelFormat::Gguf,
            path: "/models/test.gguf".into(),
            context_tokens: self.limit,
            local: true,
            load_state: ModelLoadState::Ready,
        }])
    }

    async fn generate_stream(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationStream, ModelError> {
        let prompt = request
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        *self.last_prompt_tokens.lock().unwrap() = prompt.split_whitespace().count();
        *self.last_request.lock().unwrap() = Some(request);

        let (tx, rx) = mpsc::channel();
        tx.send(Ok(GenerationChunk::Text("answer".into()))).unwrap();
        Ok(rx)
    }

    async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse, ModelError> {
        Err(ModelError::EmbeddingsUnsupported)
    }

    async fn tokenize(&self, input: &str) -> Result<Tokenization, ModelError> {
        Ok(Tokenization {
            token_count: input.split_whitespace().count(),
        })
    }

    async fn health(&self) -> ProviderHealth {
        ProviderHealth::Ready
    }
}

struct StaticContextProvider {
    count: usize,
    words_per_item: usize,
}

impl StaticContextProvider {
    fn with_ranked_sources(count: usize, words_per_item: usize) -> Self {
        Self {
            count,
            words_per_item,
        }
    }
}

#[async_trait]
impl ChatContextProvider for StaticContextProvider {
    async fn prepare(
        &self,
        _conversation: &Conversation,
        _user_text: &str,
    ) -> Result<PreparedContext, ChatContextError> {
        let mut items = Vec::new();
        for index in 0..self.count {
            let content = std::iter::repeat_n("context", self.words_per_item)
                .collect::<Vec<_>>()
                .join(" ");
            items.push(ContextItem {
                id: Uuid::new_v4(),
                chunk_id: Some(Uuid::new_v4()),
                source: ContextSource::File {
                    canonical_path: PathBuf::from(format!("/project/src/{index}.rs")),
                    display_path: format!("src/{index}.rs"),
                    indexed_content_hash: format!("hash-{index}"),
                    range: SourceRange {
                        start_line: 1,
                        end_line: 2,
                    },
                },
                content,
                estimated_tokens: self.words_per_item,
                score_milli: 10_000 - index as i32,
                rationale: vec![RetrievalSignal::LexicalTerm],
            });
        }

        let estimated_tokens = items.iter().map(|item| item.estimated_tokens).sum();
        let retrieval = RetrievedContext {
            items,
            estimated_tokens,
            trace: RetrievalTrace {
                query: "project".into(),
                modes: RetrievalModes::lexical_and_symbols(),
                selected: Vec::new(),
                excluded: Vec::new(),
                semantic: SemanticAvailability::DisabledByPreset,
                semantic_error: None,
                estimated_tokens,
            },
        };
        Ok(PreparedContext::from_retrieval(retrieval, Vec::new()))
    }
}

struct AlwaysFailContextProvider;

#[async_trait]
impl ChatContextProvider for AlwaysFailContextProvider {
    async fn prepare(
        &self,
        _conversation: &Conversation,
        _user_text: &str,
    ) -> Result<PreparedContext, ChatContextError> {
        Err(ChatContextError::Repository("forced test failure".into()))
    }
}

#[tokio::test]
async fn retrieved_context_is_trimmed_before_live_generation() {
    let provider = Arc::new(RecordingTokenProvider::new(64));
    let context = Arc::new(StaticContextProvider::with_ranked_sources(12, 20));
    let service = ChatService::with_context_provider(provider.clone(), context);

    let mut chat = Conversation::new("Project question");
    chat.ensure_workspace("Project");
    chat.attach_workspace_root("/project", true).unwrap();
    chat.settings.model = Some("test/live".into());
    chat.settings.generation.context_tokens = 256;

    let result = service
        .send_user_message(&mut chat, "Where is ChatService defined?")
        .await
        .unwrap();

    assert!(provider.last_prompt_tokens() <= 64);
    assert_eq!(chat.messages.len(), 2);
    assert_eq!(chat.messages[0].role, Role::User);
    assert_eq!(chat.messages[1].role, Role::Assistant);
    assert!(!result.retrieval.as_ref().unwrap().items.is_empty());
    assert!(result.retrieval.as_ref().unwrap().items.len() < 12);

    let request = provider.last_request();
    assert!(request.messages.iter().any(|m| m.role == Role::Developer));
    assert!(!chat.messages.iter().any(|m| m.role == Role::Developer));
}

#[tokio::test]
async fn retrieval_event_and_result_keep_citations_separate_from_model_text() {
    struct CitedProvider;

    #[async_trait]
    impl ChatContextProvider for CitedProvider {
        async fn prepare(
            &self,
            _conversation: &Conversation,
            _user_text: &str,
        ) -> Result<PreparedContext, ChatContextError> {
            let citation = LocalCitation {
                canonical_path: PathBuf::from("/project/src/lib.rs"),
                display_path: "src/lib.rs".into(),
                range: SourceRange {
                    start_line: 91,
                    end_line: 142,
                },
                indexed_content_hash: "hash".into(),
                stale: false,
            };
            let item = ContextItem {
                id: Uuid::new_v4(),
                chunk_id: Some(Uuid::new_v4()),
                source: ContextSource::File {
                    canonical_path: citation.canonical_path.clone(),
                    display_path: citation.display_path.clone(),
                    indexed_content_hash: citation.indexed_content_hash.clone(),
                    range: citation.range,
                },
                content: "pub struct ChatService;".into(),
                estimated_tokens: 4,
                score_milli: 1000,
                rationale: vec![RetrievalSignal::ExactSymbol],
            };
            let retrieval = RetrievedContext {
                items: vec![item],
                estimated_tokens: 4,
                trace: RetrievalTrace {
                    query: "ChatService".into(),
                    modes: RetrievalModes::lexical_and_symbols(),
                    selected: Vec::new(),
                    excluded: Vec::new(),
                    semantic: SemanticAvailability::DisabledByPreset,
                    semantic_error: None,
                    estimated_tokens: 4,
                },
            };
            Ok(PreparedContext::from_retrieval(retrieval, vec![citation]))
        }
    }

    let provider = Arc::new(RecordingTokenProvider::new(256));
    let service = ChatService::with_context_provider(provider, Arc::new(CitedProvider));
    let mut chat = Conversation::new("citations");
    chat.settings.model = Some("test/live".into());

    let result = service
        .send_user_message(&mut chat, "where?")
        .await
        .unwrap();

    assert_eq!(result.assistant_text, "answer");
    assert_eq!(chat.messages[1].content, "answer");
    assert_eq!(result.citations.len(), 1);
    assert!(result.events.iter().any(|event| matches!(
        event,
        ChatEvent::RetrievalPrepared { citations, .. } if citations.len() == 1
    )));
}

#[tokio::test]
async fn context_failure_keeps_user_turn_and_creates_no_assistant_turn() {
    let mut chat = Conversation::new("Failure");
    chat.settings.model = Some("test/live".into());
    let id = chat.id;

    let provider = Arc::new(RecordingTokenProvider::new(2048));
    let service = ChatService::with_context_provider(provider, Arc::new(AlwaysFailContextProvider));

    assert!(
        service
            .send_user_message(&mut chat, "Use project files")
            .await
            .is_err()
    );
    assert_eq!(chat.id, id);
    assert_eq!(
        chat.messages
            .iter()
            .filter(|message| message.role == Role::Assistant)
            .count(),
        0
    );
    assert_eq!(
        chat.messages
            .iter()
            .filter(|message| message.role == Role::User)
            .count(),
        1
    );
}
