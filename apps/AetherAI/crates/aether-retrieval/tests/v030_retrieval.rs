use std::path::{Path, PathBuf};

use aether_core::{IndexState, SourceRange};
use aether_retrieval::{
    ContextSource, ExclusionReason, RetrievalModes, RetrievalRepository, RetrievalRequest,
    RetrievalService, RetrievalSignal, SemanticAvailability,
};
use aether_storage::models::{IndexChunkRecord, IndexSymbolRecord, IndexedFileRecord};
use chrono::Utc;
use uuid::Uuid;

#[derive(Clone)]
struct TestIndexedRow {
    file: IndexedFileRecord,
    chunks: Vec<IndexChunkRecord>,
    symbols: Vec<IndexSymbolRecord>,
}

impl TestIndexedRow {
    fn rust(
        path: &str,
        start_line: u32,
        end_line: u32,
        content: &str,
        symbol: Option<&str>,
    ) -> Self {
        Self::new(path, start_line, end_line, content, symbol)
    }

    fn text(path: &str, start_line: u32, end_line: u32, content: &str) -> Self {
        Self::new(path, start_line, end_line, content, None)
    }

    fn new(
        path: &str,
        start_line: u32,
        end_line: u32,
        content: &str,
        symbol: Option<&str>,
    ) -> Self {
        let file_id = stable_uuid(path);
        let chunk_id = stable_uuid(&format!("{path}:{start_line}:{end_line}"));
        let file = IndexedFileRecord {
            id: file_id,
            attachment_id: Uuid::nil(),
            canonical_path: PathBuf::from(path),
            size_bytes: content.len() as u64,
            modified_ns: 123,
            content_hash: format!("hash-{file_id}"),
            index_version: 1,
            extractor_version: 1,
            state: IndexState::Indexed,
            last_indexed_at: Utc::now(),
            error: None,
        };
        let chunks = vec![IndexChunkRecord {
            id: chunk_id,
            file_id,
            ordinal: 0,
            start_line,
            end_line,
            content: content.to_string(),
            content_hash: format!("chunk-{chunk_id}"),
            estimated_tokens: content.len().div_ceil(4) as u32,
        }];
        let symbols = symbol
            .map(|name| {
                vec![IndexSymbolRecord {
                    id: stable_uuid(&format!("{path}:{name}")),
                    file_id,
                    kind: "Struct".into(),
                    name: name.into(),
                    qualified_name: None,
                    start_line,
                    end_line,
                    parent_symbol: None,
                }]
            })
            .unwrap_or_default();

        Self {
            file,
            chunks,
            symbols,
        }
    }
}

#[derive(Clone)]
struct MemoryRetrievalRepository {
    rows: Vec<TestIndexedRow>,
}

impl MemoryRetrievalRepository {
    fn from_rows(rows: Vec<TestIndexedRow>) -> Self {
        Self { rows }
    }
}

impl RetrievalRepository for MemoryRetrievalRepository {
    fn indexed_files_in_roots(
        &self,
        _allowed_roots: &[PathBuf],
        _limit: usize,
    ) -> Result<Vec<IndexedFileRecord>, aether_retrieval::RetrievalError> {
        Ok(self.rows.iter().map(|row| row.file.clone()).collect())
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
            .find(|row| row.file.id == file_id)
            .map(|row| row.chunks.clone())
            .unwrap_or_default())
    }

    fn lexical_candidates(
        &self,
        terms: &[String],
        _allowed_roots: &[PathBuf],
        _limit: usize,
    ) -> Result<Vec<IndexChunkRecord>, aether_retrieval::RetrievalError> {
        let terms = terms
            .iter()
            .map(|term| term.to_ascii_lowercase())
            .collect::<Vec<_>>();
        Ok(self
            .rows
            .iter()
            .flat_map(|row| row.chunks.iter())
            .filter(|chunk| {
                let lower = chunk.content.to_ascii_lowercase();
                terms.iter().any(|term| lower.contains(term))
            })
            .cloned()
            .collect())
    }

    fn symbol_candidates(
        &self,
        query: &str,
        _allowed_roots: &[PathBuf],
        _limit: usize,
    ) -> Result<Vec<IndexSymbolRecord>, aether_retrieval::RetrievalError> {
        let query = query.to_ascii_lowercase();
        Ok(self
            .rows
            .iter()
            .flat_map(|row| row.symbols.iter())
            .filter(|symbol| {
                symbol.name.to_ascii_lowercase().contains(&query)
                    || symbol
                        .qualified_name
                        .as_ref()
                        .is_some_and(|name| name.to_ascii_lowercase().contains(&query))
            })
            .cloned()
            .collect())
    }
}

fn request(query: &str) -> RetrievalRequest {
    RetrievalRequest {
        conversation_id: Uuid::nil(),
        project_id: None,
        query: query.into(),
        max_context_tokens: 1024,
        max_sources: 8,
        allowed_roots: vec![PathBuf::from("/project")],
        modes: RetrievalModes::lexical_and_symbols(),
    }
}

