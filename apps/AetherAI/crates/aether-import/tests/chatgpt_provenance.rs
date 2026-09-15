use aether_import::{
    ExternalAttachment, ExternalConversation, ExternalMessage, ExternalProject, ExternalSourceKey,
    stable_external_uuid,
};
use aether_storage::models::{ExternalRecordKind, ExternalSourceType};
use chrono::{TimeZone, Utc};

#[test]
fn stable_external_ids_are_deterministic_and_kind_scoped() {
    let project = ExternalSourceKey::new(
        ExternalSourceType::ChatGptExport,
        ExternalRecordKind::Project,
        "same-external-id",
    );
    let conversation = ExternalSourceKey::new(
        ExternalSourceType::ChatGptExport,
        ExternalRecordKind::Conversation,
        "same-external-id",
    );

    assert_eq!(
        stable_external_uuid(&project),
        stable_external_uuid(&project)
    );
    assert_ne!(
        stable_external_uuid(&project),
        stable_external_uuid(&conversation)
    );
}

#[test]
fn typed_chatgpt_records_preserve_external_relationships_without_inventing_projects() {
    let ts = Utc.with_ymd_and_hms(2026, 8, 30, 12, 0, 0).unwrap();

    let project = ExternalProject {
        key: ExternalSourceKey::chatgpt(ExternalRecordKind::Project, "project-1"),
        name: "AetherAI".into(),
        instructions: None,
        source_timestamp: Some(ts),
    };
    let assigned = ExternalConversation {
        key: ExternalSourceKey::chatgpt(ExternalRecordKind::Conversation, "chat-1"),
        project_external_id: Some("project-1".into()),
        title: "Build AetherAI".into(),
        created_at: Some(ts),
        updated_at: Some(ts),
    };
    let unassigned = ExternalConversation {
        key: ExternalSourceKey::chatgpt(ExternalRecordKind::Conversation, "chat-2"),
        project_external_id: None,
        title: "Unassigned".into(),
        created_at: Some(ts),
        updated_at: Some(ts),
    };
    let message = ExternalMessage {
        key: ExternalSourceKey::chatgpt(ExternalRecordKind::Message, "msg-1"),
        conversation_external_id: "chat-1".into(),
        parent_external_id: None,
        role: "user".into(),
        content: "hit it".into(),
        created_at: Some(ts),
    };
    let attachment = ExternalAttachment {
        key: ExternalSourceKey::chatgpt(ExternalRecordKind::Attachment, "file-1"),
        conversation_external_id: "chat-1".into(),
        file_name: "verify.txt".into(),
        extracted_path: None,
    };

    assert_eq!(project.key.external_id, "project-1");
    assert_eq!(assigned.project_external_id.as_deref(), Some("project-1"));
    assert_eq!(unassigned.project_external_id, None);
    assert_eq!(message.conversation_external_id, "chat-1");
    assert_eq!(attachment.conversation_external_id, "chat-1");
}
