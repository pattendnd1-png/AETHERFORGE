#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use aether_core::{
    AttachmentId, AttachmentKind, AttachmentRecord, ConversationId, IndexState, PermissionDecision,
    PermissionKind, ProjectId,
};
use aether_storage::models::{
    IndexChunkRecord, IndexJobRecord, IndexSymbolRecord, IndexedFileRecord,
};
use aether_tools::{ActivityEvent, ActivityKind, ActivitySink, ActivityStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ApprovedSource, AttachmentPermissionEvaluator, ChunkBudget, FileFingerprint, FileKind,
    IndexError, IndexPolicy, SymbolKind, chunk_file, detect_file_kind, enumerate_approved_sources,
    extract_rust_symbols, terms_for_chunk,
};

const INDEX_VERSION: u32 = 1;
const EXTRACTOR_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexJobSummary {
    pub seen_files: usize,
    pub indexed_files: usize,
    pub skipped_files: usize,
    pub removed_files: usize,
    pub failed_files: usize,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreshnessReport {
    pub checked_files: usize,
    pub fresh_files: usize,
    pub reindexed_files: usize,
    pub removed_files: usize,
    pub failed_files: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexActivity {
    pub attachment_id: AttachmentId,
    pub conversation_id: ConversationId,
    pub project_id: Option<ProjectId>,
    pub summary: IndexJobSummary,
    pub errors: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

pub trait IndexActivitySink {
    fn record_index(&self, event: IndexActivity) -> Result<(), IndexError>;
}

impl<T> IndexActivitySink for T
where
    T: ActivitySink,
{
    fn record_index(&self, event: IndexActivity) -> Result<(), IndexError> {
        let status = if event.summary.cancelled {
            ActivityStatus::Cancelled
        } else if event.summary.failed_files > 0 {
            ActivityStatus::Failed
        } else {
            ActivityStatus::Passed
        };

        let details_json = serde_json::to_string(&event)
            .map_err(|error| IndexError::Activity(error.to_string()))?;

        self.record(ActivityEvent {
            id: Uuid::new_v4(),
            kind: ActivityKind::Index,
            status,
            command: None,
            cwd: None,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            started_at: event.started_at,
            finished_at: Some(event.finished_at),
            details_json: Some(details_json),
        })
        .map_err(|error| IndexError::Activity(error.to_string()))
    }
}

pub trait IndexRepository {
    fn attachment_by_id(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<Option<AttachmentRecord>, IndexError>;

    fn mark_attachment_state(
        &self,
        attachment_id: AttachmentId,
        state: &IndexState,
    ) -> Result<(), IndexError>;

    fn indexed_file_by_path(&self, path: &Path) -> Result<Option<IndexedFileRecord>, IndexError>;

    fn indexed_files_for_attachment(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<Vec<IndexedFileRecord>, IndexError>;

    fn upsert_indexed_file(&self, record: &IndexedFileRecord) -> Result<(), IndexError>;

    fn replace_file_index(
        &self,
        file: &IndexedFileRecord,
        chunks: &[IndexChunkRecord],
        symbols: &[IndexSymbolRecord],
        terms: &[(String, Uuid, u32)],
    ) -> Result<(), IndexError>;

    fn remove_file_index(&self, file_id: Uuid) -> Result<(), IndexError>;

    fn save_index_job(&self, record: &IndexJobRecord) -> Result<(), IndexError>;
}

impl IndexRepository for aether_storage::SqliteStore {
    fn attachment_by_id(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<Option<AttachmentRecord>, IndexError> {
        aether_storage::SqliteStore::attachment_by_id(self, attachment_id).map_err(storage_error)
    }

    fn mark_attachment_state(
        &self,
        attachment_id: AttachmentId,
        state: &IndexState,
    ) -> Result<(), IndexError> {
        aether_storage::SqliteStore::mark_attachment_state(self, attachment_id, state)
            .map_err(storage_error)
    }

    fn indexed_file_by_path(&self, path: &Path) -> Result<Option<IndexedFileRecord>, IndexError> {
        aether_storage::SqliteStore::indexed_file_by_path(self, path).map_err(storage_error)
    }

    fn indexed_files_for_attachment(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<Vec<IndexedFileRecord>, IndexError> {
        aether_storage::SqliteStore::indexed_files_for_attachment(self, attachment_id)
            .map_err(storage_error)
    }

    fn upsert_indexed_file(&self, record: &IndexedFileRecord) -> Result<(), IndexError> {
        aether_storage::SqliteStore::upsert_indexed_file(self, record).map_err(storage_error)
    }

    fn replace_file_index(
        &self,
        file: &IndexedFileRecord,
        chunks: &[IndexChunkRecord],
        symbols: &[IndexSymbolRecord],
        terms: &[(String, Uuid, u32)],
    ) -> Result<(), IndexError> {
        aether_storage::SqliteStore::replace_file_index(self, file, chunks, symbols, terms)
            .map_err(storage_error)
    }

    fn remove_file_index(&self, file_id: Uuid) -> Result<(), IndexError> {
        aether_storage::SqliteStore::remove_file_index(self, file_id).map_err(storage_error)
    }

    fn save_index_job(&self, record: &IndexJobRecord) -> Result<(), IndexError> {
        aether_storage::SqliteStore::save_index_job(self, record).map_err(storage_error)
    }
}

pub struct IndexService<R, P, A> {
    repository: R,
    permissions: P,
    activity: A,
    policy: IndexPolicy,
}

impl<R, P, A> IndexService<R, P, A>
where
    R: IndexRepository,
    P: AttachmentPermissionEvaluator,
    A: IndexActivitySink,
{
    pub fn new(repository: R, permissions: P, activity: A) -> Self {
        Self {
            repository,
            permissions,
            activity,
            policy: IndexPolicy::default(),
        }
    }

    pub fn with_policy(mut self, policy: IndexPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub fn refresh_attachment(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<IndexJobSummary, IndexError> {
        let cancelled = AtomicBool::new(false);
        self.refresh_attachment_with_cancel(attachment_id, &cancelled)
    }

    pub fn refresh_attachment_with_cancel(
        &self,
        attachment_id: AttachmentId,
        cancelled: &AtomicBool,
    ) -> Result<IndexJobSummary, IndexError> {
        let started_at = Utc::now();
        let attachment = self
            .repository
            .attachment_by_id(attachment_id)?
            .ok_or(IndexError::MissingAttachment(attachment_id))?;

        if !attachment.canonical_path.exists() {
            self.repository
                .mark_attachment_state(attachment_id, &IndexState::Unavailable)?;
            let summary = IndexJobSummary::default();
            self.finish_job(&attachment, summary.clone(), Vec::new(), started_at)?;
            return Ok(summary);
        }

        self.require_permission(
            &attachment,
            PermissionKind::ReadAttachment,
            &attachment.canonical_path,
        )?;
        if matches!(
            attachment.attachment_kind,
            AttachmentKind::Directory | AttachmentKind::ProjectRoot
        ) {
            self.require_permission(
                &attachment,
                PermissionKind::IndexRoot,
                &attachment.canonical_path,
            )?;
        }

        let sources = self.sources_for_attachment(&attachment)?;
        let existing = self
            .repository
            .indexed_files_for_attachment(attachment_id)?;
        let mut existing_by_path = existing
            .into_iter()
            .map(|record| (record.canonical_path.clone(), record))
            .collect::<BTreeMap<_, _>>();
        let mut seen_paths = BTreeSet::new();

        let mut summary = IndexJobSummary {
            seen_files: sources.len(),
            ..IndexJobSummary::default()
        };
        let mut errors = Vec::new();

        for source in sources {
            if cancelled.load(Ordering::SeqCst) {
                summary.cancelled = true;
                break;
            }

            seen_paths.insert(source.canonical_path.clone());
            let previous = existing_by_path.remove(&source.canonical_path);

            match self.refresh_source(&attachment, &source, previous.as_ref()) {
                Ok(SourceRefresh::Indexed) => summary.indexed_files += 1,
                Ok(SourceRefresh::Skipped) => summary.skipped_files += 1,
                Err(error) => {
                    summary.failed_files += 1;
                    errors.push(format!("{}: {error}", source.canonical_path.display()));
                    if let Some(mut stale) = previous {
                        stale.state = IndexState::Stale;
                        stale.error = Some(error.to_string());
                        stale.last_indexed_at = Utc::now();
                        let _ = self.repository.upsert_indexed_file(&stale);
                    }
                }
            }
        }

        if !summary.cancelled {
            for (path, record) in existing_by_path {
                if seen_paths.contains(&path) {
                    continue;
                }
                self.repository.remove_file_index(record.id)?;
                summary.removed_files += 1;
            }
        }

        let attachment_state = if summary.cancelled || summary.failed_files > 0 {
            IndexState::Stale
        } else {
            IndexState::Indexed
        };
        self.repository
            .mark_attachment_state(attachment_id, &attachment_state)?;

        self.finish_job(&attachment, summary.clone(), errors, started_at)?;
        Ok(summary)
    }

    pub fn ensure_fresh(&self, paths: &[PathBuf]) -> Result<FreshnessReport, IndexError> {
        let mut report = FreshnessReport::default();

        for requested in paths {
            report.checked_files += 1;
            let canonical = match requested.canonicalize() {
                Ok(path) => path,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if let Some(existing) = self.repository.indexed_file_by_path(requested)? {
                        self.repository.remove_file_index(existing.id)?;
                        report.removed_files += 1;
                    }
                    continue;
                }
                Err(source) => {
                    return Err(IndexError::Io {
                        path: requested.clone(),
                        source,
                    });
                }
            };

            let Some(existing) = self.repository.indexed_file_by_path(&canonical)? else {
                report.failed_files += 1;
                continue;
            };

            let attachment = self
                .repository
                .attachment_by_id(existing.attachment_id)?
                .ok_or(IndexError::MissingAttachment(existing.attachment_id))?;
            self.require_permission(&attachment, PermissionKind::ReadAttachment, &canonical)?;

            let current =
                FileFingerprint::from_path(&canonical, INDEX_VERSION, EXTRACTOR_VERSION, true)?;
            let previous = fingerprint_from_record(&existing);

            if current.content_hash == previous.content_hash
                && current.index_version == previous.index_version
                && current.extractor_version == previous.extractor_version
            {
                report.fresh_files += 1;
                continue;
            }

            let metadata = std::fs::metadata(&canonical).map_err(|source| IndexError::Io {
                path: canonical.clone(),
                source,
            })?;
            let kind = detect_file_kind(&canonical);
            let source = ApprovedSource {
                canonical_path: canonical,
                kind,
                metadata_only: kind == FileKind::Unsupported,
                size_bytes: metadata.len(),
            };

            match self.refresh_source(&attachment, &source, Some(&existing)) {
                Ok(SourceRefresh::Indexed) => report.reindexed_files += 1,
                Ok(SourceRefresh::Skipped) => report.fresh_files += 1,
                Err(_) => report.failed_files += 1,
            }
        }

        Ok(report)
    }

    fn sources_for_attachment(
        &self,
        attachment: &AttachmentRecord,
    ) -> Result<Vec<ApprovedSource>, IndexError> {
        match attachment.attachment_kind {
            AttachmentKind::Directory | AttachmentKind::ProjectRoot => enumerate_approved_sources(
                &attachment.canonical_path,
                std::slice::from_ref(&attachment.canonical_path),
                &self.policy,
            ),
            AttachmentKind::File | AttachmentKind::Artifact => {
                let metadata = std::fs::metadata(&attachment.canonical_path).map_err(|source| {
                    IndexError::Io {
                        path: attachment.canonical_path.clone(),
                        source,
                    }
                })?;
                let kind = detect_file_kind(&attachment.canonical_path);
                Ok(vec![ApprovedSource {
                    canonical_path: attachment.canonical_path.clone(),
                    kind,
                    metadata_only: kind == FileKind::Unsupported,
                    size_bytes: metadata.len(),
                }])
            }
        }
    }

    fn refresh_source(
        &self,
        attachment: &AttachmentRecord,
        source: &ApprovedSource,
        previous: Option<&IndexedFileRecord>,
    ) -> Result<SourceRefresh, IndexError> {
        let metadata_fingerprint = FileFingerprint::from_path(
            &source.canonical_path,
            INDEX_VERSION,
            EXTRACTOR_VERSION,
            false,
        )?;

        if let Some(previous) = previous {
            let previous_fingerprint = fingerprint_from_record(previous);
            if previous_fingerprint.size_bytes == metadata_fingerprint.size_bytes
                && previous_fingerprint.modified_ns == metadata_fingerprint.modified_ns
                && previous_fingerprint.index_version == metadata_fingerprint.index_version
                && previous_fingerprint.extractor_version == metadata_fingerprint.extractor_version
            {
                return Ok(SourceRefresh::Skipped);
            }
        }

        let verified = FileFingerprint::from_path(
            &source.canonical_path,
            INDEX_VERSION,
            EXTRACTOR_VERSION,
            true,
        )?;

        if let Some(previous) = previous {
            let old = fingerprint_from_record(previous);
            if old.content_hash == verified.content_hash
                && old.index_version == verified.index_version
                && old.extractor_version == verified.extractor_version
            {
                return Ok(SourceRefresh::Skipped);
            }
        }

        let file_id = previous.map_or_else(Uuid::new_v4, |record| record.id);
        let content_hash = verified
            .content_hash
            .clone()
            .ok_or_else(|| IndexError::Index("verified hash missing".into()))?;

        let (chunks, symbols, terms) = if source.metadata_only {
            (Vec::new(), Vec::new(), Vec::new())
        } else {
            let text = std::fs::read_to_string(&source.canonical_path).map_err(|source_error| {
                IndexError::Io {
                    path: source.canonical_path.clone(),
                    source: source_error,
                }
            })?;

            let extracted = if source.kind == FileKind::Rust {
                extract_rust_symbols(file_id, &text)
            } else {
                Vec::new()
            };
            let chunks = chunk_file(
                file_id,
                source.kind,
                &text,
                &extracted,
                ChunkBudget::default(),
            );

            let chunk_records = chunks
                .iter()
                .map(|chunk| IndexChunkRecord {
                    id: chunk.id,
                    file_id,
                    ordinal: chunk.ordinal,
                    start_line: chunk.range.start_line,
                    end_line: chunk.range.end_line,
                    content: chunk.content.clone(),
                    content_hash: chunk.content_hash.clone(),
                    estimated_tokens: chunk.estimated_tokens,
                })
                .collect::<Vec<_>>();

            let symbol_records = extracted
                .iter()
                .map(|symbol| IndexSymbolRecord {
                    id: symbol.id,
                    file_id,
                    kind: symbol_kind_name(symbol.kind).to_string(),
                    name: symbol.name.clone(),
                    qualified_name: symbol.qualified_name.clone(),
                    start_line: symbol.range.start_line,
                    end_line: symbol.range.end_line,
                    parent_symbol: symbol.parent_symbol,
                })
                .collect::<Vec<_>>();

            let mut term_records = Vec::new();
            for chunk in &chunks {
                for (term, frequency) in terms_for_chunk(&chunk.content) {
                    term_records.push((term, chunk.id, frequency));
                }
            }

            (chunk_records, symbol_records, term_records)
        };

        let record = IndexedFileRecord {
            id: file_id,
            attachment_id: attachment.attachment_id,
            canonical_path: source.canonical_path.clone(),
            size_bytes: verified.size_bytes,
            modified_ns: verified.modified_ns,
            content_hash,
            index_version: verified.index_version,
            extractor_version: verified.extractor_version,
            state: IndexState::Indexed,
            last_indexed_at: Utc::now(),
            error: None,
        };

        self.repository
            .replace_file_index(&record, &chunks, &symbols, &terms)?;
        Ok(SourceRefresh::Indexed)
    }

    fn require_permission(
        &self,
        attachment: &AttachmentRecord,
        kind: PermissionKind,
        path: &Path,
    ) -> Result<(), IndexError> {
        let decision = self.permissions.decision(
            attachment.conversation_id,
            attachment.project_id,
            kind,
            path,
        );

        match decision {
            PermissionDecision::AllowThisChat
            | PermissionDecision::AllowThisProject
            | PermissionDecision::AlwaysAllow => Ok(()),
            PermissionDecision::Blocked | PermissionDecision::AskEveryTime => {
                Err(IndexError::PermissionDenied(path.to_path_buf()))
            }
        }
    }

    fn finish_job(
        &self,
        attachment: &AttachmentRecord,
        summary: IndexJobSummary,
        errors: Vec<String>,
        started_at: DateTime<Utc>,
    ) -> Result<(), IndexError> {
        let finished_at = Utc::now();
        let status = if summary.cancelled {
            "Cancelled"
        } else if summary.failed_files > 0 {
            "Failed"
        } else {
            "Passed"
        };

        let payload_json = serde_json::to_string(&serde_json::json!({
            "summary": summary,
            "errors": errors,
        }))
        .map_err(|error| IndexError::Index(error.to_string()))?;

        self.repository.save_index_job(&IndexJobRecord {
            id: Uuid::new_v4(),
            conversation_id: attachment.conversation_id,
            project_id: attachment.project_id,
            status: status.to_string(),
            started_at,
            finished_at: Some(finished_at),
            payload_json,
        })?;

        self.activity.record_index(IndexActivity {
            attachment_id: attachment.attachment_id,
            conversation_id: attachment.conversation_id,
            project_id: attachment.project_id,
            summary,
            errors,
            started_at,
            finished_at,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceRefresh {
    Indexed,
    Skipped,
}

fn fingerprint_from_record(record: &IndexedFileRecord) -> FileFingerprint {
    FileFingerprint {
        size_bytes: record.size_bytes,
        modified_ns: record.modified_ns,
        content_hash: Some(record.content_hash.clone()),
        index_version: record.index_version,
        extractor_version: record.extractor_version,
    }
}

fn storage_error(error: aether_storage::StorageError) -> IndexError {
    IndexError::Storage(error.to_string())
}

fn symbol_kind_name(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Module => "Module",
        SymbolKind::Struct => "Struct",
        SymbolKind::Enum => "Enum",
        SymbolKind::Trait => "Trait",
        SymbolKind::Impl => "Impl",
        SymbolKind::Function => "Function",
        SymbolKind::Method => "Method",
        SymbolKind::Const => "Const",
        SymbolKind::Static => "Static",
        SymbolKind::Test => "Test",
        SymbolKind::Macro => "Macro",
    }
}
