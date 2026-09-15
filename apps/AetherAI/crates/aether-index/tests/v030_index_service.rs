use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use aether_core::{
    AttachmentKind, AttachmentRecord, IndexState, PermissionDecision, PermissionKind, ProjectId,
};
use aether_index::{AttachmentPermissionEvaluator, IndexActivitySink, IndexService};
use aether_storage::SqliteStore;
use aether_tools::{
    ActivityEvent, ActivityKind, ActivitySink, ActivityStatus, MemoryActivitySink, ToolError,
};
use chrono::Utc;
use uuid::Uuid;

fn allow(
    _conversation: Uuid,
    _project: Option<ProjectId>,
    _kind: PermissionKind,
    _path: &Path,
) -> PermissionDecision {
    PermissionDecision::AllowThisProject
}

#[derive(Clone)]
struct SharedToolSink(Arc<MemoryActivitySink>);

impl ActivitySink for SharedToolSink {
    fn record(&self, event: ActivityEvent) -> Result<(), ToolError> {
        self.0.record(event)
    }
}

fn root_attachment(conversation_id: Uuid, root: PathBuf) -> AttachmentRecord {
    let now = Utc::now();
    AttachmentRecord {
        attachment_id: Uuid::new_v4(),
        conversation_id,
        project_id: None,
        canonical_path: root,
        attachment_kind: AttachmentKind::ProjectRoot,
        permission_scope: PermissionDecision::AllowThisProject,
        added_at: now,
        last_seen_at: now,
        index_state: IndexState::Pending,
    }
}

fn fixture() -> (
    tempfile::TempDir,
    IndexService<SqliteStore, impl AttachmentPermissionEvaluator, SharedToolSink>,
    AttachmentRecord,
    Arc<MemoryActivitySink>,
) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/a.rs"), "fn alpha() {}\n").unwrap();
    std::fs::write(root.join("README.md"), "# Project\nhello\n").unwrap();

    let store = SqliteStore::open(dir.path().join("aetherai.db")).unwrap();
    let conversation_id = Uuid::new_v4();
    let attachment = root_attachment(conversation_id, root.canonicalize().unwrap());
    store.save_attachment(&attachment).unwrap();

    let activity = Arc::new(MemoryActivitySink::default());
    let service = IndexService::new(store, allow, SharedToolSink(activity.clone()));

    (dir, service, attachment, activity)
}

#[test]
fn second_refresh_skips_unchanged_files_and_audits_counts() {
    let (_dir, service, attachment, activity) = fixture();

    let first = service
        .refresh_attachment(attachment.attachment_id)
        .unwrap();
    let second = service
        .refresh_attachment(attachment.attachment_id)
        .unwrap();

    assert_eq!(first.seen_files, 2);
    assert_eq!(first.indexed_files, 2);
    assert_eq!(second.indexed_files, 0);
    assert_eq!(second.skipped_files, first.seen_files);

    let events = activity.events();
    let last = events.last().unwrap();
    assert_eq!(last.kind, ActivityKind::Index);
    assert_eq!(last.status, ActivityStatus::Passed);
    let details = last.details_json.as_deref().unwrap();
    assert!(details.contains("\"indexed_files\":0"));
    assert!(!details.contains("fn alpha"));
}

#[test]
fn editing_one_file_replaces_only_that_files_records() {
    let (_dir, service, attachment, _activity) = fixture();
    service
        .refresh_attachment(attachment.attachment_id)
        .unwrap();

    let changed = attachment.canonical_path.join("src/a.rs");
    std::fs::write(&changed, "fn changed() {}\n").unwrap();

    let result = service
        .refresh_attachment(attachment.attachment_id)
        .unwrap();

    assert_eq!(result.indexed_files, 1);
    assert_eq!(result.skipped_files, 1);

    let record = service
        .repository()
        .indexed_file_by_path(&changed)
        .unwrap()
        .unwrap();
    assert_eq!(record.state, IndexState::Indexed);
}

#[test]
fn deleted_indexed_file_disappears_without_touching_other_source_files() {
    let (_dir, service, attachment, _activity) = fixture();
    service
        .refresh_attachment(attachment.attachment_id)
        .unwrap();

    let removed = attachment.canonical_path.join("src/a.rs");
    let kept = attachment.canonical_path.join("README.md");
    std::fs::remove_file(&removed).unwrap();

    let result = service
        .refresh_attachment(attachment.attachment_id)
        .unwrap();

    assert_eq!(result.removed_files, 1);
    assert!(
        service
            .repository()
            .indexed_file_by_path(&removed)
            .unwrap()
            .is_none()
    );
    assert!(kept.exists());
    assert_eq!(std::fs::read_to_string(kept).unwrap(), "# Project\nhello\n");
}

#[test]
fn cancellation_before_first_file_preserves_tree_and_marks_job_cancelled() {
    let (_dir, service, attachment, activity) = fixture();
    let cancelled = AtomicBool::new(true);

    let result = service
        .refresh_attachment_with_cancel(attachment.attachment_id, &cancelled)
        .unwrap();

    assert!(result.cancelled);
    assert_eq!(result.indexed_files, 0);
    assert!(attachment.canonical_path.join("src/a.rs").exists());

    let events = activity.events();
    let last = events.last().unwrap();
    assert_eq!(last.kind, ActivityKind::Index);
    assert_eq!(last.status, ActivityStatus::Cancelled);
}

#[test]
fn ensure_fresh_rehashes_and_reindexes_only_selected_stale_file() {
    let (_dir, service, attachment, _activity) = fixture();
    service
        .refresh_attachment(attachment.attachment_id)
        .unwrap();

    let changed = attachment.canonical_path.join("src/a.rs");
    let untouched = attachment.canonical_path.join("README.md");
    let untouched_before = service
        .repository()
        .indexed_file_by_path(&untouched)
        .unwrap()
        .unwrap();

    std::fs::write(&changed, "fn freshness_changed() {}\n").unwrap();

    let report = service
        .ensure_fresh(std::slice::from_ref(&changed))
        .unwrap();

    assert_eq!(report.checked_files, 1);
    assert_eq!(report.reindexed_files, 1);

    let untouched_after = service
        .repository()
        .indexed_file_by_path(&untouched)
        .unwrap()
        .unwrap();
    assert_eq!(untouched_before.content_hash, untouched_after.content_hash);
    assert_eq!(
        untouched_before.last_indexed_at,
        untouched_after.last_indexed_at
    );
}

#[test]
fn missing_attachment_relationship_is_preserved_as_unavailable() {
    let (_dir, service, attachment, _activity) = fixture();
    std::fs::remove_dir_all(&attachment.canonical_path).unwrap();

    let result = service
        .refresh_attachment(attachment.attachment_id)
        .unwrap();

    assert_eq!(result.seen_files, 0);
    let stored = service
        .repository()
        .attachment_by_id(attachment.attachment_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored.index_state, IndexState::Unavailable);
}

#[test]
fn activity_domain_has_task7_kinds() {
    assert_ne!(ActivityKind::Index, ActivityKind::Retrieval);
    assert_ne!(ActivityKind::Retrieval, ActivityKind::Memory);
}

#[test]
fn index_activity_sink_boundary_is_object_safe_enough_for_adapter() {
    fn accepts_sink<T: IndexActivitySink>(_sink: &T) {}
    let sink = SharedToolSink(Arc::new(MemoryActivitySink::default()));
    accepts_sink(&sink);
}
