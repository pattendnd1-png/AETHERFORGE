#![forbid(unsafe_code)]

use std::{
    fs,
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

use crate::{IndexError, sha256_hex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFingerprint {
    pub size_bytes: u64,
    pub modified_ns: i64,
    pub content_hash: Option<String>,
    pub index_version: u32,
    pub extractor_version: u32,
}

impl FileFingerprint {
    pub fn new(
        size_bytes: u64,
        modified_ns: i64,
        content_hash: impl Into<String>,
        index_version: u32,
        extractor_version: u32,
    ) -> Self {
        Self {
            size_bytes,
            modified_ns,
            content_hash: Some(content_hash.into()),
            index_version,
            extractor_version,
        }
    }

    pub fn metadata_only(
        size_bytes: u64,
        modified_ns: i64,
        index_version: u32,
        extractor_version: u32,
    ) -> Self {
        Self {
            size_bytes,
            modified_ns,
            content_hash: None,
            index_version,
            extractor_version,
        }
    }

    pub fn from_path(
        path: &Path,
        index_version: u32,
        extractor_version: u32,
        verify_content: bool,
    ) -> Result<Self, IndexError> {
        let metadata = fs::metadata(path).map_err(|source| IndexError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let modified = metadata.modified().map_err(|source| IndexError::Io {
            path: path.to_path_buf(),
            source,
        })?;

        let content_hash = if verify_content {
            let content = fs::read(path).map_err(|source| IndexError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            Some(sha256_hex(&content))
        } else {
            None
        };

        Ok(Self {
            size_bytes: metadata.len(),
            modified_ns: system_time_ns(modified),
            content_hash,
            index_version,
            extractor_version,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeDecision {
    Skip,
    Reindex,
    Remove,
    Unavailable,
}

pub fn decide_change(
    previous: Option<&FileFingerprint>,
    current: Option<&FileFingerprint>,
) -> ChangeDecision {
    match (previous, current) {
        (None, None) => ChangeDecision::Skip,
        (None, Some(_)) => ChangeDecision::Reindex,
        (Some(_), None) => ChangeDecision::Remove,
        (Some(previous), Some(current)) => decide_existing(previous, current),
    }
}

fn decide_existing(previous: &FileFingerprint, current: &FileFingerprint) -> ChangeDecision {
    if previous.index_version != current.index_version
        || previous.extractor_version != current.extractor_version
    {
        return ChangeDecision::Reindex;
    }

    if let (Some(previous_hash), Some(current_hash)) =
        (&previous.content_hash, &current.content_hash)
    {
        return if previous_hash == current_hash {
            ChangeDecision::Skip
        } else {
            ChangeDecision::Reindex
        };
    }

    if previous.size_bytes == current.size_bytes && previous.modified_ns == current.modified_ns {
        ChangeDecision::Skip
    } else {
        ChangeDecision::Reindex
    }
}

fn system_time_ns(time: std::time::SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration_to_i64_ns(duration),
        Err(error) => -duration_to_i64_ns(error.duration()),
    }
}

fn duration_to_i64_ns(duration: Duration) -> i64 {
    i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX)
}
