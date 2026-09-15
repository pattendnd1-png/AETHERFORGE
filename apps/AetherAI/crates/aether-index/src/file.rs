#![forbid(unsafe_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use aether_core::{
    AttachmentKind, AttachmentRecord, ConversationId, IndexState, PermissionDecision,
    PermissionKind, ProjectId,
};
use chrono::Utc;

use crate::{IndexError, ignore::should_exclude};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Rust,
    Markdown,
    Text,
    Json,
    Yaml,
    Toml,
    Shell,
    Python,
    JavaScript,
    TypeScript,
    C,
    Cpp,
    BuildLog,
    VerificationText,
    Manifest,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedSource {
    pub canonical_path: PathBuf,
    pub kind: FileKind,
    pub metadata_only: bool,
    pub size_bytes: u64,
}

impl ApprovedSource {
    pub fn fingerprint(
        &self,
        index_version: u32,
        extractor_version: u32,
        verify_content: bool,
    ) -> Result<crate::FileFingerprint, IndexError> {
        crate::FileFingerprint::from_path(
            &self.canonical_path,
            index_version,
            extractor_version,
            verify_content,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexPolicy {
    pub max_depth: usize,
    pub follow_symlinks: bool,
    pub user_ignores: Vec<String>,
    pub allow_high_risk_names: bool,
}

impl Default for IndexPolicy {
    fn default() -> Self {
        Self {
            max_depth: 32,
            follow_symlinks: false,
            user_ignores: Vec::new(),
            allow_high_risk_names: false,
        }
    }
}

pub trait AttachmentRepository {
    fn save_attachment(&self, record: &AttachmentRecord) -> Result<(), IndexError>;
}

impl AttachmentRepository for aether_storage::SqliteStore {
    fn save_attachment(&self, record: &AttachmentRecord) -> Result<(), IndexError> {
        aether_storage::SqliteStore::save_attachment(self, record)
            .map_err(|error| IndexError::Storage(error.to_string()))
    }
}

pub trait AttachmentPermissionEvaluator {
    fn decision(
        &self,
        conversation_id: ConversationId,
        project_id: Option<ProjectId>,
        kind: PermissionKind,
        path: &Path,
    ) -> PermissionDecision;
}

impl<F> AttachmentPermissionEvaluator for F
where
    F: Fn(ConversationId, Option<ProjectId>, PermissionKind, &Path) -> PermissionDecision,
{
    fn decision(
        &self,
        conversation_id: ConversationId,
        project_id: Option<ProjectId>,
        kind: PermissionKind,
        path: &Path,
    ) -> PermissionDecision {
        self(conversation_id, project_id, kind, path)
    }
}

pub struct AttachmentRegistry<R, P> {
    repository: R,
    permissions: P,
}

impl<R, P> AttachmentRegistry<R, P>
where
    R: AttachmentRepository,
    P: AttachmentPermissionEvaluator,
{
    pub fn new(repository: R, permissions: P) -> Self {
        Self {
            repository,
            permissions,
        }
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }

    pub fn attach_file(
        &self,
        conversation_id: ConversationId,
        project_id: Option<ProjectId>,
        path: &Path,
    ) -> Result<AttachmentRecord, IndexError> {
        let canonical_path = canonical_existing(path)?;
        if !canonical_path.is_file() {
            return Err(IndexError::WrongAttachmentKind(canonical_path));
        }

        let read = self.permissions.decision(
            conversation_id,
            project_id,
            PermissionKind::ReadAttachment,
            &canonical_path,
        );
        require_allowed(read, &canonical_path)?;

        let record = AttachmentRecord::new_file(conversation_id, project_id, canonical_path, read);
        self.repository.save_attachment(&record)?;
        Ok(record)
    }

    pub fn attach_root(
        &self,
        conversation_id: ConversationId,
        project_id: Option<ProjectId>,
        path: &Path,
    ) -> Result<AttachmentRecord, IndexError> {
        let canonical_path = canonical_existing(path)?;
        if !canonical_path.is_dir() {
            return Err(IndexError::WrongAttachmentKind(canonical_path));
        }

        let read = self.permissions.decision(
            conversation_id,
            project_id,
            PermissionKind::ReadAttachment,
            &canonical_path,
        );
        require_allowed(read, &canonical_path)?;

        let index = self.permissions.decision(
            conversation_id,
            project_id,
            PermissionKind::IndexRoot,
            &canonical_path,
        );
        require_allowed(index, &canonical_path)?;

        let now = Utc::now();
        let record = AttachmentRecord {
            attachment_id: uuid::Uuid::new_v4(),
            conversation_id,
            project_id,
            canonical_path,
            attachment_kind: AttachmentKind::ProjectRoot,
            permission_scope: index,
            added_at: now,
            last_seen_at: now,
            index_state: IndexState::Pending,
        };
        self.repository.save_attachment(&record)?;
        Ok(record)
    }

    pub fn refresh_attachment_state(
        &self,
        record: &mut AttachmentRecord,
    ) -> Result<(), IndexError> {
        record.last_seen_at = Utc::now();
        match fs::symlink_metadata(&record.canonical_path) {
            Ok(_) => {
                if record.index_state == IndexState::Unavailable {
                    record.index_state = IndexState::Stale;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                record.index_state = IndexState::Unavailable;
            }
            Err(error) => {
                return Err(IndexError::Io {
                    path: record.canonical_path.clone(),
                    source: error,
                });
            }
        }
        self.repository.save_attachment(record)
    }
}

pub fn enumerate_approved_sources(
    root: &Path,
    allowed_roots: &[PathBuf],
    policy: &IndexPolicy,
) -> Result<Vec<ApprovedSource>, IndexError> {
    let canonical_root = canonical_existing(root)?;
    if !canonical_root.is_dir() {
        return Err(IndexError::WrongAttachmentKind(canonical_root));
    }

    let canonical_allowed = allowed_roots
        .iter()
        .filter_map(|path| path.canonicalize().ok())
        .collect::<Vec<_>>();

    if canonical_allowed.is_empty()
        || !canonical_allowed
            .iter()
            .any(|allowed| canonical_root.starts_with(allowed))
    {
        return Err(IndexError::PermissionDenied(canonical_root));
    }

    let mut output = Vec::new();
    walk(
        &canonical_root,
        &canonical_root,
        &canonical_allowed,
        policy,
        0,
        &mut output,
    )?;
    output.sort_by(|left, right| left.canonical_path.cmp(&right.canonical_path));
    Ok(output)
}

fn walk(
    root: &Path,
    current: &Path,
    allowed_roots: &[PathBuf],
    policy: &IndexPolicy,
    depth: usize,
    output: &mut Vec<ApprovedSource>,
) -> Result<(), IndexError> {
    if depth > policy.max_depth {
        return Ok(());
    }

    let mut entries = fs::read_dir(current)
        .map_err(|source| IndexError::Io {
            path: current.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| IndexError::Io {
            path: current.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        if should_exclude(&path, root, policy) {
            continue;
        }

        let metadata = fs::symlink_metadata(&path).map_err(|source| IndexError::Io {
            path: path.clone(),
            source,
        })?;

        if metadata.file_type().is_symlink() {
            if !policy.follow_symlinks {
                continue;
            }
            let resolved = match path.canonicalize() {
                Ok(value) => value,
                Err(_) => continue,
            };
            if !allowed_roots
                .iter()
                .any(|allowed| resolved.starts_with(allowed))
            {
                continue;
            }
            if resolved.is_dir() {
                walk(root, &resolved, allowed_roots, policy, depth + 1, output)?;
            } else if resolved.is_file() {
                push_source(resolved, output)?;
            }
            continue;
        }

        if metadata.is_dir() {
            walk(root, &path, allowed_roots, policy, depth + 1, output)?;
        } else if metadata.is_file() {
            let canonical = path.canonicalize().map_err(|source| IndexError::Io {
                path: path.clone(),
                source,
            })?;
            if allowed_roots
                .iter()
                .any(|allowed| canonical.starts_with(allowed))
            {
                push_source(canonical, output)?;
            }
        }
    }

    Ok(())
}

fn push_source(
    canonical_path: PathBuf,
    output: &mut Vec<ApprovedSource>,
) -> Result<(), IndexError> {
    let metadata = fs::metadata(&canonical_path).map_err(|source| IndexError::Io {
        path: canonical_path.clone(),
        source,
    })?;
    let kind = detect_file_kind(&canonical_path);
    output.push(ApprovedSource {
        canonical_path,
        kind,
        metadata_only: kind == FileKind::Unsupported,
        size_bytes: metadata.len(),
    });
    Ok(())
}

pub fn detect_file_kind(path: &Path) -> FileKind {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let lower_name = name.to_ascii_lowercase();

    if lower_name.contains("verify") && lower_name.ends_with(".txt") {
        return FileKind::VerificationText;
    }

    if matches!(
        lower_name.as_str(),
        "cargo.toml"
            | "package.json"
            | "pyproject.toml"
            | "requirements.txt"
            | "cmakelists.txt"
            | "meson.build"
    ) {
        return FileKind::Manifest;
    }

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "rs" => FileKind::Rust,
        "md" | "markdown" => FileKind::Markdown,
        "txt" => FileKind::Text,
        "json" => FileKind::Json,
        "yaml" | "yml" => FileKind::Yaml,
        "toml" => FileKind::Toml,
        "sh" | "bash" | "zsh" | "fish" => FileKind::Shell,
        "py" => FileKind::Python,
        "js" | "jsx" | "mjs" | "cjs" => FileKind::JavaScript,
        "ts" | "tsx" => FileKind::TypeScript,
        "c" | "h" => FileKind::C,
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => FileKind::Cpp,
        "log" => FileKind::BuildLog,
        _ => FileKind::Unsupported,
    }
}

fn canonical_existing(path: &Path) -> Result<PathBuf, IndexError> {
    path.canonicalize().map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            IndexError::MissingPath(path.to_path_buf())
        } else {
            IndexError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
    })
}

fn require_allowed(decision: PermissionDecision, path: &Path) -> Result<(), IndexError> {
    match decision {
        PermissionDecision::AllowThisChat
        | PermissionDecision::AllowThisProject
        | PermissionDecision::AlwaysAllow => Ok(()),
        PermissionDecision::Blocked | PermissionDecision::AskEveryTime => {
            Err(IndexError::PermissionDenied(path.to_path_buf()))
        }
    }
}
