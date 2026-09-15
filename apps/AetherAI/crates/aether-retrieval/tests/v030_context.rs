use std::path::{Path, PathBuf};

use aether_core::{LocalCitation, SourceRange};
use aether_index::FreshnessReport;
use aether_retrieval::{
    ContextItem, ContextSource, FreshnessChecker, RetrievalModes, RetrievalSignal, RetrievalTrace,
    RetrievedContext, SemanticAvailability, TraceCandidate, build_context_messages,
    ensure_retrieved_context_fresh,
};
use uuid::Uuid;

#[derive(Clone, Copy)]
struct FakeFreshness {
    changed: bool,
}

impl FreshnessChecker for FakeFreshness {
    fn ensure_fresh_path(&self, _path: &Path) -> Result<FreshnessReport, String> {
        if self.changed {
            Ok(FreshnessReport {
                checked_files: 1,
                fresh_files: 0,
                reindexed_files: 1,
                removed_files: 0,
                failed_files: 0,
            })
        } else {
            Ok(FreshnessReport {
                checked_files: 1,
                fresh_files: 1,
                reindexed_files: 0,
                removed_files: 0,
                failed_files: 0,
            })
        }
    }
}

fn file_item() -> ContextItem {
    ContextItem {
        id: Uuid::new_v4(),
        chunk_id: Some(Uuid::new_v4()),
        source: ContextSource::File {
            canonical_path: PathBuf::from("/project/src/lib.rs"),
            display_path: "src/lib.rs".into(),
            indexed_content_hash: "old-hash".into(),
            range: SourceRange {
                start_line: 91,
                end_line: 142,
            },
        },
        content: "pub struct ChatService;".into(),
        estimated_tokens: 5,
        score_milli: 1200,
        rationale: vec![RetrievalSignal::ExactFilename],
    }
}

fn memory_item() -> ContextItem {
    ContextItem {
        id: Uuid::new_v4(),
        chunk_id: None,
        source: ContextSource::Memory {
            memory_id: Uuid::new_v4(),
            display_path: "Project Memory · CurrentBaseline".into(),
            source_links: vec!["checkpoint:0.3.0".into()],
        },
        content: "AetherAI v0.3.0 verified".into(),
        estimated_tokens: 5,
        score_milli: 900,
        rationale: vec![RetrievalSignal::Memory],
    }
}

fn retrieved(items: Vec<ContextItem>) -> RetrievedContext {
    let estimated_tokens = items.iter().map(|item| item.estimated_tokens).sum();
    RetrievedContext {
        items,
        estimated_tokens,
        trace: RetrievalTrace {
            query: "ChatService".into(),
            modes: RetrievalModes::lexical_and_symbols(),
            selected: vec![TraceCandidate::selected("src/lib.rs", 1200)],
            excluded: Vec::new(),
            semantic: SemanticAvailability::DisabledByPreset,
            semantic_error: None,
            estimated_tokens,
        },
    }
}

#[test]
fn current_file_context_produces_exact_local_citation() {
    let mut context = retrieved(vec![file_item()]);
    let citations =
        ensure_retrieved_context_fresh(&mut context, &FakeFreshness { changed: false }).unwrap();

    assert_eq!(
        citations,
        vec![LocalCitation {
            canonical_path: PathBuf::from("/project/src/lib.rs"),
            display_path: "src/lib.rs".into(),
            range: SourceRange {
                start_line: 91,
                end_line: 142,
            },
            indexed_content_hash: "old-hash".into(),
            stale: false,
        }]
    );
    assert_eq!(context.items.len(), 1);
}

#[test]
fn changed_file_is_not_injected_as_current_and_citation_is_stale() {
    let mut context = retrieved(vec![file_item(), memory_item()]);
    let citations =
        ensure_retrieved_context_fresh(&mut context, &FakeFreshness { changed: true }).unwrap();

    assert_eq!(citations.len(), 1);
    assert!(citations[0].stale);
    assert_eq!(context.items.len(), 1);
    assert!(matches!(
        context.items[0].source,
        ContextSource::Memory { .. }
    ));
}

#[test]
fn context_formatter_uses_stable_file_and_memory_labels() {
    let messages = build_context_messages(&retrieved(vec![file_item(), memory_item()]));
    assert_eq!(messages.len(), 1);
    let body = &messages[0].content;
    assert!(body.contains("[LOCAL_SOURCE S1]"));
    assert!(body.contains("path=src/lib.rs"));
    assert!(body.contains("lines=91-142"));
    assert!(body.contains("[PROJECT_MEMORY M1]"));
    assert!(body.contains("category=CurrentBaseline"));
}
