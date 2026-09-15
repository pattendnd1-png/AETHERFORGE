use std::path::PathBuf;

use aether_core::{
    AttachmentRecord, IndexState, MemoryCategory, MemoryItem, MemoryScope, MemorySourceType,
    MemoryStatus, PermissionDecision,
};
use aether_model_api::{ModelBackend, ModelDescriptor, ModelFormat, ModelLoadState};
use aether_storage::{
    SqliteStore,
    models::{
        ActivityRecord, IndexChunkRecord, IndexSymbolRecord, IndexedFileRecord, ModelRecord,
        ProjectRecord, RetrievalHistoryRecord, vector_from_le_bytes, vector_to_le_bytes,
    },
};
use chrono::Utc;
use rusqlite::{Connection, params};
use uuid::Uuid;

fn create_exact_v2_schema(connection: &Connection) {
    connection
        .execute_batch(
            r#"
CREATE TABLE schema_meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
INSERT INTO schema_meta VALUES('schema_version','2');
CREATE TABLE conversations(id TEXT PRIMARY KEY,title TEXT NOT NULL,updated_at TEXT NOT NULL,payload_json TEXT NOT NULL);
CREATE TABLE ui_state(key TEXT PRIMARY KEY,payload_json TEXT NOT NULL);
CREATE TABLE projects(id TEXT PRIMARY KEY,name TEXT NOT NULL,root TEXT NOT NULL UNIQUE,payload_json TEXT NOT NULL);
CREATE TABLE workspace_roots(conversation_id TEXT NOT NULL,root TEXT NOT NULL,payload_json TEXT NOT NULL,PRIMARY KEY(conversation_id,root));
CREATE TABLE permissions(scope_key TEXT NOT NULL,kind TEXT NOT NULL,decision TEXT NOT NULL,PRIMARY KEY(scope_key,kind));
CREATE TABLE models(id TEXT PRIMARY KEY,backend TEXT NOT NULL,path TEXT NOT NULL,payload_json TEXT NOT NULL);
CREATE TABLE activities(id TEXT PRIMARY KEY,conversation_id TEXT,project_id TEXT,started_at TEXT NOT NULL,payload_json TEXT NOT NULL);
"#,
        )
        .unwrap();
}

fn seed_v2_chat_project_model_activity(connection: &Connection) -> Uuid {
    let conversation = aether_core::Conversation::new("Verified v0.2.2 chat");
    connection
        .execute(
            "INSERT INTO conversations(id,title,updated_at,payload_json) VALUES(?1,?2,?3,?4)",
            params![
                conversation.id.to_string(),
                conversation.title,
                conversation.updated_at.to_rfc3339(),
                serde_json::to_string(&conversation).unwrap()
            ],
        )
        .unwrap();

    let project = ProjectRecord {
        id: Uuid::new_v4(),
        name: "Verified project".into(),
        root: "/tmp/verified-project".into(),
        provenance: "fixture".into(),
    };
    connection
        .execute(
            "INSERT INTO projects(id,name,root,payload_json) VALUES(?1,?2,?3,?4)",
            params![
                project.id.to_string(),
                project.name,
                project.root,
                serde_json::to_string(&project).unwrap()
            ],
        )
        .unwrap();

    let model = ModelRecord {
        descriptor: ModelDescriptor {
            id: "local-fixture".into(),
            display_name: "Local Fixture".into(),
            backend: ModelBackend::AetherGguf,
            format: ModelFormat::Gguf,
            path: "/tmp/model.gguf".into(),
            context_tokens: 8192,
            local: true,
            load_state: ModelLoadState::Registered,
        },
        registered_at: Utc::now(),
        last_used: None,
    };
    connection
        .execute(
            "INSERT INTO models(id,backend,path,payload_json) VALUES(?1,?2,?3,?4)",
            params![
                model.descriptor.id,
                "AetherGguf",
                model.descriptor.path,
                serde_json::to_string(&model).unwrap()
            ],
        )
        .unwrap();

    let activity = ActivityRecord {
        id: Uuid::new_v4(),
        conversation_id: Some(conversation.id),
        project_id: Some(project.id),
        started_at: Utc::now(),
        payload_json: r#"{"fixture":"v2"}"#.into(),
    };
    connection
        .execute(
            "INSERT INTO activities(id,conversation_id,project_id,started_at,payload_json) VALUES(?1,?2,?3,?4,?5)",
            params![
                activity.id.to_string(),
                activity.conversation_id.map(|id| id.to_string()),
                activity.project_id.map(|id| id.to_string()),
                activity.started_at.to_rfc3339(),
                serde_json::to_string(&activity).unwrap()
            ],
        )
        .unwrap();

    conversation.id
}

