#![forbid(unsafe_code)]

use std::path::{Component, Path, PathBuf};

use aether_core::{AttachmentRecord, PermissionDecision, ProjectId};
use aether_index::{checkpoint_pass_line, parse_pass_checkpoint};
use aether_storage::SqliteStore;

use crate::{ExternalAttachment, ImportError, stable_external_uuid};

pub fn resolve_extracted_attachment_path(
    extracted_root: &Path,
    source: &ExternalAttachment,
) -> Result<Option<PathBuf>, ImportError> {
    let Some(relative) = source.extracted_path.as_deref() else {
        return Ok(None);
    };

    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ImportError::InvalidExport(format!(
            "attachment path escapes extracted export: {}",
            relative.display()
        )));
    }

    let root = extracted_root
        .canonicalize()
        .map_err(|error| ImportError::Archive(error.to_string()))?;
    let candidate = extracted_root.join(relative);
    if !candidate.exists() {
        return Ok(None);
    }
    let canonical = candidate
        .canonicalize()
        .map_err(|error| ImportError::Archive(error.to_string()))?;
    if !canonical.starts_with(&root) {
        return Err(ImportError::InvalidExport(format!(
            "attachment resolved outside extracted export: {}",
            canonical.display()
        )));
    }
    if !canonical.is_file() {
        return Ok(None);
    }
    Ok(Some(canonical))
}

pub fn imported_attachment_record(
    source: &ExternalAttachment,
    conversation_id: uuid::Uuid,
    project_id: Option<ProjectId>,
    extracted_root: &Path,
    permission_scope: PermissionDecision,
) -> Result<Option<AttachmentRecord>, ImportError> {
    let Some(path) = resolve_extracted_attachment_path(extracted_root, source)? else {
        return Ok(None);
    };

    let mut record =
        AttachmentRecord::new_file(conversation_id, project_id, path, permission_scope);
    record.attachment_id = stable_external_uuid(&source.key);
    Ok(Some(record))
}

pub fn register_imported_attachment(
    store: &SqliteStore,
    source: &ExternalAttachment,
    conversation_id: uuid::Uuid,
    project_id: Option<ProjectId>,
    extracted_root: &Path,
    permission_scope: PermissionDecision,
) -> Result<Option<AttachmentRecord>, ImportError> {
    let Some(record) = imported_attachment_record(
        source,
        conversation_id,
        project_id,
        extracted_root,
        permission_scope,
    )?
    else {
        return Ok(None);
    };
    store.save_attachment(&record)?;
    Ok(Some(record))
}

pub fn bridge_imported_verify_checkpoint(
    store: &SqliteStore,
    project_id: ProjectId,
    path: &Path,
    text: &str,
) -> Result<Option<(aether_core::MemoryItem, aether_core::MemoryItem)>, ImportError> {
    let Some(checkpoint) = parse_pass_checkpoint(path, text) else {
        return Ok(None);
    };
    let line = checkpoint_pass_line(text, &checkpoint.pass_endpoint).unwrap_or(1);
    let source_link = format!("{}#L{line}", path.display());

    store
        .upsert_checkpoint_memory(
            project_id,
            checkpoint.version.as_deref(),
            &checkpoint.pass_endpoint,
            &source_link,
        )
        .map(Some)
        .map_err(|error| ImportError::Storage(error.to_string()))
}
