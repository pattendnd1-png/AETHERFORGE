#![forbid(unsafe_code)]

pub mod chunk;
pub mod file;
pub mod hash;
mod ignore;
pub mod incremental;
pub mod pass_artifact;
pub mod rust_symbols;
pub mod service;
pub mod symbol;

pub use hash::{sha256_bytes, sha256_hex};
pub use incremental::{ChangeDecision, FileFingerprint, decide_change};
pub use pass_artifact::{PassCheckpoint, checkpoint_pass_line, parse_pass_checkpoint};

pub use file::{
    ApprovedSource, AttachmentPermissionEvaluator, AttachmentRegistry, AttachmentRepository,
    FileKind, IndexPolicy, detect_file_kind, enumerate_approved_sources,
};

pub use chunk::{ChunkBudget, IndexChunk, chunk_file, terms_for_chunk};
pub use rust_symbols::extract_rust_symbols;
pub use service::{
    FreshnessReport, IndexActivity, IndexActivitySink, IndexJobSummary, IndexRepository,
    IndexService,
};
pub use symbol::{IndexSymbol, SymbolKind};

use std::path::PathBuf;
use thiserror::Error;

pub const INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("permission denied for {0}")]
    PermissionDenied(PathBuf),
    #[error("attachment path does not exist: {0}")]
    MissingPath(PathBuf),
    #[error("wrong attachment kind for {0}")]
    WrongAttachmentKind(PathBuf),
    #[error("filesystem error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("attachment repository error: {0}")]
    Storage(String),
    #[error("attachment not found: {0}")]
    MissingAttachment(uuid::Uuid),
    #[error("index service error: {0}")]
    Index(String),
    #[error("activity error: {0}")]
    Activity(String),
}
