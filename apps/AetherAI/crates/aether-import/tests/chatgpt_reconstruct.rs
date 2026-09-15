use aether_core::Role;
use aether_import::{
    parse_chatgpt_conversations_json, reconstruct_chatgpt_export, stable_external_uuid,
};
use aether_storage::models::{ExternalRecordKind, ExternalSourceType};
use chrono::{TimeZone, Utc};

fn imported_at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap()
}

#[test]
fn reconstruction_uses_real_aether_domain_and_only_explicit_project_links() {
    let json = br#"
[
  {
    "id":"chat-project",
    "title":"Project Chat",
    "create_time":1788112800.0,
    "update_time":1788116400.0,
    "project":{"id":"project-1","name":"AetherAI","instructions":"Rust first"},
    "mapping":{
      "n1":{"id":"n1","parent":null,"message":{
        "id":"message-1",
        "author":{"role":"user"},
        "create_time":1788112810.0,
        "content":{"parts":["hit it"]}
      }},
      "n2":{"id":"n2","parent":"n1","message":{
        "id":"message-2",
        "author":{"role":"assistant"},
        "create_time":1788112820.0,
        "content":{"parts":["done"]}
      }}
    }
  },
  {
    "id":"chat-free",
    "title":"Free Chat",
    "mapping":{
      "x":{"id":"x","parent":null,"message":{
        "id":"message-3",
        "author":{"role":"system"},
        "content":{"parts":["source system text"]}
      }}
    }
  }
]
"#;

    let parsed = parse_chatgpt_conversations_json(json, "export-fp", imported_at()).unwrap();
    let rebuilt = reconstruct_chatgpt_export(parsed).unwrap();

    assert_eq!(rebuilt.projects.len(), 1);
    assert_eq!(rebuilt.projects[0].name, "AetherAI");
    assert_eq!(rebuilt.projects[0].provenance, "ChatGPTExport");
    assert!(
        rebuilt.projects[0]
            .root
            .starts_with("external://chatgpt/project/")
    );

    let project_chat = rebuilt
        .conversations
        .iter()
        .find(|conversation| conversation.title == "Project Chat")
        .unwrap();
    let workspace = project_chat.workspace.as_ref().unwrap();
    assert_eq!(workspace.project_id, Some(rebuilt.projects[0].id));
    assert!(workspace.roots.is_empty());
    assert_eq!(workspace.root, None);

    let free_chat = rebuilt
        .conversations
        .iter()
        .find(|conversation| conversation.title == "Free Chat")
        .unwrap();
    assert_eq!(free_chat.workspace, None);

    assert_eq!(project_chat.messages.len(), 2);
    assert_eq!(project_chat.messages[0].role, Role::User);
    assert_eq!(project_chat.messages[0].content, "hit it");
    assert_eq!(project_chat.messages[1].role, Role::Assistant);
    assert_eq!(project_chat.messages[1].content, "done");

    let expected_chat_id = stable_external_uuid(&aether_import::ExternalSourceKey::new(
        ExternalSourceType::ChatGptExport,
        ExternalRecordKind::Conversation,
        "chat-project",
    ));
    assert_eq!(project_chat.id, expected_chat_id);

    let expected_message_id = stable_external_uuid(&aether_import::ExternalSourceKey::new(
        ExternalSourceType::ChatGptExport,
        ExternalRecordKind::Message,
        "message-1",
    ));
    assert_eq!(project_chat.messages[0].id, expected_message_id);

    assert!(rebuilt.external_records.len() >= 6);
    assert_eq!(rebuilt.export.fingerprint, "export-fp");
}

#[test]
fn reconstruction_preserves_unknown_roles_in_provenance_but_does_not_fabricate_chat_role() {
    let json = br#"
[
  {
    "id":"future-chat",
    "mapping":{
      "n":{"id":"n","parent":null,"message":{
        "id":"future-message",
        "author":{"role":"future_role"},
        "content":{"parts":["future"]}
      }}
    }
  }
]
"#;

    let parsed = parse_chatgpt_conversations_json(json, "future-fp", imported_at()).unwrap();
    let rebuilt = reconstruct_chatgpt_export(parsed).unwrap();

    let chat = &rebuilt.conversations[0];
    assert!(chat.messages.is_empty());

    let external_message = rebuilt
        .external_records
        .iter()
        .find(|record| record.record_kind == ExternalRecordKind::Message)
        .unwrap();
    assert!(external_message.payload_json.contains("future_role"));
}
