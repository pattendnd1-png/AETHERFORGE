use std::fs;

use aether_import::import_chatgpt_extracted_all;
use aether_storage::SqliteStore;
use chrono::{TimeZone, Utc};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 2, 17, 0, 0).unwrap()
}

#[test]
fn import_all_merges_numbered_conversation_documents_and_is_idempotent() {
    let root = std::env::temp_dir().join(format!(
        "aetherai-task11a-stage6-import-all-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let extracted = root.join("export");
    fs::create_dir_all(&extracted).unwrap();

    fs::write(
        extracted.join("conversations.json"),
        br#"[{
          "id":"chat-1",
          "title":"First",
          "project":{"id":"project-1","name":"Imported Project"},
          "mapping":{"n1":{"id":"n1","parent":null,"message":{
            "id":"message-1","author":{"role":"user"},
            "content":{"parts":["alpha"]}
          }}}
        }]"#,
    )
    .unwrap();

    fs::write(
        extracted.join("conversations-1.json"),
        br#"{"conversations":[{
          "id":"chat-2",
          "title":"Second",
          "project_id":"project-1",
          "mapping":{"n2":{"id":"n2","parent":null,"message":{
            "id":"message-2","author":{"role":"assistant"},
            "content":{"parts":["beta"]}
          }}}
        }]}"#,
    )
    .unwrap();

    let store = SqliteStore::open(root.join("aetherai.db")).unwrap();
    let first =
        import_chatgpt_extracted_all(&store, &extracted, "stage6-fingerprint", now()).unwrap();

    assert_eq!(first.project_count, 1);
    assert_eq!(first.conversation_count, 2);
    assert_eq!(first.message_count, 2);
    assert_eq!(store.list_projects().unwrap().len(), 1);
    assert_eq!(store.list_conversations().unwrap().len(), 2);

    let second =
        import_chatgpt_extracted_all(&store, &extracted, "stage6-fingerprint", now()).unwrap();

    assert_eq!(second.conversation_count, 2);
    assert_eq!(store.list_projects().unwrap().len(), 1);
    assert_eq!(store.list_conversations().unwrap().len(), 2);

    let _ = fs::remove_dir_all(root);
}
