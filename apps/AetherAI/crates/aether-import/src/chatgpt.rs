#![forbid(unsafe_code)]

use std::path::PathBuf;

use aether_index::sha256_bytes;
use aether_storage::models::{ExternalRecordKind, ExternalSourceType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExternalSourceKey {
    pub source_type: ExternalSourceType,
    pub record_kind: ExternalRecordKind,
    pub external_id: String,
}

impl ExternalSourceKey {
    pub fn new(
        source_type: ExternalSourceType,
        record_kind: ExternalRecordKind,
        external_id: impl Into<String>,
    ) -> Self {
        Self {
            source_type,
            record_kind,
            external_id: external_id.into(),
        }
    }

    pub fn chatgpt(record_kind: ExternalRecordKind, external_id: impl Into<String>) -> Self {
        Self::new(ExternalSourceType::ChatGptExport, record_kind, external_id)
    }
}

pub fn stable_external_uuid(key: &ExternalSourceKey) -> Uuid {
    let material = format!(
        "AetherAI\0{}\0{}\0{}",
        key.source_type.storage_key(),
        key.record_kind.storage_key(),
        key.external_id
    );
    let digest = sha256_bytes(material.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);

    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatGptExportManifest {
    pub fingerprint: String,
    pub imported_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalProject {
    pub key: ExternalSourceKey,
    pub name: String,
    pub instructions: Option<String>,
    pub source_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalConversation {
    pub key: ExternalSourceKey,
    pub project_external_id: Option<String>,
    pub title: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalMessage {
    pub key: ExternalSourceKey,
    pub conversation_external_id: String,
    pub parent_external_id: Option<String>,
    pub role: String,
    pub content: String,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalAttachment {
    pub key: ExternalSourceKey,
    pub conversation_external_id: String,
    pub file_name: String,
    pub extracted_path: Option<PathBuf>,
}
