use std::path::PathBuf;

use aether_core::{
    AttachmentKind, AttachmentRecord, IndexState, LocalCitation, MemoryCategory, MemoryItem,
    MemoryScope, MemorySourceType, MemoryStatus, PermissionDecision, PermissionKind,
    ProcessingPreset, RetrievalBudget, SourceRange,
};
use uuid::Uuid;

#[test]
fn adding_attachment_preserves_conversation_identity() {
    let conversation_id = Uuid::new_v4();
    let attachment = AttachmentRecord::new_file(
        conversation_id,
        None,
        PathBuf::from("/tmp/project/src/lib.rs"),
        PermissionDecision::AllowThisChat,
    );

    assert_eq!(attachment.conversation_id, conversation_id);
    assert_eq!(attachment.attachment_kind, AttachmentKind::File);
    assert_eq!(attachment.index_state, IndexState::Pending);
}

#[test]
fn local_citation_preserves_path_range_and_freshness_metadata() {
    let citation = LocalCitation {
        canonical_path: PathBuf::from("/tmp/project/src/lib.rs"),
        display_path: "src/lib.rs".into(),
        range: SourceRange {
            start_line: 7,
            end_line: 19,
        },
        indexed_content_hash: "fixture-sha256".into(),
        stale: false,
    };

    assert_eq!(citation.display_path, "src/lib.rs");
    assert_eq!(citation.range.start_line, 7);
    assert_eq!(citation.range.end_line, 19);
    assert!(!citation.stale);
}

#[test]
fn memory_items_are_concise_active_records() {
    let item = MemoryItem::new(
        MemoryScope::Project(Uuid::nil()),
        MemoryCategory::DesignDecision,
        "Use hybrid lexical + symbol retrieval",
        MemorySourceType::UserApproved,
    );

    assert_eq!(item.status, MemoryStatus::Active);
    assert!(item.summary.len() < 4096);
    assert_eq!(item.confidence, 1.0);
}

#[test]
fn memory_constructor_caps_oversized_source_text() {
    let item = MemoryItem::new(
        MemoryScope::Conversation(Uuid::nil()),
        MemoryCategory::KnownIssue,
        "x".repeat(9000),
        MemorySourceType::UserExplicit,
    );
    assert!(item.summary.len() < 4096);
}

#[test]
fn maximum_retrieval_budget_is_larger_than_fast() {
    let fast = RetrievalBudget::from_preset(ProcessingPreset::Fast);
    let maximum = RetrievalBudget::from_preset(ProcessingPreset::Maximum);

    assert!(maximum.max_context_tokens > fast.max_context_tokens);
    assert!(maximum.max_sources >= fast.max_sources);
    assert!(!fast.allow_semantic);
    assert!(maximum.allow_semantic);
}

#[test]
fn v030_permission_kinds_extend_the_existing_permission_domain() {
    let kinds = [
        PermissionKind::ReadAttachment,
        PermissionKind::IndexRoot,
        PermissionKind::WatchRoot,
        PermissionKind::StoreEmbeddings,
        PermissionKind::PersistProjectMemory,
        PermissionKind::RevealSource,
        PermissionKind::ExtractArchive,
    ];
    assert_eq!(kinds.len(), 7);
}
