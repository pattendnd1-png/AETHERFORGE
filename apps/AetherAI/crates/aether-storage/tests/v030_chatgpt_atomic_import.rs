use aether_core::Conversation;
use aether_storage::{
    SqliteStore,
    models::{
        ExternalRecord, ExternalRecordKind, ExternalSourceType, ImportExportRecord, ProjectRecord,
    },
};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap()
}

fn export() -> ImportExportRecord {
    ImportExportRecord {
        fingerprint: "atomic-fp".into(),
        source_type: ExternalSourceType::ChatGptExport,
        imported_at: now(),
        payload_json: r#"{"fingerprint":"atomic-fp"}"#.into(),
    }
}

#[test]
fn atomic_external_import_commits_all_domain_and_provenance_records() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().join("atomic.db")).unwrap();

    let project = ProjectRecord {
        id: Uuid::new_v4(),
        name: "Imported".into(),
        root: "external://chatgpt/project/imported".into(),
        provenance: "ChatGPTExport".into(),
    };

    let mut conversation = Conversation::new("Imported Chat");
    conversation.id = Uuid::new_v4();

    let external = ExternalRecord {
        aether_id: conversation.id,
        source_type: ExternalSourceType::ChatGptExport,
        record_kind: ExternalRecordKind::Conversation,
        external_id: "chat-1".into(),
        export_fingerprint: "atomic-fp".into(),
        source_timestamp: Some(now()),
        updated_at: now(),
        payload_json: r#"{"external_id":"chat-1"}"#.into(),
    };

    store
        .commit_external_import(
            &export(),
            std::slice::from_ref(&project),
            std::slice::from_ref(&conversation),
            std::slice::from_ref(&external),
        )
        .unwrap();

    assert_eq!(store.list_projects().unwrap(), vec![project]);
    assert_eq!(
        store.load_conversation(conversation.id).unwrap().unwrap(),
        conversation
    );
    assert_eq!(
        store
            .external_record(
                ExternalSourceType::ChatGptExport,
                ExternalRecordKind::Conversation,
                "chat-1",
            )
            .unwrap()
            .unwrap(),
        external
    );
    assert!(
        store
            .import_export_by_fingerprint("atomic-fp")
            .unwrap()
            .is_some()
    );
}

#[test]
fn atomic_external_import_rolls_back_everything_on_mid_batch_failure() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().join("rollback.db")).unwrap();

    let p1 = ProjectRecord {
        id: Uuid::new_v4(),
        name: "One".into(),
        root: "external://same-root".into(),
        provenance: "ChatGPTExport".into(),
    };
    let p2 = ProjectRecord {
        id: Uuid::new_v4(),
        name: "Two".into(),
        root: "external://same-root".into(),
        provenance: "ChatGPTExport".into(),
    };
    let conversation = Conversation::new("Must Roll Back");

    assert!(
        store
            .commit_external_import(
                &export(),
                &[p1, p2],
                std::slice::from_ref(&conversation),
                &[],
            )
            .is_err()
    );

    assert!(
        store
            .import_export_by_fingerprint("atomic-fp")
            .unwrap()
            .is_none()
    );
    assert!(store.list_projects().unwrap().is_empty());
    assert!(store.load_conversation(conversation.id).unwrap().is_none());
}
