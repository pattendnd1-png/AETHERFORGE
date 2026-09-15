use std::{
    fs,
    path::{Path, PathBuf},
};

use aether_import::{
    ExternalAttachment, ExternalSourceKey, bridge_imported_verify_checkpoint,
    parse_chatgpt_conversations_json, resolve_extracted_attachment_path,
};
use aether_storage::SqliteStore;
use aether_storage::models::ExternalRecordKind;
use chrono::{TimeZone, Utc};
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap()
}

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "aetherai-task11a-stage5-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn parser_preserves_explicit_attachment_reference_and_resolves_only_real_extracted_file() {
    let json = br#"
[
  {
    "id":"chat-attachments",
    "mapping":{
      "n":{"id":"n","parent":null,"message":{
        "id":"message-attachments",
        "author":{"role":"user"},
        "content":{"parts":["see file"]},
        "metadata":{"attachments":[
          {
            "id":"file-1",
            "name":"AetherAI-v0.3.0-VERIFY.txt",
            "file_path":"files/AetherAI-v0.3.0-VERIFY.txt"
          }
        ]}
      }}
    }
  }
]
"#;

    let parsed = parse_chatgpt_conversations_json(json, "attachment-fp", now()).unwrap();
    assert_eq!(parsed.attachments.len(), 1);
    let attachment = &parsed.attachments[0];
    assert_eq!(attachment.key.external_id, "file-1");
    assert_eq!(attachment.file_name, "AetherAI-v0.3.0-VERIFY.txt");
    assert_eq!(
        attachment.extracted_path.as_deref(),
        Some(Path::new("files/AetherAI-v0.3.0-VERIFY.txt"))
    );

    let root = temp_root("resolve");
    fs::create_dir_all(root.join("files")).unwrap();
    fs::write(
        root.join("files/AetherAI-v0.3.0-VERIFY.txt"),
        b"AETHERAI_VERSION=0.3.0\nAETHERAI_V0_3_0_VERIFY=PASS\n",
    )
    .unwrap();

    let resolved = resolve_extracted_attachment_path(&root, attachment)
        .unwrap()
        .unwrap();
    assert!(resolved.is_file());
    assert!(resolved.starts_with(root.canonicalize().unwrap()));

    let missing = ExternalAttachment {
        key: ExternalSourceKey::chatgpt(ExternalRecordKind::Attachment, "missing"),
        conversation_external_id: "chat-attachments".into(),
        file_name: "missing.txt".into(),
        extracted_path: Some(PathBuf::from("files/missing.txt")),
    };
    assert_eq!(
        resolve_extracted_attachment_path(&root, &missing).unwrap(),
        None
    );

    let traversal = ExternalAttachment {
        key: ExternalSourceKey::chatgpt(ExternalRecordKind::Attachment, "escape"),
        conversation_external_id: "chat-attachments".into(),
        file_name: "escape.txt".into(),
        extracted_path: Some(PathBuf::from("../escape.txt")),
    };
    assert!(resolve_extracted_attachment_path(&root, &traversal).is_err());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn only_real_verify_filename_can_create_deterministic_checkpoint_memory() {
    let root = temp_root("checkpoint");
    let store = SqliteStore::open(root.join("memory.db")).unwrap();
    let project_id = Uuid::new_v4();

    let chat_text = "AETHERAI_VERSION=0.3.0\nAETHERAI_V0_3_0_VERIFY=PASS\n";
    assert!(
        bridge_imported_verify_checkpoint(
            &store,
            project_id,
            Path::new("conversation.txt"),
            chat_text,
        )
        .unwrap()
        .is_none()
    );
    assert!(
        store
            .list_active_memory(Some(project_id), Uuid::nil())
            .unwrap()
            .is_empty()
    );

    let strict = concat!(
        "AETHERAI_VERSION=0.3.0\n",
        "AETHERAI_PLATFORM=linux-x86_64\n",
        "AETHERAI_V0_3_0_VERIFY=PASS\n",
    );
    let created = bridge_imported_verify_checkpoint(
        &store,
        project_id,
        Path::new("AetherAI-v0.3.0-VERIFY.txt"),
        strict,
    )
    .unwrap()
    .unwrap();

    assert_eq!(created.0.source_links.len(), 1);
    assert!(created.0.source_links[0].ends_with("#L3"));
    let active = store
        .list_active_memory(Some(project_id), Uuid::nil())
        .unwrap();
    assert_eq!(active.len(), 2);

    let _ = fs::remove_dir_all(root);
}