#[test]
fn v2_database_migrates_to_v3_without_identity_or_state_loss() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("aetherai-v2.db");
    let connection = Connection::open(&db).unwrap();
    create_exact_v2_schema(&connection);
    let chat_id = seed_v2_chat_project_model_activity(&connection);
    drop(connection);

    let store = SqliteStore::open(&db).unwrap();
    assert_eq!(store.schema_version().unwrap(), 3);

    let chat = store.load_conversation(chat_id).unwrap().unwrap();
    assert_eq!(chat.id, chat_id);
    assert_eq!(chat.title, "Verified v0.2.2 chat");
    assert_eq!(store.list_projects().unwrap().len(), 1);
    assert_eq!(store.list_models().unwrap().len(), 1);
    assert_eq!(store.list_activities().unwrap().len(), 1);
    assert!(store.list_attachments(chat_id).unwrap().is_empty());
}

#[test]
fn failed_v3_statement_rolls_back_and_leaves_schema_version_two() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("broken-v2.db");
    let connection = Connection::open(&db).unwrap();
    create_exact_v2_schema(&connection);
    connection
        .execute_batch("CREATE TABLE attachments(dummy TEXT NOT NULL);")
        .unwrap();
    drop(connection);

    assert!(SqliteStore::open(&db).is_err());

    let connection = Connection::open(&db).unwrap();
    let version: String = connection
        .query_row(
            "SELECT value FROM schema_meta WHERE key='schema_version'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, "2");

    let v3_table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='indexed_files'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(v3_table_count, 0);
}

#[test]
fn attachment_repository_round_trips_and_marks_state() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().join("attachments.db")).unwrap();
    let conversation_id = Uuid::new_v4();
    let record = AttachmentRecord::new_file(
        conversation_id,
        None,
        PathBuf::from("/tmp/project/src/lib.rs"),
        PermissionDecision::AllowThisChat,
    );

    store.save_attachment(&record).unwrap();
    let listed = store.list_attachments(conversation_id).unwrap();
    assert_eq!(listed, vec![record.clone()]);

    store
        .mark_attachment_state(record.attachment_id, &IndexState::Indexed)
        .unwrap();
    let listed = store.list_attachments(conversation_id).unwrap();
    assert_eq!(listed[0].index_state, IndexState::Indexed);

    store.remove_attachment(record.attachment_id).unwrap();
    assert!(store.list_attachments(conversation_id).unwrap().is_empty());
}

#[test]
fn deleting_index_metadata_does_not_delete_source_file() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("important.rs");
    std::fs::write(&source, "fn keep_me() {}\n").unwrap();

    let store = SqliteStore::open(dir.path().join("index.db")).unwrap();
    let file_id = Uuid::new_v4();
    let record = IndexedFileRecord {
        id: file_id,
        attachment_id: Uuid::new_v4(),
        canonical_path: source.clone(),
        size_bytes: std::fs::metadata(&source).unwrap().len(),
        modified_ns: 123,
        content_hash: "fixture-hash".into(),
        index_version: 1,
        extractor_version: 1,
        state: IndexState::Indexed,
        last_indexed_at: Utc::now(),
        error: None,
    };

    store.upsert_indexed_file(&record).unwrap();
    assert_eq!(
        store.indexed_file_by_path(&source).unwrap().unwrap(),
        record
    );

    store.remove_file_index(file_id).unwrap();

    assert!(source.exists());
    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        "fn keep_me() {}\n"
    );
    assert!(store.indexed_file_by_path(&source).unwrap().is_none());
}

