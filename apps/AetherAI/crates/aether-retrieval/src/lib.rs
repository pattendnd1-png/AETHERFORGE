#![forbid(unsafe_code)]

pub mod context;
mod lexical;
mod memory;
mod ranking;
pub mod search;
mod semantic;
mod symbols;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use aether_core::{ConversationId, IndexState, MemoryItem, ProjectId, SourceRange};
use aether_storage::models::{IndexChunkRecord, IndexSymbolRecord, IndexedFileRecord};
use aether_workspace::{GitContextSnapshot, PermissionedGitContextProvider};
use chrono::{DateTime, Utc};
use lexical::{lexical_points, query_terms};
use memory::{memory_display_path, memory_score};
use ranking::{RankedCandidate, apply_git_boosts, merge_candidate, sort_candidates};
use semantic::{semantic_score_milli, validate_batch};
use serde::{Deserialize, Serialize};
use symbols::{SymbolMatch, symbol_match};
use thiserror::Error;
use uuid::Uuid;

pub use context::{
    FreshnessChecker, RetrievalContextError, build_context_messages, citations_for_context,
    ensure_retrieved_context_fresh, file_paths,
};
pub use search::{
    SearchResult, SearchSource, UniversalSearchError, UniversalSearchRepository,
    UniversalSearchService,
};
pub use semantic::{EmbeddingError, EmbeddingProvider, cosine_similarity};

pub const RETRIEVAL_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalModes {
    pub path: bool,
    pub lexical: bool,
    pub symbols: bool,
    pub memory: bool,
    pub semantic: bool,
    pub git: bool,
}

