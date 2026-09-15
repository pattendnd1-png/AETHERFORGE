#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap};

use aether_core::{
    Conversation, ConversationKind, ConversationSettings, Message, Role, WorkspaceState,
};
use aether_storage::models::{
    ExternalRecord, ExternalRecordKind, ExternalSourceType, ImportExportRecord, ProjectRecord,
};
use chrono::Utc;

use crate::{
    ExternalMessage, ExternalSourceKey, ImportError, ParsedChatGptExport, stable_external_uuid,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ReconstructedChatGptImport {
    pub export: ImportExportRecord,
    pub projects: Vec<ProjectRecord>,
    pub conversations: Vec<Conversation>,
    pub external_records: Vec<ExternalRecord>,
}

pub fn reconstruct_chatgpt_export(
    parsed: ParsedChatGptExport,
) -> Result<ReconstructedChatGptImport, ImportError> {
    let fingerprint = parsed.manifest.fingerprint.clone();
    let imported_at = parsed.manifest.imported_at;

    let export = ImportExportRecord {
        fingerprint: fingerprint.clone(),
        source_type: ExternalSourceType::ChatGptExport,
        imported_at,
        payload_json: serde_json::to_string(&parsed.manifest)
            .map_err(|error| ImportError::InvalidExport(error.to_string()))?,
    };

    let mut project_ids = BTreeMap::new();
    let mut project_names = BTreeMap::new();
    let mut projects = Vec::new();
    let mut external_records = Vec::new();

    for project in &parsed.projects {
        let id = stable_external_uuid(&project.key);
        project_ids.insert(project.key.external_id.clone(), id);
        project_names.insert(project.key.external_id.clone(), project.name.clone());

        projects.push(ProjectRecord {
            id,
            name: project.name.clone(),
            root: format!("external://chatgpt/project/{}", project.key.external_id),
            provenance: "ChatGPTExport".into(),
        });

        external_records.push(external_record(
            id,
            ExternalRecordKind::Project,
            &project.key.external_id,
            &fingerprint,
            project.source_timestamp,
            imported_at,
            project,
        )?);
    }

    let mut messages_by_conversation = HashMap::<String, Vec<&ExternalMessage>>::new();
    for message in &parsed.messages {
        messages_by_conversation
            .entry(message.conversation_external_id.clone())
            .or_default()
            .push(message);
    }

    for attachment in &parsed.attachments {
        external_records.push(external_record(
            stable_external_uuid(&attachment.key),
            ExternalRecordKind::Attachment,
            &attachment.key.external_id,
            &fingerprint,
            None,
            imported_at,
            attachment,
        )?);
    }

    let mut conversations = Vec::new();
    for source in &parsed.conversations {
        let id = stable_external_uuid(&source.key);
        let created_at = source.created_at.unwrap_or(imported_at);
        let updated_at = source.updated_at.unwrap_or(created_at);

        let workspace = source.project_external_id.as_ref().and_then(|external_id| {
            let project_id = project_ids.get(external_id).copied()?;
            let project_name = project_names
                .get(external_id)
                .cloned()
                .unwrap_or_else(|| "Imported ChatGPT Project".into());
            Some(WorkspaceState {
                id: stable_workspace_uuid(&source.key),
                name: project_name,
                project_id: Some(project_id),
                roots: Vec::new(),
                root: None,
            })
        });

        let mut messages = Vec::new();
        if let Some(source_messages) = messages_by_conversation.get(&source.key.external_id) {
            for source_message in source_messages {
                if let Some(role) = aether_role(&source_message.role) {
                    messages.push(Message {
                        id: stable_external_uuid(&source_message.key),
                        role,
                        content: source_message.content.clone(),
                        created_at: source_message.created_at.unwrap_or(created_at),
                    });
                }

                external_records.push(external_record(
                    stable_external_uuid(&source_message.key),
                    ExternalRecordKind::Message,
                    &source_message.key.external_id,
                    &fingerprint,
                    source_message.created_at,
                    imported_at,
                    *source_message,
                )?);
            }
        }

        let conversation = Conversation {
            id,
            title: source.title.clone(),
            kind: ConversationKind::Chat,
            pinned: false,
            archived: false,
            created_at,
            updated_at,
            settings: ConversationSettings::default(),
            workspace,
            messages,
        };

        external_records.push(external_record(
            id,
            ExternalRecordKind::Conversation,
            &source.key.external_id,
            &fingerprint,
            source.updated_at.or(source.created_at),
            imported_at,
            source,
        )?);

        conversations.push(conversation);
    }

    Ok(ReconstructedChatGptImport {
        export,
        projects,
        conversations,
        external_records,
    })
}

fn aether_role(role: &str) -> Option<Role> {
    match role {
        "system" => Some(Role::System),
        "developer" => Some(Role::Developer),
        "user" => Some(Role::User),
        "assistant" => Some(Role::Assistant),
        "tool" => Some(Role::Tool),
        _ => None,
    }
}

fn stable_workspace_uuid(conversation_key: &ExternalSourceKey) -> uuid::Uuid {
    let workspace_key = ExternalSourceKey::new(
        conversation_key.source_type,
        ExternalRecordKind::Conversation,
        format!("workspace:{}", conversation_key.external_id),
    );
    stable_external_uuid(&workspace_key)
}

fn external_record<T: serde::Serialize>(
    aether_id: uuid::Uuid,
    record_kind: ExternalRecordKind,
    external_id: &str,
    export_fingerprint: &str,
    source_timestamp: Option<chrono::DateTime<Utc>>,
    updated_at: chrono::DateTime<Utc>,
    payload: &T,
) -> Result<ExternalRecord, ImportError> {
    Ok(ExternalRecord {
        aether_id,
        source_type: ExternalSourceType::ChatGptExport,
        record_kind,
        external_id: external_id.into(),
        export_fingerprint: export_fingerprint.into(),
        source_timestamp,
        updated_at,
        payload_json: serde_json::to_string(payload)
            .map_err(|error| ImportError::InvalidExport(error.to_string()))?,
    })
}
