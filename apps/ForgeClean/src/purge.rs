use crate::manifest::{CleanupBatch, CleanupEntry, FileIdentity};
use crate::package::{is_package_partial, is_package_signature, parse_package_filename};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VerificationReport {
    pub valid: usize,
    pub issues: Vec<String>,
}

impl VerificationReport {
    pub fn is_ok(&self) -> bool {
        self.issues.is_empty()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PurgeReport {
    pub deleted_files: usize,
    pub deleted_bytes: u64,
    pub skipped: Vec<String>,
}

impl PurgeReport {
    pub fn is_ok(&self) -> bool {
        self.skipped.is_empty()
    }
}

pub fn verify_batch(batch: &CleanupBatch) -> VerificationReport {
    let mut report = VerificationReport::default();
    let root = match batch.root.canonicalize() {
        Ok(root) => root,
        Err(e) => {
            report.issues.push(format!(
                "cannot canonicalize batch root {}: {e}",
                batch.root.display()
            ));
            return report;
        }
    };

    for entry in &batch.entries {
        match validate_entry(&root, entry) {
            Ok(_) => report.valid += 1,
            Err(e) => report.issues.push(e),
        }
    }
    report
}

pub fn validate_entry_against_batch(
    batch: &CleanupBatch,
    entry: &CleanupEntry,
) -> Result<PathBuf, String> {
    let root = batch.root.canonicalize().map_err(|e| {
        format!(
            "cannot canonicalize batch root {}: {e}",
            batch.root.display()
        )
    })?;
    validate_entry(&root, entry)
}

pub fn purge_batch(batch: &CleanupBatch) -> PurgeReport {
    let mut report = PurgeReport::default();
    let root = match batch.root.canonicalize() {
        Ok(root) => root,
        Err(e) => {
            report.skipped.push(format!(
                "cannot canonicalize batch root {}: {e}",
                batch.root.display()
            ));
            return report;
        }
    };

    for entry in &batch.entries {
        let candidate = match validate_entry(&root, entry) {
            Ok(candidate) => candidate,
            Err(e) => {
                report.skipped.push(e);
                continue;
            }
        };
        match fs::remove_file(&candidate) {
            Ok(()) => {
                report.deleted_files += 1;
                report.deleted_bytes = report.deleted_bytes.saturating_add(entry.bytes);
            }
            Err(e) => report
                .skipped
                .push(format!("delete refused {}: {e}", candidate.display())),
        }
    }
    report
}

fn validate_entry(root: &Path, entry: &CleanupEntry) -> Result<PathBuf, String> {
    if entry.path.file_name().is_none() {
        return Err(format!("entry has no filename: {}", entry.path.display()));
    }
    if parse_package_filename(&entry.path).is_none()
        && !is_package_partial(&entry.path)
        && !is_package_signature(&entry.path)
    {
        return Err(format!(
            "entry is not a recognized package-cache artifact: {}",
            entry.path.display()
        ));
    }

    let parent = entry
        .path
        .parent()
        .ok_or_else(|| format!("entry has no parent: {}", entry.path.display()))?;
    let parent = parent
        .canonicalize()
        .map_err(|e| format!("cannot canonicalize parent {}: {e}", parent.display()))?;
    if !parent.starts_with(root) {
        return Err(format!(
            "AETHER_GUARD=BLOCKED path escaped approved root: {}",
            entry.path.display()
        ));
    }
    let candidate = parent.join(entry.path.file_name().expect("checked above"));
    let metadata = fs::symlink_metadata(&candidate)
        .map_err(|e| format!("cannot stat approved entry {}: {e}", candidate.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED symlink entry: {}",
            candidate.display()
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED non-file entry: {}",
            candidate.display()
        ));
    }
    let current = FileIdentity::from_metadata(&metadata);
    if current != entry.identity || metadata.len() != entry.bytes {
        return Err(format!(
            "AETHER_GUARD=BLOCKED file changed since scan: {}",
            candidate.display()
        ));
    }
    Ok(candidate)
}