#[test]
fn replacing_file_index_replaces_chunks_and_lexical_candidates() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("src.rs");
    std::fs::write(&source, "fn alpha() {}\n").unwrap();

    let store = SqliteStore::open(dir.path().join("replace.db")).unwrap();
    let file = IndexedFileRecord {
        id: Uuid::new_v4(),
        attachment_id: Uuid::new_v4(),
        canonical_path: source,
        size_bytes: 14,
        modified_ns: 1,
        content_hash: "hash-1".into(),
        index_version: 1,
        extractor_version: 1,
        state: IndexState::Indexed,
        last_indexed_at: Utc::now(),
        error: None,
    };
    let first = IndexChunkRecord {
        id: Uuid::new_v4(),
        file_id: file.id,
        ordinal: 0,
        start_line: 1,
        end_line: 1,
        content: "fn alpha() {}".into(),
        content_hash: "chunk-1".into(),
        estimated_tokens: 4,
    };

    let symbol = IndexSymbolRecord {
        id: Uuid::new_v4(),
        file_id: file.id,
        kind: "Function".into(),
        name: "alpha".into(),
        qualified_name: Some("crate::alpha".into()),
        start_line: 1,
        end_line: 1,
        parent_symbol: None,
    };

    store
        .replace_file_index(
            &file,
            std::slice::from_ref(&first),
            std::slice::from_ref(&symbol),
            &[("alpha".into(), first.id, 2)],
        )
        .unwrap();

    assert_eq!(
        store
            .symbol_candidates("alpha", std::slice::from_ref(&root), 8)
            .unwrap(),
        vec![symbol]
    );

    let found = store
        .lexical_candidates(&["alpha".into()], std::slice::from_ref(&root), 8)
        .unwrap();
    assert_eq!(found, vec![first.clone()]);

    let second = IndexChunkRecord {
        id: Uuid::new_v4(),
        file_id: file.id,
        ordinal: 0,
        start_line: 1,
        end_line: 1,
        content: "fn beta() {}".into(),
        content_hash: "chunk-2".into(),
        estimated_tokens: 4,
    };
    store
        .replace_file_index(
            &file,
            std::slice::from_ref(&second),
            &[],
            &[("beta".into(), second.id, 1)],
        )
        .unwrap();

    assert!(
        store
            .lexical_candidates(&["alpha".into()], std::slice::from_ref(&root), 8)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .lexical_candidates(&["beta".into()], std::slice::from_ref(&root), 8)
            .unwrap(),
        vec![second]
    );
}

#[test]
fn project_and_conversation_memory_round_trip_with_status_changes() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().join("memory.db")).unwrap();

    let project_id = Uuid::new_v4();
    let conversation_id = Uuid::new_v4();
    let project_memory = MemoryItem::new(
        MemoryScope::Project(project_id),
        MemoryCategory::DesignDecision,
        "Use local SQLite memory",
        MemorySourceType::UserApproved,
    );
    let chat_memory = MemoryItem::new(
        MemoryScope::Conversation(conversation_id),
        MemoryCategory::PendingWork,
        "Finish Task 3",
        MemorySourceType::UserExplicit,
    );

    store.save_memory(&project_memory).unwrap();
    store.save_memory(&chat_memory).unwrap();

    let active = store
        .list_active_memory(Some(project_id), conversation_id)
        .unwrap();
    assert_eq!(active.len(), 2);

    store
        .set_memory_status(project_memory.id, MemoryStatus::Superseded)
        .unwrap();
    let active = store
        .list_active_memory(Some(project_id), conversation_id)
        .unwrap();
    assert_eq!(active, vec![chat_memory.clone()]);

    store.delete_memory(chat_memory.id).unwrap();
    assert!(
        store
            .list_active_memory(Some(project_id), conversation_id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn retrieval_history_is_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(dir.path().join("retrieval.db")).unwrap();
    let conversation_id = Uuid::new_v4();

    for ordinal in 0..4 {
        store
            .append_retrieval_history(&RetrievalHistoryRecord {
                id: Uuid::new_v4(),
                conversation_id,
                project_id: None,
                created_at: Utc::now() + chrono::Duration::seconds(ordinal),
                payload_json: format!(r#"{{"ordinal":{ordinal}}}"#),
            })
            .unwrap();
    }

    store.prune_retrieval_history(2).unwrap();
    assert_eq!(store.retrieval_history_count().unwrap(), 2);
}

#[test]
fn vector_codec_rejects_invalid_lengths_and_dimension_mismatch() {
    let vector = vec![1.0_f32, -2.5, 3.25];
    let bytes = vector_to_le_bytes(&vector);
    assert_eq!(vector_from_le_bytes(&bytes, 3).unwrap(), vector);
    assert!(vector_from_le_bytes(&bytes[..bytes.len() - 1], 3).is_err());
    assert!(vector_from_le_bytes(&bytes, 2).is_err());
}
