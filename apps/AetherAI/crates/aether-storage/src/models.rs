use aether_core::{ConversationId, PermissionDecision, PermissionKind, ProjectId};
use aether_model_api::ModelDescriptor;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiState {
    pub left_open: bool,
    pub left_width: f32,
    pub processing_open: bool,
    pub processing_width: f32,
    pub ui_scale: f32,
    #[serde(default)]
    pub first_run_complete: bool,
    pub last_conversation: Option<ConversationId>,
}
impl Default for UiState {
    fn default() -> Self {
        Self {
            left_open: true,
            left_width: 270.0,
            processing_open: true,
            processing_width: 330.0,
            ui_scale: 1.0,
            first_run_complete: false,
            last_conversation: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: ProjectId,
    pub name: String,
    pub root: String,
    pub provenance: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRecord {
    pub descriptor: ModelDescriptor,
    pub registered_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionRecord {
    pub scope_key: String,
    pub kind: PermissionKind,
    pub decision: PermissionDecision,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityRecord {
    pub id: Uuid,
    pub conversation_id: Option<ConversationId>,
    pub project_id: Option<ProjectId>,
    pub started_at: DateTime<Utc>,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedFileRecord {
    pub id: Uuid,
    pub attachment_id: Uuid,
    pub canonical_path: std::path::PathBuf,
    pub size_bytes: u64,
    pub modified_ns: i64,
    pub content_hash: String,
    pub index_version: u32,
    pub extractor_version: u32,
    pub state: aether_core::IndexState,
    pub last_indexed_at: DateTime<Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexChunkRecord {
    pub id: Uuid,
    pub file_id: Uuid,
    pub ordinal: u32,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub content_hash: String,
    pub estimated_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexSymbolRecord {
    pub id: Uuid,
    pub file_id: Uuid,
    pub kind: String,
    pub name: String,
    pub qualified_name: Option<String>,
    pub start_line: u32,
    pub end_line: u32,
    pub parent_symbol: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexJobRecord {
    pub id: Uuid,
    pub conversation_id: ConversationId,
    pub project_id: Option<ProjectId>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalHistoryRecord {
    pub id: Uuid,
    pub conversation_id: ConversationId,
    pub project_id: Option<ProjectId>,
    pub created_at: DateTime<Utc>,
    pub payload_json: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VectorDecodeError {
    #[error("embedding vector byte length {0} is not divisible by four")]
    InvalidByteLength(usize),
    #[error("embedding vector dimensions mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
}

pub fn vector_to_le_bytes(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(vector));
    for value in vector {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub fn vector_from_le_bytes(blob: &[u8], dimensions: usize) -> Result<Vec<f32>, VectorDecodeError> {
    let chunks = blob.chunks_exact(std::mem::size_of::<f32>());
    if !chunks.remainder().is_empty() {
        return Err(VectorDecodeError::InvalidByteLength(blob.len()));
    }

    let values = chunks
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect::<Vec<_>>();

    if values.len() != dimensions {
        return Err(VectorDecodeError::DimensionMismatch {
            expected: dimensions,
            actual: values.len(),
        });
    }

    Ok(values)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExternalSourceType {
    ChatGptExport,
}

impl ExternalSourceType {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::ChatGptExport => "ChatGPTExport",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExternalRecordKind {
    Project,
    Conversation,
    Message,
    Attachment,
}

impl ExternalRecordKind {
    pub const fn storage_key(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Conversation => "Conversation",
            Self::Message => "Message",
            Self::Attachment => "Attachment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportExportRecord {
    pub fingerprint: String,
    pub source_type: ExternalSourceType,
    pub imported_at: DateTime<Utc>,
    pub payload_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalRecord {
    pub aether_id: Uuid,
    pub source_type: ExternalSourceType,
    pub record_kind: ExternalRecordKind,
    pub external_id: String,
    pub export_fingerprint: String,
    pub source_timestamp: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub payload_json: String,
}
