use std::path::PathBuf;

use aether_core::{MemoryCategory, MemoryItem, MemoryScope, MemorySourceType};
use aether_retrieval::{
    ContextSource, RetrievalModes, RetrievalRepository, RetrievalRequest, RetrievalService,
    RetrievalSignal,
};
use aether_storage::models::{IndexChunkRecord, IndexSymbolRecord, IndexedFileRecord};
use uuid::Uuid;

#[derive(Clone)]
struct MemoryRepo {
    memories: Vec<MemoryItem>,
}

impl RetrievalRepository for MemoryRepo {
    fn indexed_files_in_roots(
        &self,
        _allowed_roots: &[PathBuf],
        _limit: usize,
    ) -> Result<Vec<IndexedFileRecord>, aether_retrieval::RetrievalError> {
        Ok(Vec::new())
    }

    fn indexed_file_by_id(
        &self,
        _file_id: Uuid,
    ) -> Result<Option<IndexedFileRecord>, aether_retrieval::RetrievalError> {
        Ok(None)
    }

    fn chunks_for_file(
        &self,
        _file_id: Uuid,
    ) -> Result<Vec<IndexChunkRecord>, aether_retrieval::RetrievalError> {
        Ok(Vec::new())
    }

    fn lexical_candidates(
        &self,
        _terms: &[String],
        _allowed_roots: &[PathBuf],
        _limit: usize,
    ) -> Result<Vec<IndexChunkRecord>, aether_retrieval::RetrievalError> {
        Ok(Vec::new())
    }

    fn symbol_candidates(
        &self,
        _query: &str,
        _allowed_roots: &[PathBuf],
        _limit: usize,
    ) -> Result<Vec<IndexSymbolRecord>, aether_retrieval::RetrievalError> {
        Ok(Vec::new())
    }

    fn active_memory(
        &self,
        _project_id: Option<Uuid>,
        _conversation_id: Uuid,
    ) -> Result<Vec<MemoryItem>, aether_retrieval::RetrievalError> {
        Ok(self.memories.clone())
    }
}

fn memory(project_id: Uuid, category: MemoryCategory, summary: &str, source: &str) -> MemoryItem {
    let mut item = MemoryItem::new(
        MemoryScope::Project(project_id),
        category,
        summary,
        MemorySourceType::DeterministicSystem,
    );
    item.source_links.push(source.into());
    item
}

#[test]
fn current_baseline_and_exact_version_terms_rank_memory_deterministically() {
    let project_id = Uuid::new_v4();
    let repo = MemoryRepo {
        memories: vec![
            memory(
                project_id,
                MemoryCategory::Requirement,
                "Support version 0.3.0 during development",
                "requirement:1",
            ),
            memory(
                project_id,
                MemoryCategory::CurrentBaseline,
                "Current baseline 0.3.0 (AETHERAI_V0_3_0_VERIFY=PASS)",
                "/project/AetherAI-v0.3.0-VERIFY.txt#L23",
            ),
        ],
    };

    let request = RetrievalRequest {
        conversation_id: Uuid::nil(),
        project_id: Some(project_id),
        query: "0.3.0 AETHERAI_V0_3_0_VERIFY".into(),
        max_context_tokens: 1024,
        max_sources: 4,
        allowed_roots: vec![PathBuf::from("/project")],
        modes: RetrievalModes {
            path: false,
            lexical: false,
            symbols: false,
            memory: true,
            semantic: false,
            git: false,
        },
    };

    let first = RetrievalService::new(repo.clone())
        .retrieve_sync(request.clone())
        .unwrap();
    let second = RetrievalService::new(repo).retrieve_sync(request).unwrap();

    assert_eq!(first.items, second.items);
    assert_eq!(first.items.len(), 2);
    assert!(first.items[0].rationale.contains(&RetrievalSignal::Memory));

    match &first.items[0].source {
        ContextSource::Memory {
            memory_id: _,
            display_path,
            source_links,
        } => {
            assert_eq!(display_path, "Project Memory · CurrentBaseline");
            assert_eq!(
                source_links,
                &vec!["/project/AetherAI-v0.3.0-VERIFY.txt#L23".to_string()]
            );
        }
        other => panic!("expected memory source, got {other:?}"),
    }
}
