use aether_storage::{
    SqliteStore,
    models::{ExternalRecord, ExternalRecordKind, ExternalSourceType, ImportExportRecord},
};
use chrono::{TimeZone, Utc};
use rusqlite::Connection;
use uuid::Uuid;

fn export(fingerprint: &str) -> ImportExportRecord {
    ImportExportRecord {
        fingerprint: fingerprint.into(),
        source_type: ExternalSourceType::ChatGptExport,
        imported_at: Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0).unwrap(),
        payload_json: format!(r#"{{"fingerprint":"{fingerprint}"}}"#),
    }
}

fn external(id: Uuid, fingerprint: &str, payload: &str) -> ExternalRecord {
    ExternalRecord {
        aether_id: id,
        source_type: ExternalSourceType::ChatGptExport,
        record_kind: ExternalRecordKind::Conversation,
        external_id: "chat-123".into(),
        export_fingerprint: fingerprint.into(),
        source_timestamp: Some(Utc.with_ymd_and_hms(2026, 8, 31, 20, 0, 0).unwrap()),
        updated_at: Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0).unwrap(),
        payload_json: payload.into(),
    }
}

#[test]
fn current_v3_database_gains_import_tables_without_schema_number_bump() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("existing-v3.db");
    let connection = Connection::open(&db).unwrap();
    connection
        .execute_batch(
            r#"
CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
INSERT INTO schema_meta VALUES('schema_version','3');
CREATE TABLE conversations(
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    payload_json TEXT NOT NULL
);
"#,
        )
        .unwrap();
    drop(connection);

    let store = SqliteStore::open(&db).unwrap();
    assert_eq!(store.schema_version().unwrap(), 3);

    let record = export("sha256-export-a");
    store.save_import_export(&record).unwrap();
    assert_eq!(
        store
            .import_export_by_fingerprint("sha256-export-a")
            .unwrap()
            .unwrap(),
        record
    );
}

#[test]
fn repeated_external_upsert_is_idempotent_and_newer_export_updates_payload() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().join("import.db")).unwrap();

    let stable_id = Uuid::from_bytes([7; 16]);
    let first = external(stable_id, "export-a", r#"{"title":"Old"}"#);
    store.upsert_external_record(&first).unwrap();
    store.upsert_external_record(&first).unwrap();

    let listed = store
        .list_external_records(ExternalSourceType::ChatGptExport)
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0], first);

    let mut newer = external(stable_id, "export-b", r#"{"title":"New"}"#);
    newer.updated_at = Utc.with_ymd_and_hms(2026, 9, 1, 13, 0, 0).unwrap();
    store.upsert_external_record(&newer).unwrap();

    let stored = store
        .external_record(
            ExternalSourceType::ChatGptExport,
            ExternalRecordKind::Conversation,
            "chat-123",
        )
        .unwrap()
        .unwrap();

    assert_eq!(stored.aether_id, stable_id);
    assert_eq!(stored.export_fingerprint, "export-b");
    assert_eq!(stored.payload_json, r#"{"title":"New"}"#);
    assert_eq!(
        store
            .list_external_records(ExternalSourceType::ChatGptExport)
            .unwrap()
            .len(),
        1
    );
}
