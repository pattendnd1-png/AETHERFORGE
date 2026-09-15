use std::fs;

use aether_core::{MemoryCategory, MemoryScope, MemorySourceType, MemoryStatus};
use aether_storage::{MAX_MEMORY_SUMMARY_BYTES, SqliteStore};
use uuid::Uuid;

#[test]
fn user_memory_creation_rejects_oversize_instead_of_truncating() {
    let root = std::env::temp_dir().join(format!("aetherai-task10-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    let project_id = Uuid::new_v4();

    let too_long = "x".repeat(MAX_MEMORY_SUMMARY_BYTES + 1);
    let error = store
        .create_memory(
            MemoryScope::Project(project_id),
            MemoryCategory::Requirement,
            &too_long,
            MemorySourceType::UserExplicit,
        )
        .unwrap_err();

    assert!(error.to_string().contains("summary"));
    assert!(
        store
            .list_active_memory(Some(project_id), Uuid::nil())
            .unwrap()
            .is_empty()
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn checkpoint_upsert_is_idempotent_and_supersedes_previous_baseline() {
    let root = std::env::temp_dir().join(format!("aetherai-task10-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    let project_id = Uuid::new_v4();

    let first = store
        .upsert_checkpoint_memory(
            project_id,
            Some("0.2.2"),
            "AETHERAI_V0_2_2_VERIFY=PASS",
            "/project/AetherAI-v0.2.2-VERIFY.txt#L3",
        )
        .unwrap();

    let repeated = store
        .upsert_checkpoint_memory(
            project_id,
            Some("0.2.2"),
            "AETHERAI_V0_2_2_VERIFY=PASS",
            "/project/AetherAI-v0.2.2-VERIFY.txt#L3",
        )
        .unwrap();

    assert_eq!(first.0.id, repeated.0.id);
    assert_eq!(first.1.id, repeated.1.id);

    let second = store
        .upsert_checkpoint_memory(
            project_id,
            Some("0.3.0"),
            "AETHERAI_V0_3_0_VERIFY=PASS",
            "/project/AetherAI-v0.3.0-VERIFY.txt#L5",
        )
        .unwrap();

    let old_baseline = store.memory_by_id(first.1.id).unwrap().unwrap();
    let new_baseline = store.memory_by_id(second.1.id).unwrap().unwrap();

    assert_eq!(old_baseline.status, MemoryStatus::Superseded);
    assert_eq!(new_baseline.status, MemoryStatus::Active);
    assert_eq!(new_baseline.category, MemoryCategory::CurrentBaseline);
    assert!(new_baseline.source_links[0].ends_with("#L5"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn edit_archive_supersede_and_delete_never_delete_linked_source_file() {
    let root = std::env::temp_dir().join(format!("aetherai-task10-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("source.txt");
    fs::write(&source, b"keep-me").unwrap();

    let store = SqliteStore::open(root.join("db.sqlite")).unwrap();
    let project_id = Uuid::new_v4();
    let mut memory = store
        .create_memory(
            MemoryScope::Project(project_id),
            MemoryCategory::DesignDecision,
            "Use local retrieval",
            MemorySourceType::UserApproved,
        )
        .unwrap();

    memory.source_links = vec![source.display().to_string()];
    store.save_memory(&memory).unwrap();

    let edited = store
        .edit_memory(memory.id, "Use local-first retrieval")
        .unwrap()
        .unwrap();
    assert_eq!(edited.summary, "Use local-first retrieval");

    store.archive_memory(memory.id).unwrap();
    assert_eq!(
        store.memory_by_id(memory.id).unwrap().unwrap().status,
        MemoryStatus::Archived
    );

    store.activate_memory(memory.id).unwrap();
    store.supersede_memory(memory.id).unwrap();
    assert_eq!(
        store.memory_by_id(memory.id).unwrap().unwrap().status,
        MemoryStatus::Superseded
    );

    store.delete_memory(memory.id).unwrap();
    assert!(store.memory_by_id(memory.id).unwrap().is_none());
    assert_eq!(fs::read(&source).unwrap(), b"keep-me");

    let _ = fs::remove_dir_all(root);
}
