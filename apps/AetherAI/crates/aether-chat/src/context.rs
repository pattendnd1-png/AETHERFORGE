#![forbid(unsafe_code)]

use aether_core::{Conversation, LocalCitation, Message};
use aether_retrieval::{RetrievedContext, build_context_messages, citations_for_context};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ChatContextError {
    #[error("context repository error: {0}")]
    Repository(String),
    #[error("context freshness error: {0}")]
    Freshness(String),
}

#[derive(Debug, Clone)]
pub struct PreparedContext {
    pub injected_messages: Vec<Message>,
    pub citations: Vec<LocalCitation>,
    pub retrieval: RetrievedContext,
}

impl PreparedContext {
    pub fn from_retrieval(retrieval: RetrievedContext, citations: Vec<LocalCitation>) -> Self {
        let citations = if citations.is_empty() {
            citations_for_context(&retrieval)
        } else {
            citations
        };
        let injected_messages = build_context_messages(&retrieval);
        Self {
            injected_messages,
            citations,
            retrieval,
        }
    }

    pub fn rebuild_after_trim(&mut self) {
        self.injected_messages = build_context_messages(&self.retrieval);
        let active = citations_for_context(&self.retrieval);
        self.citations.retain(|citation| {
            citation.stale
                || active.iter().any(|candidate| {
                    candidate.canonical_path == citation.canonical_path
                        && candidate.range == citation.range
                        && candidate.indexed_content_hash == citation.indexed_content_hash
                })
        });
    }
}

#[async_trait]
pub trait ChatContextProvider: Send + Sync {
    async fn prepare(
        &self,
        conversation: &Conversation,
        user_text: &str,
    ) -> Result<PreparedContext, ChatContextError>;
}
