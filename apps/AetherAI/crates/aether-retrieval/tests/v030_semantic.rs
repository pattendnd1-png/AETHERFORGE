use std::{path::PathBuf, sync::Arc};

use aether_core::{IndexState, SourceRange};
use aether_retrieval::{
    EmbeddingError, EmbeddingProvider, RetrievalModes, RetrievalRepository, RetrievalRequest,
    RetrievalService, RetrievalSignal, SemanticAvailability, cosine_similarity,
};
use aether_storage::models::{IndexChunkRecord, IndexSymbolRecord, IndexedFileRecord};
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
struct Row {
    file: IndexedFileRecord,
    chunk: IndexChunkRecord,
}

impl Row {
    fn new(path: &str, content: &str) -> Self {
        let file_id = stable_uuid(path);
        let chunk_id = stable_uuid(&format!("{path}:chunk"));
        Self {
            file: IndexedFileRecord {
                id: file_id,
                attachment_id: Uuid::nil(),
                canonical_path: PathBuf::from(path),
                size_bytes: content.len() as u64,
                modified_ns: 1,
                content_hash: format!("file-{file_id}"),
                index_version: 1,
                extractor_version: 1,
                state: IndexState::Indexed,
                last_indexed_at: Utc::now(),
                error: None,
            },
            chunk: IndexChunkRecord {
                id: chunk_id,
                file_id,
                ordinal: 0,
                start_line: 1,
                end_line: 1,
                content: content.into(),
                content_hash: format!("chunk-{chunk_id}"),
                estimated_tokens: 8,
            },
        }
    }
}

#[derive(Clone)]
struct Repo {
    rows: Vec<Row>,
}

impl RetrievalRepository for Repo {
    fn indexed_files_in_roots(
        &self,
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexedFileRecord>, aether_retrieval::RetrievalError> {
        Ok(self
            .rows
            .iter()
            .filter(|row| {
                allowed_roots.is_empty()
                    || allowed_roots
                        .iter()
                        .any(|root| row.file.canonical_path.starts_with(root))
            })
            .take(limit)
            .map(|row| row.file.clone())
            .collect())
    }

    fn indexed_file_by_id(
        &self,
        file_id: Uuid,
    ) -> Result<Option<IndexedFileRecord>, aether_retrieval::RetrievalError> {
        Ok(self
            .rows
            .iter()
            .find(|row| row.file.id == file_id)
            .map(|row| row.file.clone()))
    }

    fn chunks_for_file(
        &self,
        file_id: Uuid,
    ) -> Result<Vec<IndexChunkRecord>, aether_retrieval::RetrievalError> {
        Ok(self
            .rows
            .iter()
            .filter(|row| row.file.id == file_id)
            .map(|row| row.chunk.clone())
            .collect())
    }

    fn lexical_candidates(
        &self,
        terms: &[String],
        allowed_roots: &[PathBuf],
        limit: usize,
    ) -> Result<Vec<IndexChunkRecord>, aether_retrieval::RetrievalError> {
        let terms = terms
            .iter()
            .map(|term| term.to_ascii_lowercase())
            .collect::<Vec<_>>();
        Ok(self
            .rows
            .iter()
            .filter(|row| {
                allowed_roots
                    .iter()
                    .any(|root| row.file.canonical_path.starts_with(root))
            })
            .filter(|row| {
                let content = row.chunk.content.to_ascii_lowercase();
                terms.iter().any(|term| content.contains(term))
            })
            .take(limit)
            .map(|row| row.chunk.clone())
            .collect())
    }

    fn symbol_candidates(
        &self,
        _query: &str,
        _allowed_roots: &[PathBuf],
        _limit: usize,
    ) -> Result<Vec<IndexSymbolRecord>, aether_retrieval::RetrievalError> {
        Ok(Vec::new())
    }
}

struct DeterministicEmbedding;

#[async_trait]
impl EmbeddingProvider for DeterministicEmbedding {
    async fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        Ok(inputs
            .iter()
            .map(|input| {
                if input.contains("relevant") || input == "alpha" {
                    vec![1.0, 0.0]
                } else {
                    vec![0.0, 1.0]
                }
            })
            .collect())
    }
    fn dimensions(&self) -> usize {
        2
    }
    fn model_id(&self) -> &str {
        "deterministic-local"
    }
}

fn request(semantic: bool) -> RetrievalRequest {
    RetrievalRequest {
        conversation_id: Uuid::nil(),
        project_id: None,
        query: "alpha".into(),
        max_context_tokens: 1024,
        max_sources: 8,
        allowed_roots: vec![PathBuf::from("/project")],
        modes: RetrievalModes {
            semantic,
            ..RetrievalModes::lexical_and_symbols()
        },
    }
}

fn service() -> RetrievalService<Repo> {
    RetrievalService::new(Repo {
        rows: vec![
            Row::new("/project/a-other.txt", "alpha other"),
            Row::new("/project/z-relevant.txt", "alpha relevant"),
        ],
    })
}

#[tokio::test]
async fn semantic_requested_without_model_falls_back_to_lexical() {
    let result = service().retrieve(request(true)).await.unwrap();
    assert_eq!(
        result.trace.semantic,
        SemanticAvailability::UnavailableNoModel
    );
    assert!(result.trace.semantic_error.is_none());
    assert!(!result.items.is_empty());
}

#[tokio::test]
async fn semantic_signal_changes_ranking_only_when_enabled() {
    let plain = service().retrieve(request(false)).await.unwrap();
    assert_eq!(plain.items[0].source.display_path(), "a-other.txt");

    let semantic = service()
        .with_embedding_provider(Some(Arc::new(DeterministicEmbedding)))
        .retrieve(request(true))
        .await
        .unwrap();

    assert_eq!(semantic.trace.semantic, SemanticAvailability::Available);
    assert_eq!(semantic.items[0].source.display_path(), "z-relevant.txt");
    assert!(
        semantic.items[0]
            .rationale
            .contains(&RetrievalSignal::Semantic)
    );
}

#[test]
fn cosine_similarity_rejects_invalid_vectors() {
    assert!(cosine_similarity(&[], &[]).is_err());
    assert!(cosine_similarity(&[1.0], &[1.0, 2.0]).is_err());
    assert!(cosine_similarity(&[f32::NAN], &[1.0]).is_err());
    assert_eq!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]).unwrap(), 1.0);
}

#[test]
fn task9_context_items_keep_exact_source_ranges() {
    let result = service().retrieve_sync(request(false)).unwrap();
    assert_eq!(
        result.items[0].source.range(),
        SourceRange {
            start_line: 1,
            end_line: 1
        }
    );
    assert!(result.items[0].chunk_id.is_some());
}

fn stable_uuid(input: &str) -> Uuid {
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
