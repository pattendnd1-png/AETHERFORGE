#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use aether_storage::models::ExternalRecordKind;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Map, Value};

use crate::{
    ChatGptExportManifest, ExternalAttachment, ExternalConversation, ExternalMessage,
    ExternalProject, ExternalSourceKey, ImportError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedChatGptExport {
    pub manifest: ChatGptExportManifest,
    pub projects: Vec<ExternalProject>,
    pub conversations: Vec<ExternalConversation>,
    pub messages: Vec<ExternalMessage>,
    pub attachments: Vec<ExternalAttachment>,
}

pub fn parse_chatgpt_conversations_json(
    bytes: &[u8],
    fingerprint: impl Into<String>,
    imported_at: DateTime<Utc>,
) -> Result<ParsedChatGptExport, ImportError> {
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|error| ImportError::InvalidExport(error.to_string()))?;

    let conversations = if let Some(array) = root.as_array() {
        array
    } else {
        root.as_object()
            .and_then(|object| object.get("conversations"))
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ImportError::UnsupportedSchema("expected conversations array at export root".into())
            })?
    };

    let mut projects = BTreeMap::new();
    let mut parsed_conversations = Vec::new();
    let mut messages = Vec::new();
    let mut attachments = Vec::new();
    let mut seen_conversations = BTreeSet::new();
    let mut seen_messages = BTreeSet::new();
    let mut seen_attachments = BTreeSet::new();

    for value in conversations {
        let object = value.as_object().ok_or_else(|| {
            ImportError::UnsupportedSchema("conversation entry is not an object".into())
        })?;
        let conversation_id =
            first_string(object, &["id", "conversation_id"]).ok_or_else(|| {
                ImportError::UnsupportedSchema("conversation has no stable external id".into())
            })?;

        if !seen_conversations.insert(conversation_id.clone()) {
            continue;
        }

        let project = parse_project(object);
        let project_external_id = project
            .as_ref()
            .map(|project| project.key.external_id.clone())
            .or_else(|| first_string(object, &["project_id"]));

        if let Some(project) = project {
            projects
                .entry(project.key.external_id.clone())
                .or_insert(project);
        }

        parsed_conversations.push(ExternalConversation {
            key: ExternalSourceKey::chatgpt(
                ExternalRecordKind::Conversation,
                conversation_id.clone(),
            ),
            project_external_id,
            title: first_string(object, &["title"])
                .unwrap_or_else(|| "Untitled ChatGPT conversation".into()),
            created_at: timestamp(object.get("create_time")),
            updated_at: timestamp(object.get("update_time")),
        });

        if let Some(mapping) = object.get("mapping").and_then(Value::as_object) {
            parse_mapping(
                &conversation_id,
                mapping,
                &mut seen_messages,
                &mut messages,
                &mut seen_attachments,
                &mut attachments,
            );
        }
    }

    parsed_conversations.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.key.external_id.cmp(&right.key.external_id))
    });
    messages.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.key.external_id.cmp(&right.key.external_id))
    });
    attachments.sort_by(|left, right| {
        left.conversation_external_id
            .cmp(&right.conversation_external_id)
            .then_with(|| left.key.external_id.cmp(&right.key.external_id))
    });

    Ok(ParsedChatGptExport {
        manifest: ChatGptExportManifest {
            fingerprint: fingerprint.into(),
            imported_at,
        },
        projects: projects.into_values().collect(),
        conversations: parsed_conversations,
        messages,
        attachments,
    })
}

fn parse_project(object: &Map<String, Value>) -> Option<ExternalProject> {
    let project = object.get("project")?.as_object()?;
    let id = first_string(project, &["id", "project_id"])?;
    let name = first_string(project, &["name", "title"])?;
    Some(ExternalProject {
        key: ExternalSourceKey::chatgpt(ExternalRecordKind::Project, id),
        name,
        instructions: first_string(project, &["instructions"]),
        source_timestamp: timestamp(
            project
                .get("update_time")
                .or_else(|| project.get("create_time")),
        ),
    })
}

