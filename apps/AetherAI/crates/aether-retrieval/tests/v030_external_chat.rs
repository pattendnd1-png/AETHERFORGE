use aether_retrieval::{
    ContextSource, RetrievalModes, RetrievalRequest, RetrievalService, RetrievalSignal,
    build_context_messages, citations_for_context,
};
use aether_storage::{
    SqliteStore,
    models::{ExternalRecord, ExternalRecordKind, ExternalSourceType},
};
use chrono::{TimeZone, Utc};
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap()
}

fn record(
    kind: ExternalRecordKind,
    external_id: &str,
    aether_id: Uuid,
    payload_json: &str,
) -> ExternalRecord {
    ExternalRecord {
        aether_id,
        source_type: ExternalSourceType::ChatGptExport,
        record_kind: kind,
        external_id: external_id.into(),
        export_fingerprint: "retrieval-fp".into(),
        source_timestamp: Some(now()),
        updated_at: now(),
        payload_json: payload_json.into(),
    }
}

#[test]
fn imported_chat_is_retrievable_with_typed_non_filesystem_provenance() {
    let root = std::env::temp_dir().join(format!(
        "aetherai-external-chat-retrieval-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let store = SqliteStore::open(root.join("retrieval.db")).unwrap();
    let project_id = Uuid::new_v4();
    let old_conversation_id = Uuid::new_v4();
    let current_conversation_id = Uuid::new_v4();
    let message_id = Uuid::new_v4();

    store
        .upsert_external_record(&record(
            ExternalRecordKind::Project,
            "project-1",
            project_id,
            r#"{"name":"Imported Project"}"#,
        ))
        .unwrap();
    store
        .upsert_external_record(&record(
            ExternalRecordKind::Conversation,
            "chat-old",
            old_conversation_id,
            r#"{"project_external_id":"project-1","title":"Old Dragon Chat"}"#,
        ))
        .unwrap();
    store
        .upsert_external_record(&record(
            ExternalRecordKind::Message,
            "message-old",
            message_id,
            r#"{"conversation_external_id":"chat-old","role":"user","content":"purple dragon architecture"}"#,
        ))
        .unwrap();

    let result = RetrievalService::new(store)
        .retrieve_sync(RetrievalRequest {
            conversation_id: current_conversation_id,
            project_id: Some(project_id),
            query: "purple dragon".into(),
            max_context_tokens: 1000,
            max_sources: 8,
            allowed_roots: Vec::new(),
            modes: RetrievalModes::lexical_and_symbols(),
        })
        .unwrap();

    assert_eq!(result.items.len(), 1);
    assert!(
        result.items[0]
            .rationale
            .contains(&RetrievalSignal::ImportedChat)
    );
    match &result.items[0].source {
        ContextSource::ExternalChat {
            source_type,
            conversation_id,
            message_id: returned_message_id,
            display_path,
            ..
        } => {
            assert_eq!(source_type, "ChatGPTExport");
            assert_eq!(*conversation_id, old_conversation_id);
            assert_eq!(*returned_message_id, message_id);
            assert!(display_path.contains("Old Dragon Chat"));
        }
        other => panic!("expected ExternalChat source, got {other:?}"),
    }

    assert!(citations_for_context(&result).is_empty());
    let messages = build_context_messages(&result);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].content.contains("[IMPORTED_CHAT C1]"));
    assert!(messages[0].content.contains("source=ChatGPTExport"));
    assert!(messages[0].content.contains("purple dragon architecture"));

    let _ = std::fs::remove_dir_all(root);
}
