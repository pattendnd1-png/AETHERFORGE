#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use aether_core::PermissionDecision;
use aether_storage::SqliteStore;
use chrono::{DateTime, Utc};

use crate::archive::extract_chatgpt_export;
use crate::{
    ExternalSourceKey, ImportError, bridge_imported_verify_checkpoint,
    parse_chatgpt_conversations_json, reconstruct_chatgpt_export, register_imported_attachment,
    stable_external_uuid,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatGptImportSummary {
    pub fingerprint: String,
    pub project_count: usize,
    pub conversation_count: usize,
    pub message_count: usize,
    pub attachment_count: usize,
    pub checkpoint_count: usize,
}

pub fn import_chatgpt_export_all(
    store: &SqliteStore,
    archive: &Path,
    imports_root: &Path,
    imported_at: DateTime<Utc>,
) -> Result<ChatGptImportSummary, ImportError> {
    let archive_bytes =
        fs::read(archive).map_err(|error| ImportError::Archive(error.to_string()))?;
    let fingerprint = aether_index::sha256_hex(&archive_bytes);
    let destination = imports_root.join("chatgpt").join(&fingerprint);

    if !destination.is_dir() {
        let staging = imports_root
            .join("chatgpt")
            .join(format!(".{fingerprint}.staging-{}", std::process::id()));
        if staging.exists() {
            fs::remove_dir_all(&staging)
                .map_err(|error| ImportError::Archive(error.to_string()))?;
        }
        fs::create_dir_all(&staging).map_err(|error| ImportError::Archive(error.to_string()))?;
        if let Err(error) = extract_chatgpt_export(archive, &staging) {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| ImportError::Archive(error.to_string()))?;
        }
        fs::rename(&staging, &destination)
            .map_err(|error| ImportError::Archive(error.to_string()))?;
    }

    import_chatgpt_extracted_all(store, &destination, fingerprint, imported_at)
}

pub fn import_chatgpt_extracted_all(
    store: &SqliteStore,
    extracted_root: &Path,
    fingerprint: impl Into<String>,
    imported_at: DateTime<Utc>,
) -> Result<ChatGptImportSummary, ImportError> {
    let fingerprint = fingerprint.into();
    let documents = conversation_json_documents(extracted_root)?;
    if documents.is_empty() {
        return Err(ImportError::InvalidExport(
            "no conversations JSON document found in ChatGPT export".into(),
        ));
    }

    let merged = merge_conversation_documents(&documents)?;
    let parsed = parse_chatgpt_conversations_json(&merged, fingerprint.clone(), imported_at)?;

    let project_count = parsed.projects.len();
    let conversation_count = parsed.conversations.len();
    let message_count = parsed.messages.len();
    let parsed_for_bridges = parsed.clone();
    let rebuilt = reconstruct_chatgpt_export(parsed)?;

    store.commit_external_import(
        &rebuilt.export,
        &rebuilt.projects,
        &rebuilt.conversations,
        &rebuilt.external_records,
    )?;

    let conversation_projects = parsed_for_bridges
        .conversations
        .iter()
        .map(|conversation| {
            (
                conversation.key.external_id.clone(),
                conversation.project_external_id.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut attachment_count = 0usize;
    let mut checkpoint_count = 0usize;

    for attachment in &parsed_for_bridges.attachments {
        let conversation_key = ExternalSourceKey::chatgpt(
            aether_storage::models::ExternalRecordKind::Conversation,
            attachment.conversation_external_id.clone(),
        );
        let conversation_id = stable_external_uuid(&conversation_key);
        let project_id = conversation_projects
            .get(&attachment.conversation_external_id)
            .and_then(|project| project.as_ref())
            .map(|project_external_id| {
                stable_external_uuid(&ExternalSourceKey::chatgpt(
                    aether_storage::models::ExternalRecordKind::Project,
                    project_external_id.clone(),
                ))
            });

        let permission = if project_id.is_some() {
            PermissionDecision::AllowThisProject
        } else {
            PermissionDecision::AllowThisChat
        };

        if let Some(record) = register_imported_attachment(
            store,
            attachment,
            conversation_id,
            project_id,
            extracted_root,
            permission,
        )? {
            attachment_count += 1;

            if let Some(project_id) = project_id {
                if let Ok(text) = fs::read_to_string(&record.canonical_path) {
                    if bridge_imported_verify_checkpoint(
                        store,
                        project_id,
                        &record.canonical_path,
                        &text,
                    )?
                    .is_some()
                    {
                        checkpoint_count += 1;
                    }
                }
            }
        }
    }

    Ok(ChatGptImportSummary {
        fingerprint,
        project_count,
        conversation_count,
        message_count,
        attachment_count,
        checkpoint_count,
    })
}

fn conversation_json_documents(root: &Path) -> Result<Vec<PathBuf>, ImportError> {
    let mut pending = vec![root.to_path_buf()];
    let mut found = Vec::new();

    while let Some(path) = pending.pop() {
        let mut entries = fs::read_dir(&path)
            .map_err(|error| ImportError::Archive(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| ImportError::Archive(error.to_string()))?;
        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| ImportError::Archive(error.to_string()))?;
            if file_type.is_dir() {
                pending.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            if lower.starts_with("conversations") && lower.ends_with(".json") {
                found.push(path);
            }
        }
    }

    found.sort();
    Ok(found)
}

fn merge_conversation_documents(paths: &[PathBuf]) -> Result<Vec<u8>, ImportError> {
    let mut conversations = Vec::new();

    for path in paths {
        let bytes = fs::read(path).map_err(|error| ImportError::Archive(error.to_string()))?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|error| ImportError::InvalidExport(error.to_string()))?;

        if let Some(array) = value.as_array() {
            conversations.extend(array.iter().cloned());
            continue;
        }

        if let Some(array) = value
            .as_object()
            .and_then(|object| object.get("conversations"))
            .and_then(serde_json::Value::as_array)
        {
            conversations.extend(array.iter().cloned());
            continue;
        }

        return Err(ImportError::UnsupportedSchema(format!(
            "{} is not a supported conversations JSON document",
            path.display()
        )));
    }

    serde_json::to_vec(&conversations)
        .map_err(|error| ImportError::InvalidExport(error.to_string()))
}