fn parse_mapping(
    conversation_id: &str,
    mapping: &Map<String, Value>,
    seen_messages: &mut BTreeSet<String>,
    messages: &mut Vec<ExternalMessage>,
    seen_attachments: &mut BTreeSet<String>,
    attachments: &mut Vec<ExternalAttachment>,
) {
    let mut node_message_ids = BTreeMap::new();
    for (node_key, value) in mapping {
        if let Some(message_id) = value
            .as_object()
            .and_then(|node| node.get("message"))
            .and_then(Value::as_object)
            .and_then(|message| first_string(message, &["id"]))
        {
            node_message_ids.insert(node_key.clone(), message_id);
        }
    }

    for value in mapping.values() {
        let Some(node) = value.as_object() else {
            continue;
        };
        let Some(message) = node.get("message").and_then(Value::as_object) else {
            continue;
        };
        let Some(message_id) = first_string(message, &["id"]) else {
            continue;
        };

        parse_message_attachments(conversation_id, message, seen_attachments, attachments);

        if !seen_messages.insert(message_id.clone()) {
            continue;
        }

        let parent_external_id = node
            .get("parent")
            .and_then(Value::as_str)
            .and_then(|node_id| node_message_ids.get(node_id))
            .cloned();
        let role = message
            .get("author")
            .and_then(Value::as_object)
            .and_then(|author| first_string(author, &["role"]))
            .unwrap_or_else(|| "unknown".into());

        messages.push(ExternalMessage {
            key: ExternalSourceKey::chatgpt(ExternalRecordKind::Message, message_id),
            conversation_external_id: conversation_id.into(),
            parent_external_id,
            role,
            content: message_content(message.get("content")),
            created_at: timestamp(message.get("create_time")),
        });
    }
}

fn parse_message_attachments(
    conversation_id: &str,
    message: &Map<String, Value>,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<ExternalAttachment>,
) {
    if let Some(items) = message
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("attachments"))
        .and_then(Value::as_array)
    {
        for item in items {
            let Some(object) = item.as_object() else {
                continue;
            };
            push_attachment(conversation_id, object, seen, out);
        }
    }

    if let Some(parts) = message
        .get("content")
        .and_then(Value::as_object)
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
    {
        for part in parts {
            let Some(object) = part.as_object() else {
                continue;
            };
            if object.contains_key("asset_pointer")
                || object.contains_key("file_id")
                || object.contains_key("file_path")
            {
                push_attachment(conversation_id, object, seen, out);
            }
        }
    }
}

fn push_attachment(
    conversation_id: &str,
    object: &Map<String, Value>,
    seen: &mut BTreeSet<String>,
    out: &mut Vec<ExternalAttachment>,
) {
    let relative_path = first_string(object, &["file_path", "path"]);
    let external_id =
        first_string(object, &["id", "file_id", "asset_pointer"]).or_else(|| relative_path.clone());
    let Some(external_id) = external_id else {
        return;
    };
    if !seen.insert(external_id.clone()) {
        return;
    }

    let file_name = first_string(object, &["name", "file_name"])
        .or_else(|| {
            relative_path
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| external_id.clone());

    out.push(ExternalAttachment {
        key: ExternalSourceKey::chatgpt(ExternalRecordKind::Attachment, external_id),
        conversation_external_id: conversation_id.into(),
        file_name,
        extracted_path: relative_path.map(PathBuf::from),
    });
}

fn message_content(value: Option<&Value>) -> String {
    let Some(object) = value.and_then(Value::as_object) else {
        return String::new();
    };
    if let Some(parts) = object.get("parts").and_then(Value::as_array) {
        let mut result = String::new();
        for part in parts {
            if let Some(text) = part.as_str() {
                result.push_str(text);
            } else if let Some(text) = part
                .as_object()
                .and_then(|part| part.get("text"))
                .and_then(Value::as_str)
            {
                result.push_str(text);
            }
        }
        return result;
    }
    object
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .into()
}

fn first_string(object: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn timestamp(value: Option<&Value>) -> Option<DateTime<Utc>> {
    let value = value?;
    if let Some(seconds) = value.as_f64() {
        let whole = seconds.trunc() as i64;
        let nanos = (seconds.fract().abs() * 1_000_000_000.0) as u32;
        return Utc.timestamp_opt(whole, nanos).single();
    }
    value
        .as_str()
        .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
        .map(|value| value.with_timezone(&Utc))
}
