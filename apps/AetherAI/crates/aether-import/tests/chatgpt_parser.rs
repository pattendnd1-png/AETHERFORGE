use aether_import::{ParsedChatGptExport, parse_chatgpt_conversations_json};
use chrono::{TimeZone, Utc};
fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 2, 12, 0, 0).unwrap()
}

#[test]
fn imports_all_conversations_messages_and_explicit_project_metadata() {
    let j=br#"[
 {"id":"c1","title":"AetherAI","project":{"id":"p1","name":"Project One","instructions":"Rust first"},"mapping":{
  "root":{"id":"root","parent":null,"message":null},
  "n1":{"id":"n1","parent":"root","message":{"id":"m1","author":{"role":"user"},"content":{"parts":["hit it"]}}},
  "n2":{"id":"n2","parent":"n1","message":{"id":"m2","author":{"role":"assistant"},"content":{"parts":["work","ing"]}}}
 },"unknown":{"future":true}},
 {"conversation_id":"c2","title":"No Project","mapping":{"x":{"id":"x","parent":null,"message":{"id":"m3","author":{"role":"user"},"content":{"parts":["free"]}}}}}
]"#;
    let p = parse_chatgpt_conversations_json(j, "fp", now()).unwrap();
    assert_eq!(p.projects.len(), 1);
    assert_eq!(p.projects[0].key.external_id, "p1");
    assert_eq!(p.projects[0].instructions.as_deref(), Some("Rust first"));
    assert_eq!(p.conversations.len(), 2);
    assert_eq!(p.messages.len(), 3);
    let c2 = p
        .conversations
        .iter()
        .find(|c| c.key.external_id == "c2")
        .unwrap();
    assert_eq!(c2.project_external_id, None);
    let m2 = p
        .messages
        .iter()
        .find(|m| m.key.external_id == "m2")
        .unwrap();
    assert_eq!(m2.content, "working");
    assert_eq!(m2.parent_external_id.as_deref(), Some("m1"));
}
#[test]
fn accepts_object_root_and_rejects_incompatible_root() {
    let ok = br#"{"conversations":[{"id":"c","mapping":{}}],"future":1}"#;
    assert_eq!(
        parse_chatgpt_conversations_json(ok, "x", now())
            .unwrap()
            .conversations
            .len(),
        1
    );
    let bad = parse_chatgpt_conversations_json(br#"{"wrong":1}"#, "x", now()).unwrap_err();
    assert!(
        bad.to_string()
            .contains("unsupported ChatGPT export schema")
    );
}
#[test]
fn deduplicates_explicit_projects() {
    let j=br#"[{"id":"c1","project":{"id":"p","name":"P"},"mapping":{}},{"id":"c2","project":{"id":"p","name":"P"},"mapping":{}}]"#;
    let ParsedChatGptExport { projects, .. } =
        parse_chatgpt_conversations_json(j, "x", now()).unwrap();
    assert_eq!(projects.len(), 1);
}