impl RetrievalModes {
    pub const fn lexical_and_symbols() -> Self {
        Self {
            path: true,
            lexical: true,
            symbols: true,
            memory: false,
            semantic: false,
            git: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalRequest {
    pub conversation_id: ConversationId,
    pub project_id: Option<ProjectId>,
    pub query: String,
    pub max_context_tokens: usize,
    pub max_sources: usize,
    pub allowed_roots: Vec<PathBuf>,
    pub modes: RetrievalModes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RetrievalSignal {
    ExactPath,
    ExactFilename,
    ExactSymbol,
    PrefixSymbol,
    LexicalTerm,
    TermFrequency,
    ProjectLocal,
    VerificationArtifact,
    Recency,
    Semantic,
    Memory,
    ImportedChat,
    Git,
}

impl RetrievalSignal {
    pub(crate) const fn default_points(self) -> i32 {
        match self {
            Self::ExactPath => 1200,
            Self::ExactFilename => 900,
            Self::ExactSymbol => 1100,
            Self::PrefixSymbol => 700,
            Self::LexicalTerm => 100,
            Self::TermFrequency => 10,
            Self::ProjectLocal => 100,
            Self::VerificationArtifact => 80,
            Self::Recency => 50,
            Self::ImportedChat => 75,
            Self::Semantic | Self::Memory | Self::Git => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticAvailability {
    Available,
    UnavailableNoModel,
    UnavailableProvider,
    DisabledByPreset,
    BlockedByPermission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExclusionReason {
    Permission,
    ContextBudget,
    SourceLimit,
    Stale,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextSource {
    File {
        canonical_path: PathBuf,
        display_path: String,
        indexed_content_hash: String,
        range: SourceRange,
    },
    Symbol {
        canonical_path: PathBuf,
        display_path: String,
        indexed_content_hash: String,
        range: SourceRange,
        symbol_name: String,
    },
    Memory {
        memory_id: Uuid,
        display_path: String,
        source_links: Vec<String>,
    },
    ExternalChat {
        source_type: String,
        conversation_id: Uuid,
        message_id: Uuid,
        external_id: String,
        display_path: String,
    },
    VerificationCheckpoint {
        canonical_path: PathBuf,
        display_path: String,
        indexed_content_hash: String,
        range: SourceRange,
    },
}

impl ContextSource {
    pub fn display_path(&self) -> &str {
        match self {
            Self::File { display_path, .. }
            | Self::Symbol { display_path, .. }
            | Self::Memory { display_path, .. }
            | Self::ExternalChat { display_path, .. }
            | Self::VerificationCheckpoint { display_path, .. } => display_path,
        }
    }

    pub fn canonical_path(&self) -> &Path {
        match self {
            Self::File { canonical_path, .. }
            | Self::Symbol { canonical_path, .. }
            | Self::VerificationCheckpoint { canonical_path, .. } => canonical_path,
            Self::Memory { .. } | Self::ExternalChat { .. } => Path::new(""),
        }
    }

    pub fn indexed_content_hash(&self) -> Option<&str> {
        match self {
            Self::File {
                indexed_content_hash,
                ..
            }
            | Self::Symbol {
                indexed_content_hash,
                ..
            }
            | Self::VerificationCheckpoint {
                indexed_content_hash,
                ..
            } => Some(indexed_content_hash),
            Self::Memory { .. } | Self::ExternalChat { .. } => None,
        }
    }

    pub fn range(&self) -> SourceRange {
        match self {
            Self::File { range, .. }
            | Self::Symbol { range, .. }
            | Self::VerificationCheckpoint { range, .. } => *range,
            Self::Memory { .. } | Self::ExternalChat { .. } => SourceRange {
                start_line: 0,
                end_line: 0,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextItem {
    pub id: Uuid,
    pub chunk_id: Option<Uuid>,
    pub source: ContextSource,
    pub content: String,
    pub estimated_tokens: usize,
    pub score_milli: i32,
    pub rationale: Vec<RetrievalSignal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceCandidate {
    pub display_path: String,
    pub score_milli: i32,
    pub rationale: Vec<RetrievalSignal>,
    pub exclusion: Option<ExclusionReason>,
}

impl TraceCandidate {
    pub fn selected(path: impl Into<String>, score_milli: i32) -> Self {
        Self {
            display_path: path.into(),
            score_milli,
            rationale: Vec::new(),
            exclusion: None,
        }
    }

    pub fn excluded(path: impl Into<String>, score_milli: i32, reason: ExclusionReason) -> Self {
        Self {
            display_path: path.into(),
            score_milli,
            rationale: Vec::new(),
            exclusion: Some(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalTrace {
    pub query: String,
    pub modes: RetrievalModes,
    pub selected: Vec<TraceCandidate>,
    pub excluded: Vec<TraceCandidate>,
    pub semantic: SemanticAvailability,
    #[serde(default)]
    pub semantic_error: Option<String>,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievedContext {
    pub items: Vec<ContextItem>,
    pub estimated_tokens: usize,
    pub trace: RetrievalTrace,
}

#[derive(Debug, Error)]
pub enum RetrievalError {
    #[error("retrieval storage error: {0}")]
    Storage(String),
    #[error("git context error: {0}")]
    Git(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalChatCandidate {
    pub message_id: Uuid,
    pub conversation_id: Uuid,
    pub external_id: String,
    pub display_path: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

pub trait RetrievalRepository {
    fn indexed_files_in_roots(
        &self,
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexedFileRecord>, RetrievalError>;

    fn indexed_file_by_id(
        &self,
        file_id: Uuid,
    ) -> Result<Option<IndexedFileRecord>, RetrievalError>;

    fn chunks_for_file(&self, file_id: Uuid) -> Result<Vec<IndexChunkRecord>, RetrievalError>;

    fn lexical_candidates(
        &self,
        terms: &[String],
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexChunkRecord>, RetrievalError>;

    fn symbol_candidates(
        &self,
        query: &str,
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexSymbolRecord>, RetrievalError>;

    fn embedding_vector(
        &self,
        _chunk_id: Uuid,
        _model_id: &str,
        _content_hash: &str,
        _dimensions: usize,
    ) -> Result<Option<Vec<f32>>, RetrievalError> {
        Ok(None)
    }

    fn save_embedding_vector(
        &self,
        _chunk_id: Uuid,
        _model_id: &str,
        _content_hash: &str,
        _vector: &[f32],
    ) -> Result<(), RetrievalError> {
        Ok(())
    }

    fn invalidate_embedding_models_except(&self, _model_id: &str) -> Result<usize, RetrievalError> {
        Ok(0)
    }

    fn active_memory(
        &self,
        _project_id: Option<ProjectId>,
        _conversation_id: ConversationId,
    ) -> Result<Vec<MemoryItem>, RetrievalError> {
        Ok(Vec::new())
    }

    fn external_chat_messages(
        &self,
        _project_id: Option<ProjectId>,
        _conversation_id: ConversationId,
        _limit: usize,
    ) -> Result<Vec<ExternalChatCandidate>, RetrievalError> {
        Ok(Vec::new())
    }
}

impl RetrievalRepository for aether_storage::SqliteStore {
    fn indexed_files_in_roots(
        &self,
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexedFileRecord>, RetrievalError> {
        aether_storage::SqliteStore::indexed_files_in_roots(self, allowed_roots, limit)
            .map_err(storage_error)
    }

    fn indexed_file_by_id(
        &self,
        file_id: Uuid,
    ) -> Result<Option<IndexedFileRecord>, RetrievalError> {
        aether_storage::SqliteStore::indexed_file_by_id(self, file_id).map_err(storage_error)
    }

    fn chunks_for_file(&self, file_id: Uuid) -> Result<Vec<IndexChunkRecord>, RetrievalError> {
        aether_storage::SqliteStore::chunks_for_file(self, file_id).map_err(storage_error)
    }

    fn lexical_candidates(
        &self,
        terms: &[String],
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexChunkRecord>, RetrievalError> {
        aether_storage::SqliteStore::lexical_candidates(self, terms, allowed_roots, limit)
            .map_err(storage_error)
    }

    fn symbol_candidates(
        &self,
        query: &str,
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexSymbolRecord>, RetrievalError> {
        aether_storage::SqliteStore::symbol_candidates(self, query, allowed_roots, limit)
            .map_err(storage_error)
    }

    fn embedding_vector(
        &self,
        chunk_id: Uuid,
        model_id: &str,
        content_hash: &str,
        dimensions: usize,
    ) -> Result<Option<Vec<f32>>, RetrievalError> {
        aether_storage::SqliteStore::embedding_vector(
            self,
            chunk_id,
            model_id,
            content_hash,
            dimensions,
        )
        .map_err(storage_error)
    }

    fn save_embedding_vector(
        &self,
        chunk_id: Uuid,
        model_id: &str,
        content_hash: &str,
        vector: &[f32],
    ) -> Result<(), RetrievalError> {
        aether_storage::SqliteStore::save_embedding_vector(
            self,
            chunk_id,
            model_id,
            content_hash,
            vector,
        )
        .map_err(storage_error)
    }

    fn invalidate_embedding_models_except(&self, model_id: &str) -> Result<usize, RetrievalError> {
        aether_storage::SqliteStore::invalidate_embedding_models_except(self, model_id)
            .map_err(storage_error)
    }

    fn active_memory(
        &self,
        project_id: Option<ProjectId>,
        conversation_id: ConversationId,
    ) -> Result<Vec<MemoryItem>, RetrievalError> {
        aether_storage::SqliteStore::list_active_memory(self, project_id, conversation_id)
            .map_err(storage_error)
    }

    fn external_chat_messages(
        &self,
        project_id: Option<ProjectId>,
        conversation_id: ConversationId,
        limit: usize,
    ) -> Result<Vec<ExternalChatCandidate>, RetrievalError> {
        let records = aether_storage::SqliteStore::list_external_records(
            self,
            aether_storage::models::ExternalSourceType::ChatGptExport,
        )
        .map_err(storage_error)?;

        let project_external_id = project_id.and_then(|id| {
            records
                .iter()
                .find(|record| {
                    record.record_kind == aether_storage::models::ExternalRecordKind::Project
                        && record.aether_id == id
                })
                .map(|record| record.external_id.clone())
        });

        let mut scoped_conversations = std::collections::BTreeMap::<String, (Uuid, String)>::new();
        for record in records.iter().filter(|record| {
            record.record_kind == aether_storage::models::ExternalRecordKind::Conversation
        }) {
            let payload: serde_json::Value = match serde_json::from_str(&record.payload_json) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let linked_project = payload
                .get("project_external_id")
                .and_then(serde_json::Value::as_str);
            let in_scope = if let Some(project_external_id) = project_external_id.as_deref() {
                linked_project == Some(project_external_id)
            } else {
                record.aether_id == conversation_id
            };
            if !in_scope {
                continue;
            }
            let title = payload
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Imported ChatGPT conversation")
                .to_string();
            scoped_conversations.insert(record.external_id.clone(), (record.aether_id, title));
        }

        let mut candidates = Vec::new();
        for record in records.iter().filter(|record| {
            record.record_kind == aether_storage::models::ExternalRecordKind::Message
        }) {
            let payload: serde_json::Value = match serde_json::from_str(&record.payload_json) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let Some(external_conversation_id) = payload
                .get("conversation_external_id")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            let Some((conversation_id, title)) = scoped_conversations.get(external_conversation_id)
            else {
                continue;
            };
            let content = payload
                .get("content")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if content.is_empty() {
                continue;
            }
            let role = payload
                .get("role")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            candidates.push(ExternalChatCandidate {
                message_id: record.aether_id,
                conversation_id: *conversation_id,
                external_id: record.external_id.clone(),
                display_path: format!("ChatGPT Import · {title} · {role}"),
                content: content.to_string(),
                created_at: record.source_timestamp.unwrap_or(record.updated_at),
            });
        }

        candidates.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| left.external_id.cmp(&right.external_id))
        });
        candidates.truncate(limit);
        Ok(candidates)
    }
}

pub trait GitContextProvider: Send + Sync {
    fn snapshots(
        &self,
        allowed_roots: &[PathBuf],
    ) -> Result<Vec<GitContextSnapshot>, RetrievalError>;
}

impl GitContextProvider for PermissionedGitContextProvider {
    fn snapshots(
        &self,
        allowed_roots: &[PathBuf],
    ) -> Result<Vec<GitContextSnapshot>, RetrievalError> {
        PermissionedGitContextProvider::snapshots(self, allowed_roots)
            .map_err(|error| RetrievalError::Git(error.to_string()))
    }
}

pub struct RetrievalService<R> {
    repository: R,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    git_context_provider: Option<Arc<dyn GitContextProvider>>,
}

impl<R> RetrievalService<R>
where
    R: RetrievalRepository,
{
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            embedding_provider: None,
            git_context_provider: None,
        }
    }

    pub fn with_embedding_provider(mut self, provider: Option<Arc<dyn EmbeddingProvider>>) -> Self {
        self.embedding_provider = provider;
        self
    }

    pub fn with_git_context_provider(
        mut self,
        provider: Option<Arc<dyn GitContextProvider>>,
    ) -> Self {
        self.git_context_provider = provider;
        self
    }

    pub async fn retrieve(
        &self,
        request: RetrievalRequest,
    ) -> Result<RetrievedContext, RetrievalError> {
        let mut result = self.retrieve_sync(request.clone())?;
        if !request.modes.semantic {
            return Ok(result);
        }

        let Some(provider) = self.embedding_provider.as_ref() else {
            result.trace.semantic = SemanticAvailability::UnavailableNoModel;
            result.trace.semantic_error = None;
            return Ok(result);
        };

        let dimensions = provider.dimensions();
        if dimensions == 0 {
            result.trace.semantic = SemanticAvailability::UnavailableProvider;
            result.trace.semantic_error =
                Some("embedding provider reported zero dimensions".into());
            return Ok(result);
        }

        if let Err(error) = self
            .repository
            .invalidate_embedding_models_except(provider.model_id())
        {
            result.trace.semantic = SemanticAvailability::UnavailableProvider;
            result.trace.semantic_error = Some(error.to_string());
            return Ok(result);
        }

        let query_vectors = match provider
            .embed_batch(std::slice::from_ref(&request.query))
            .await
        {
            Ok(vectors) => vectors,
            Err(error) => {
                apply_semantic_error(&mut result, error);
                return Ok(result);
            }
        };
        if let Err(error) = validate_batch(&query_vectors, 1, dimensions) {
            apply_semantic_error(&mut result, error);
            return Ok(result);
        }
        let query_vector = &query_vectors[0];

        let mut vectors = vec![None; result.items.len()];
        let mut missing_indices = Vec::new();
        let mut missing_inputs = Vec::new();

        for (index, item) in result.items.iter().enumerate() {
            let Some(chunk_id) = item.chunk_id else {
                continue;
            };
            let Some(content_hash) = item.source.indexed_content_hash() else {
                continue;
            };
            match self.repository.embedding_vector(
                chunk_id,
                provider.model_id(),
                content_hash,
                dimensions,
            ) {
                Ok(Some(vector)) => vectors[index] = Some(vector),
                Ok(None) => {
                    missing_indices.push(index);
                    missing_inputs.push(item.content.clone());
                }
                Err(error) => {
                    result.trace.semantic = SemanticAvailability::UnavailableProvider;
                    result.trace.semantic_error = Some(error.to_string());
                    return Ok(result);
                }
            }
        }

        if !missing_inputs.is_empty() {
            let embedded = match provider.embed_batch(&missing_inputs).await {
                Ok(vectors) => vectors,
                Err(error) => {
                    apply_semantic_error(&mut result, error);
                    return Ok(result);
                }
            };
            if let Err(error) = validate_batch(&embedded, missing_inputs.len(), dimensions) {
                apply_semantic_error(&mut result, error);
                return Ok(result);
            }

            for (slot, vector) in missing_indices.into_iter().zip(embedded) {
                if let (Some(chunk_id), Some(content_hash)) = (
                    result.items[slot].chunk_id,
                    result.items[slot].source.indexed_content_hash(),
                ) {
                    if let Err(error) = self.repository.save_embedding_vector(
                        chunk_id,
                        provider.model_id(),
                        content_hash,
                        &vector,
                    ) {
                        result.trace.semantic = SemanticAvailability::UnavailableProvider;
                        result.trace.semantic_error = Some(error.to_string());
                        return Ok(result);
                    }
                }
                vectors[slot] = Some(vector);
            }
        }

        for (index, vector) in vectors.iter().enumerate().take(result.items.len()) {
            let Some(vector) = vector.as_ref() else {
                continue;
            };
            let similarity = match cosine_similarity(query_vector, vector) {
                Ok(value) => value,
                Err(error) => {
                    apply_semantic_error(&mut result, error);
                    return Ok(result);
                }
            };
            let item = &mut result.items[index];
            item.score_milli = item
                .score_milli
                .saturating_add(semantic_score_milli(similarity));
            if !item.rationale.contains(&RetrievalSignal::Semantic) {
                item.rationale.push(RetrievalSignal::Semantic);
                item.rationale.sort();
            }
        }

        result.items.sort_by(|left, right| {
            right
                .score_milli
                .cmp(&left.score_milli)
                .then_with(|| left.source.display_path().cmp(right.source.display_path()))
                .then_with(|| {
                    left.source
                        .range()
                        .start_line
                        .cmp(&right.source.range().start_line)
                })
                .then_with(|| left.id.cmp(&right.id))
        });
        result.trace.selected = result
            .items
            .iter()
            .map(|item| TraceCandidate {
                display_path: item.source.display_path().to_string(),
                score_milli: item.score_milli,
                rationale: item.rationale.clone(),
                exclusion: None,
            })
            .collect();
        result.trace.semantic = SemanticAvailability::Available;
        result.trace.semantic_error = None;
        Ok(result)
    }

    pub fn retrieve_sync(
        &self,
        request: RetrievalRequest,
    ) -> Result<RetrievedContext, RetrievalError> {
        let terms = query_terms(&request.query);
        let all_files = self
            .repository
            .indexed_files_in_roots(&request.allowed_roots, 4096)?;

        let newest = all_files.iter().map(|file| file.last_indexed_at).max();

        let mut candidates = BTreeMap::new();
        let mut excluded = Vec::new();

        if request.modes.path {
            for file in &all_files {
                if !path_allowed(&file.canonical_path, &request.allowed_roots) {
                    excluded.push(trace_for_file(
                        file,
                        &request.allowed_roots,
                        0,
                        Vec::new(),
                        ExclusionReason::Permission,
                    ));
                    continue;
                }
                if file.state != IndexState::Indexed {
                    excluded.push(trace_for_file(
                        file,
                        &request.allowed_roots,
                        0,
                        Vec::new(),
                        ExclusionReason::Stale,
                    ));
                    continue;
                }

                let (path_exact, filename_exact) =
                    path_signals(&request.query, file, &request.allowed_roots);
                if !path_exact && !filename_exact {
                    continue;
                }

                let chunks = self.repository.chunks_for_file(file.id)?;
                if chunks.is_empty() {
                    excluded.push(trace_for_file(
                        file,
                        &request.allowed_roots,
                        0,
                        Vec::new(),
                        ExclusionReason::Unsupported,
                    ));
                    continue;
                }

                for chunk in chunks {
                    let mut candidate = candidate_from_chunk(file, &chunk, &request.allowed_roots);
                    if path_exact {
                        candidate.add_signal(
                            RetrievalSignal::ExactPath,
                            RetrievalSignal::ExactPath.default_points(),
                        );
                    }
                    if filename_exact {
                        candidate.add_signal(
                            RetrievalSignal::ExactFilename,
                            RetrievalSignal::ExactFilename.default_points(),
                        );
                    }
                    add_common_signals(&mut candidate, file, &request, newest);
                    merge_candidate(&mut candidates, candidate);
                }
            }
        }

        if request.modes.lexical && !terms.is_empty() {
            for chunk in self.repository.lexical_candidates(
                &terms,
                &request.allowed_roots,
                request.max_sources.saturating_mul(16).max(64),
            )? {
                let Some(file) = self.repository.indexed_file_by_id(chunk.file_id)? else {
                    continue;
                };
                if !path_allowed(&file.canonical_path, &request.allowed_roots) {
                    excluded.push(trace_for_file(
                        &file,
                        &request.allowed_roots,
                        0,
                        Vec::new(),
                        ExclusionReason::Permission,
                    ));
                    continue;
                }
                if file.state != IndexState::Indexed {
                    excluded.push(trace_for_file(
                        &file,
                        &request.allowed_roots,
                        0,
                        Vec::new(),
                        ExclusionReason::Stale,
                    ));
                    continue;
                }

                let (points, occurrences) = lexical_points(&chunk.content, &terms);
                if points == 0 {
                    continue;
                }
                let mut candidate = candidate_from_chunk(&file, &chunk, &request.allowed_roots);
                candidate.add_signal(RetrievalSignal::LexicalTerm, points.min(500));
                if occurrences > 1 {
                    candidate.add_signal(
                        RetrievalSignal::TermFrequency,
                        (i32::try_from(occurrences).unwrap_or(i32::MAX) * 10).min(100),
                    );
                }
                add_common_signals(&mut candidate, &file, &request, newest);
                merge_candidate(&mut candidates, candidate);
            }
        }

        if request.modes.symbols {
            for token in terms.iter().filter(|token| token.len() >= 2) {
                for symbol in self.repository.symbol_candidates(
                    token,
                    &request.allowed_roots,
                    request.max_sources.saturating_mul(16).max(64),
                )? {
                    let Some(file) = self.repository.indexed_file_by_id(symbol.file_id)? else {
                        continue;
                    };
                    if !path_allowed(&file.canonical_path, &request.allowed_roots) {
                        excluded.push(trace_for_file(
                            &file,
                            &request.allowed_roots,
                            0,
                            Vec::new(),
                            ExclusionReason::Permission,
                        ));
                        continue;
                    }
                    if file.state != IndexState::Indexed {
                        excluded.push(trace_for_file(
                            &file,
                            &request.allowed_roots,
                            0,
                            Vec::new(),
                            ExclusionReason::Stale,
                        ));
                        continue;
                    }

                    let Some(kind) = symbol_match(&symbol, &terms) else {
                        continue;
                    };
                    let chunks = self.repository.chunks_for_file(file.id)?;
                    let Some(chunk) = chunks
                        .iter()
                        .find(|chunk| {
                            ranges_overlap(
                                chunk.start_line,
                                chunk.end_line,
                                symbol.start_line,
                                symbol.end_line,
                            )
                        })
                        .or_else(|| chunks.first())
                    else {
                        continue;
                    };

                    let mut candidate = candidate_from_chunk(&file, chunk, &request.allowed_roots);
                    candidate.source = ContextSource::Symbol {
                        canonical_path: file.canonical_path.clone(),
                        display_path: display_path(&file.canonical_path, &request.allowed_roots),
                        indexed_content_hash: file.content_hash.clone(),
                        range: SourceRange {
                            start_line: chunk.start_line,
                            end_line: chunk.end_line,
                        },
                        symbol_name: symbol.name.clone(),
                    };

                    match kind {
                        SymbolMatch::Exact => candidate.add_signal(
                            RetrievalSignal::ExactSymbol,
                            RetrievalSignal::ExactSymbol.default_points(),
                        ),
                        SymbolMatch::Prefix => candidate.add_signal(
                            RetrievalSignal::PrefixSymbol,
                            RetrievalSignal::PrefixSymbol.default_points(),
                        ),
                        SymbolMatch::Generic => {}
                    }

                    let (path_exact, filename_exact) =
                        path_signals(&request.query, &file, &request.allowed_roots);
                    if path_exact {
                        candidate.add_signal(
                            RetrievalSignal::ExactPath,
                            RetrievalSignal::ExactPath.default_points(),
                        );
                    }
                    if filename_exact {
                        candidate.add_signal(
                            RetrievalSignal::ExactFilename,
                            RetrievalSignal::ExactFilename.default_points(),
                        );
                    }
                    add_common_signals(&mut candidate, &file, &request, newest);
                    merge_candidate(&mut candidates, candidate);
                }
            }
        }

        if request.modes.lexical && !terms.is_empty() {
            for external in self.repository.external_chat_messages(
                request.project_id,
                request.conversation_id,
                request.max_sources.saturating_mul(16).max(64),
            )? {
                let (points, occurrences) = lexical_points(&external.content, &terms);
                if points == 0 {
                    continue;
                }

                let estimated = external.content.len().div_ceil(4).max(1);
                let mut candidate = RankedCandidate::new(
                    external.message_id,
                    external.message_id,
                    ContextSource::ExternalChat {
                        source_type: "ChatGPTExport".into(),
                        conversation_id: external.conversation_id,
                        message_id: external.message_id,
                        external_id: external.external_id,
                        display_path: external.display_path,
                    },
                    external.content,
                    estimated,
                );
                candidate.add_signal(
                    RetrievalSignal::ImportedChat,
                    RetrievalSignal::ImportedChat.default_points(),
                );
                candidate.add_signal(RetrievalSignal::LexicalTerm, points.min(500));
                if occurrences > 1 {
                    candidate.add_signal(
                        RetrievalSignal::TermFrequency,
                        (i32::try_from(occurrences).unwrap_or(i32::MAX) * 10).min(100),
                    );
                }
                if request.project_id.is_some() {
                    candidate.add_signal(
                        RetrievalSignal::ProjectLocal,
                        RetrievalSignal::ProjectLocal.default_points(),
                    );
                }
                merge_candidate(&mut candidates, candidate);
            }
        }

        // Safety-net root enforcement applies regardless of repository behavior.
        for file in self
            .repository
            .indexed_files_in_roots(&[], 4096)
            .unwrap_or_default()
        {
            if !path_allowed(&file.canonical_path, &request.allowed_roots)
                && query_mentions_file(&request.query, &file.canonical_path)
            {
                excluded.push(trace_for_file(
                    &file,
                    &request.allowed_roots,
                    0,
                    Vec::new(),
                    ExclusionReason::Permission,
                ));
            }
        }

        if request.modes.memory {
            let memories = self
                .repository
                .active_memory(request.project_id, request.conversation_id)?;
            let newest_memory = memories.iter().map(|memory| memory.updated_at).max();

            for memory in memories {
                let Some(points) = memory_score(&memory, &request.query, newest_memory) else {
                    continue;
                };

                let source = ContextSource::Memory {
                    memory_id: memory.id,
                    display_path: memory_display_path(memory.category),
                    source_links: memory.source_links.clone(),
                };
                let estimated_tokens = memory.summary.len().div_ceil(4).max(1);
                let mut candidate = RankedCandidate::new(
                    memory.id,
                    memory.id,
                    source,
                    memory.summary,
                    estimated_tokens,
                );
                candidate.add_signal(RetrievalSignal::Memory, points);
                merge_candidate(&mut candidates, candidate);
            }
        }

        if request.modes.git {
            if let Some(provider) = self.git_context_provider.as_ref() {
                let snapshots = provider.snapshots(&request.allowed_roots)?;
                apply_git_boosts(&mut candidates, &snapshots);
            }
        }

        let mut ranked = candidates.into_values().collect::<Vec<_>>();
        sort_candidates(&mut ranked);

        let mut items = Vec::new();
        let mut selected_trace = Vec::new();
        let mut estimated_tokens = 0_usize;

        for candidate in ranked {
            let rationale = candidate.rationale.iter().copied().collect::<Vec<_>>();
            let display = candidate.source.display_path().to_string();

            if items.len() >= request.max_sources {
                excluded.push(TraceCandidate {
                    display_path: display,
                    score_milli: candidate.score_milli,
                    rationale,
                    exclusion: Some(ExclusionReason::SourceLimit),
                });
                continue;
            }

            if candidate.estimated_tokens
                > request.max_context_tokens.saturating_sub(estimated_tokens)
            {
                excluded.push(TraceCandidate {
                    display_path: display,
                    score_milli: candidate.score_milli,
                    rationale,
                    exclusion: Some(ExclusionReason::ContextBudget),
                });
                continue;
            }

            let id = stable_item_id(
                candidate.source.canonical_path(),
                candidate.source.range(),
                candidate.chunk_id,
            );
            estimated_tokens = estimated_tokens.saturating_add(candidate.estimated_tokens);
            selected_trace.push(TraceCandidate {
                display_path: display,
                score_milli: candidate.score_milli,
                rationale: rationale.clone(),
                exclusion: None,
            });
            let chunk_id = if matches!(
                candidate.source,
                ContextSource::Memory { .. } | ContextSource::ExternalChat { .. }
            ) {
                None
            } else {
                Some(candidate.chunk_id)
            };
            items.push(ContextItem {
                id,
                chunk_id,
                source: candidate.source,
                content: candidate.content,
                estimated_tokens: candidate.estimated_tokens,
                score_milli: candidate.score_milli,
                rationale,
            });
        }

        excluded.sort_by(|left, right| {
            left.display_path
                .cmp(&right.display_path)
                .then_with(|| right.score_milli.cmp(&left.score_milli))
        });
        excluded.dedup_by(|left, right| {
            left.display_path == right.display_path && left.exclusion == right.exclusion
        });

        Ok(RetrievedContext {
            items,
            estimated_tokens,
            trace: RetrievalTrace {
                query: request.query,
                modes: request.modes,
                selected: selected_trace,
                excluded,
                semantic: SemanticAvailability::DisabledByPreset,
                semantic_error: None,
                estimated_tokens,
            },
        })
    }
}

fn apply_semantic_error(result: &mut RetrievedContext, error: EmbeddingError) {
    result.trace.semantic = if matches!(error, EmbeddingError::BlockedByPermission) {
        SemanticAvailability::BlockedByPermission
    } else {
        SemanticAvailability::UnavailableProvider
    };
    result.trace.semantic_error = Some(error.to_string());
}

fn candidate_from_chunk(
    file: &IndexedFileRecord,
    chunk: &IndexChunkRecord,
    allowed_roots: &[PathBuf],
) -> RankedCandidate {
    RankedCandidate::new(
        file.id,
        chunk.id,
        ContextSource::File {
            canonical_path: file.canonical_path.clone(),
            display_path: display_path(&file.canonical_path, allowed_roots),
            indexed_content_hash: file.content_hash.clone(),
            range: SourceRange {
                start_line: chunk.start_line,
                end_line: chunk.end_line,
            },
        },
        chunk.content.clone(),
        usize::try_from(chunk.estimated_tokens).unwrap_or(usize::MAX),
    )
}

fn add_common_signals(
    candidate: &mut RankedCandidate,
    file: &IndexedFileRecord,
    request: &RetrievalRequest,
    newest: Option<DateTime<Utc>>,
) {
    if request.project_id.is_some() {
        candidate.add_signal(
            RetrievalSignal::ProjectLocal,
            RetrievalSignal::ProjectLocal.default_points(),
        );
    }

    if file
        .canonical_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let upper = name.to_ascii_uppercase();
            upper.contains("VERIFY") && upper.ends_with(".TXT")
        })
    {
        candidate.add_signal(
            RetrievalSignal::VerificationArtifact,
            RetrievalSignal::VerificationArtifact.default_points(),
        );
    }

    if let Some(newest) = newest {
        let age_hours = newest
            .signed_duration_since(file.last_indexed_at)
            .num_hours()
            .max(0);
        let points = 50_i64.saturating_sub(age_hours.min(50)) as i32;
        if points > 0 {
            candidate.add_signal(RetrievalSignal::Recency, points);
        }
    }
}

fn path_signals(query: &str, file: &IndexedFileRecord, allowed_roots: &[PathBuf]) -> (bool, bool) {
    let lower_query = query.to_ascii_lowercase();
    let display = display_path(&file.canonical_path, allowed_roots).to_ascii_lowercase();
    let filename = file
        .canonical_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let tokens = query
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| {
                    matches!(ch, '"' | '\'' | ',' | ';' | '(' | ')' | '[' | ']')
                })
                .to_ascii_lowercase()
        })
        .collect::<Vec<_>>();

    let exact_path = !display.is_empty()
        && (lower_query == display
            || tokens.iter().any(|token| token == &display)
            || tokens
                .iter()
                .any(|token| token.contains('/') && display.ends_with(token)));
    let exact_filename = !filename.is_empty()
        && tokens
            .iter()
            .any(|token| token == &filename || token_basename(token) == filename);

    (exact_path, exact_filename)
}

fn token_basename(token: &str) -> &str {
    token.rsplit(['/', '\\']).next().unwrap_or(token)
}

fn query_mentions_file(query: &str, path: &Path) -> bool {
    let lower = query.to_ascii_lowercase();
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| lower.contains(&name.to_ascii_lowercase()))
        || lower.contains(&path.to_string_lossy().to_ascii_lowercase())
}

fn trace_for_file(
    file: &IndexedFileRecord,
    roots: &[PathBuf],
    score_milli: i32,
    rationale: Vec<RetrievalSignal>,
    reason: ExclusionReason,
) -> TraceCandidate {
    TraceCandidate {
        display_path: display_path(&file.canonical_path, roots),
        score_milli,
        rationale,
        exclusion: Some(reason),
    }
}

fn display_path(path: &Path, roots: &[PathBuf]) -> String {
    let best = roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count());

    best.and_then(|root| path.strip_prefix(root).ok())
        .filter(|relative| !relative.as_os_str().is_empty())
        .unwrap_or(path)
        .to_string_lossy()
        .trim_start_matches('/')
        .to_string()
}

fn path_allowed(path: &Path, roots: &[PathBuf]) -> bool {
    !roots.is_empty() && roots.iter().any(|root| path.starts_with(root))
}

fn ranges_overlap(left_start: u32, left_end: u32, right_start: u32, right_end: u32) -> bool {
    left_start <= right_end && right_start <= left_end
}

fn stable_item_id(path: &Path, range: SourceRange, chunk_id: Uuid) -> Uuid {
    let input = format!(
        "{}:{}:{}:{}",
        path.to_string_lossy(),
        range.start_line,
        range.end_line,
        chunk_id
    );
    let mut a = 0xcbf29ce484222325_u64;
    let mut b = 0x84222325cbf29ce4_u64;
    for byte in input.bytes() {
        a ^= u64::from(byte);
        a = a.wrapping_mul(0x100000001b3);
        b ^= u64::from(byte).wrapping_add(0x9e37);
        b = b.wrapping_mul(0x100000001b3);
    }
    Uuid::from_u128((u128::from(a) << 64) | u128::from(b))
}

fn storage_error(error: aether_storage::StorageError) -> RetrievalError {
    RetrievalError::Storage(error.to_string())
}
