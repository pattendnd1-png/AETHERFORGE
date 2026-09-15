use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

use darkstone_assets::{ArchiveFingerprint, AssetStatus, CompatibilityReport, SupportState};
use darkstone_mtf::{Limits, MtfArchive, MtfError};
use sha2::{Digest, Sha256};
use thiserror::Error;

const ARCHIVES: &[(&str, bool)] = &[
    ("DATA.MTF", true),
    ("MUSIC.MTF", false),
    ("VOICES1.MTF", false),
];

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("install path is not a directory: {path}")]
    InvalidInstallPath { path: PathBuf },
    #[error("required archive is missing: {path}")]
    MissingRequiredArchive { path: PathBuf },
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("required archive {path} could not be parsed safely: {source}")]
    RequiredArchiveParse {
        path: PathBuf,
        #[source]
        source: MtfError,
    },
}

pub fn scan_install(path: &Path) -> Result<CompatibilityReport, ScanError> {
    if !path.is_dir() {
        return Err(ScanError::InvalidInstallPath {
            path: path.to_path_buf(),
        });
    }

    let required = path.join("DATA.MTF");
    if !required.is_file() {
        return Err(ScanError::MissingRequiredArchive { path: required });
    }

    let mut report = CompatibilityReport::new(path.to_path_buf(), Vec::new());

    for &(archive_name, required) in ARCHIVES {
        let archive_path = path.join(archive_name);
        if !archive_path.is_file() {
            continue;
        }

        let (bytes, sha256) = read_and_hash(&archive_path)?;
        let parsed = MtfArchive::parse(Arc::<[u8]>::from(bytes), Limits::default());
        let archive = match parsed {
            Ok(archive) => archive,
            Err(source) if required => {
                return Err(ScanError::RequiredArchiveParse {
                    path: archive_path,
                    source,
                });
            }
            Err(source) => {
                let mut status =
                    AssetStatus::new(archive_name, "<archive>", 0, SupportState::Error);
                status.diagnostic = Some(source.to_string());
                report.assets.push(status);
                report.summary.errors += 1;
                continue;
            }
        };

        report.archives.push(ArchiveFingerprint {
            name: archive_name.to_owned(),
            sha256,
        });
        report.summary.archives_parsed += 1;
        report.summary.entries_total += archive.entries().len() as u64;

        for entry in archive.entries() {
            report.assets.push(AssetStatus::new(
                archive_name,
                entry.name.clone(),
                u64::from(entry.uncompressed_size),
                SupportState::Unknown,
            ));
        }
    }

    Ok(report)
}

fn read_and_hash(path: &Path) -> Result<(Vec<u8>, String), ScanError> {
    let mut file = File::open(path).map_err(|source| ScanError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let capacity = file
        .metadata()
        .ok()
        .and_then(|meta| usize::try_from(meta.len()).ok())
        .unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];

    loop {
        let count = file.read(&mut buf).map_err(|source| ScanError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if count == 0 {
            break;
        }
        hasher.update(&buf[..count]);
        bytes.extend_from_slice(&buf[..count]);
    }

    Ok((bytes, format!("{:x}", hasher.finalize())))
}