#[test]
fn exact_symbol_and_filename_beat_generic_term_matches() {
    let repo = MemoryRetrievalRepository::from_rows(vec![
        TestIndexedRow::rust(
            "/project/crates/aether-chat/src/lib.rs",
            1,
            1,
            "pub struct ChatService { }",
            Some("ChatService"),
        ),
        TestIndexedRow::text(
            "/project/docs/chat.md",
            1,
            1,
            "The chat service coordinates work.",
        ),
    ]);

    let result = RetrievalService::new(repo)
        .retrieve_sync(request("ChatService aether-chat/src/lib.rs"))
        .unwrap();

    assert_eq!(
        result.items[0].source.display_path(),
        "crates/aether-chat/src/lib.rs"
    );
    assert!(
        result.items[0]
            .rationale
            .contains(&RetrievalSignal::ExactSymbol)
    );
    assert!(
        result.items[0]
            .rationale
            .contains(&RetrievalSignal::ExactFilename)
    );
}

#[test]
fn repeated_retrieval_is_bitwise_order_deterministic_for_ids_and_scores() {
    let repo = MemoryRetrievalRepository::from_rows(vec![
        TestIndexedRow::text("/project/a.txt", 1, 1, "alpha beta"),
        TestIndexedRow::text("/project/b.txt", 1, 1, "alpha beta"),
    ]);
    let service = RetrievalService::new(repo);

    let first = service.retrieve_sync(request("alpha")).unwrap();
    let second = service.retrieve_sync(request("alpha")).unwrap();

    let a = first
        .items
        .iter()
        .map(|item| {
            (
                item.id,
                item.score_milli,
                item.source.display_path().to_string(),
            )
        })
        .collect::<Vec<_>>();
    let b = second
        .items
        .iter()
        .map(|item| {
            (
                item.id,
                item.score_milli,
                item.source.display_path().to_string(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(a, b);
}

#[test]
fn high_scoring_source_outside_allowed_roots_never_appears() {
    let repo = MemoryRetrievalRepository::from_rows(vec![
        TestIndexedRow::rust(
            "/secret/src/lib.rs",
            1,
            1,
            "pub struct SuperExactSecret {}",
            Some("SuperExactSecret"),
        ),
        TestIndexedRow::text("/project/readme.md", 1, 1, "SuperExactSecret reference"),
    ]);

    let result = RetrievalService::new(repo)
        .retrieve_sync(request("SuperExactSecret lib.rs"))
        .unwrap();

    assert!(result.items.iter().all(|item| {
        item.source
            .canonical_path()
            .starts_with(Path::new("/project"))
    }));
    assert!(
        result
            .trace
            .excluded
            .iter()
            .any(|candidate| candidate.exclusion == Some(ExclusionReason::Permission))
    );
}

#[test]
fn source_and_context_budgets_are_strict_and_trace_exclusions() {
    let repo = MemoryRetrievalRepository::from_rows(vec![
        TestIndexedRow::text("/project/a.txt", 1, 1, "alpha one"),
        TestIndexedRow::text("/project/b.txt", 1, 1, "alpha two"),
        TestIndexedRow::text("/project/c.txt", 1, 1, "alpha three"),
    ]);
    let service = RetrievalService::new(repo);

    let mut limited = request("alpha");
    limited.max_sources = 1;
    let by_source = service.retrieve_sync(limited).unwrap();
    assert_eq!(by_source.items.len(), 1);
    assert!(
        by_source
            .trace
            .excluded
            .iter()
            .any(|candidate| candidate.exclusion == Some(ExclusionReason::SourceLimit))
    );

    let mut tiny = request("alpha");
    tiny.max_context_tokens = 1;
    let by_tokens = service.retrieve_sync(tiny).unwrap();
    assert!(by_tokens.items.is_empty());
    assert!(
        by_tokens
            .trace
            .excluded
            .iter()
            .any(|candidate| candidate.exclusion == Some(ExclusionReason::ContextBudget))
    );
}

#[test]
fn no_embedding_model_is_required_for_task8() {
    let repo = MemoryRetrievalRepository::from_rows(vec![TestIndexedRow::text(
        "/project/a.txt",
        1,
        1,
        "alpha",
    )]);
    let result = RetrievalService::new(repo)
        .retrieve_sync(request("alpha"))
        .unwrap();

    assert_eq!(
        result.trace.semantic,
        SemanticAvailability::DisabledByPreset
    );
    assert!(!result.items.is_empty());
}

#[test]
fn file_source_keeps_exact_range_and_index_hash() {
    let repo = MemoryRetrievalRepository::from_rows(vec![TestIndexedRow::text(
        "/project/a.txt",
        7,
        9,
        "alpha",
    )]);
    let result = RetrievalService::new(repo)
        .retrieve_sync(request("alpha"))
        .unwrap();

    match &result.items[0].source {
        ContextSource::File {
            indexed_content_hash,
            range,
            ..
        } => {
            assert!(!indexed_content_hash.is_empty());
            assert_eq!(
                *range,
                SourceRange {
                    start_line: 7,
                    end_line: 9
                }
            );
        }
        other => panic!("expected File source, got {other:?}"),
    }
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
