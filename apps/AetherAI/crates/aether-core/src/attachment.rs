#![forbid(unsafe_code)]

use std::path::PathBuf;

use crate::PermissionDecision;

pub type AttachmentId = uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AttachmentKind {
    File,
    Directory,
    ProjectRoot,
    Artifact,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IndexState {
    Pending,
    Indexed,
    Stale,
    Unavailable,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceRange {
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocalCitation {
    pub canonical_path: PathBuf,
    pub display_path: String,
    pub range: SourceRange,
    pub indexed_content_hash: String,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AttachmentRecord {
    pub attachment_id: AttachmentId,
    pub conversation_id: uuid::Uuid,
    pub project_id: Option<uuid::Uuid>,
    pub canonical_path: PathBuf,
    pub attachment_kind: AttachmentKind,
    pub permission_scope: PermissionDecision,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    pub index_state: IndexState,
}

impl AttachmentRecord {
    pub fn new_file(
        conversation_id: uuid::Uuid,
        project_id: Option<uuid::Uuid>,
        canonical_path: PathBuf,
        permission_scope: PermissionDecision,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            attachment_id: uuid::Uuid::new_v4(),
            conversation_id,
            project_id,
            canonical_path,
            attachment_kind: AttachmentKind::File,
            permission_scope,
            added_at: now,
            last_seen_at: now,
            index_state: IndexState::Pending,
        }
    }
}
