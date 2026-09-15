#![forbid(unsafe_code)]

pub mod archive;
pub mod bridge;
pub mod chatgpt;
pub mod parser;
pub mod reconstruct;
pub mod service;
pub use chatgpt::{
    ChatGptExportManifest, ExternalAttachment, ExternalConversation, ExternalMessage,
    ExternalProject, ExternalSourceKey, stable_external_uuid,
};

pub const CHATGPT_IMPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("unsupported ChatGPT export schema: {0}")]
    UnsupportedSchema(String),
    #[error("invalid ChatGPT export: {0}")]
    InvalidExport(String),
    #[error("archive error: {0}")]
    Archive(String),
    #[error("storage error: {0}")]
    Storage(String),
}

impl From<aether_system::SystemError> for ImportError {
    fn from(value: aether_system::SystemError) -> Self {
        Self::Archive(value.to_string())
    }
}

pub use parser::{ParsedChatGptExport, parse_chatgpt_conversations_json};

pub use reconstruct::{ReconstructedChatGptImport, reconstruct_chatgpt_export};

impl From<aether_storage::StorageError> for ImportError {
    fn from(value: aether_storage::StorageError) -> Self {
        Self::Storage(value.to_string())
    }
}

pub use bridge::{
    bridge_imported_verify_checkpoint, imported_attachment_record, register_imported_attachment,
    resolve_extracted_attachment_path,
};

pub use service::{ChatGptImportSummary, import_chatgpt_export_all, import_chatgpt_extracted_all};
