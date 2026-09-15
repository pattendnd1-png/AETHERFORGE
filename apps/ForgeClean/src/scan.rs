use crate::manifest::{CleanupBatch, CleanupCategory, CleanupEntry, FileIdentity};
use crate::package::{
    VersionComparator, is_package_partial, package_signature_archive_path, parse_package_filename,
    select_superseded, signature_path_for_archive,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct ScanOptions {
    pub cache_dir: PathBuf,
    pub keep_versions: usize,
    pub partial_age_days: u64,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from("/var/cache/pacman/pkg"),
            keep_versions: 2,
            partial_age_days: 7,
        }
    }
}

pub fn scan_cache(
    options: &ScanOptions,
    comparator: &dyn VersionComparator,
) -> Result<CleanupBatch, String> {
    if options.keep_versions == 0 {
        return Err("keep_versions must be at least 1".to_owned());
    }
    let root = options
        .cache_dir
        .canonicalize()
        .map_err(|e| format!("cannot canonicalize {}: {e}", options.cache_dir.display()))?;
    if !root.is_dir() {
        return Err(format!("cache root is not a directory: {}", root.display()));
    }

    let mut archives = Vec::new();
    let mut partial_entries = Vec::new();
    let stale_for = Duration::from_secs(options.partial_age_days.saturating_mul(86_400));
    let now = SystemTime::now();

    let read = fs::read_dir(&root)
        .map_err(|e| format!("cannot read cache root {}: {e}", root.display()))?;
    for dir_entry in read {
        let dir_entry = dir_entry.map_err(|e| format!("cannot read cache entry: {e}"))?;
        let path = dir_entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| format!("cannot stat {}: {e}", path.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }

        if is_package_partial(&path) {
            let modified = metadata
                .modified()
                .map_err(|e| format!("cannot read mtime {}: {e}", path.display()))?;
            let age = now.duration_since(modified).unwrap_or_default();
            if age >= stale_for {
                partial_entries.push(CleanupEntry {
                    path: root.join(dir_entry.file_name()),
                    bytes: metadata.len(),
                    category: CleanupCategory::PartialDownload,
                    reason: format!(
                        "package partial older than {} day(s)",
                        options.partial_age_days
                    ),
                    identity: FileIdentity::from_metadata(&metadata),
                });
            }
            continue;
        }

        if let Some(mut archive) = parse_package_filename(&path) {
            archive.path = root.join(dir_entry.file_name());
            archives.push(archive);
        }
    }

    let superseded = select_superseded(archives, options.keep_versions, comparator)?;
    let mut entries = partial_entries;
    for archive in superseded {
        let metadata = fs::symlink_metadata(&archive.path)
            .map_err(|e| format!("cannot restat {}: {e}", archive.path.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        let signature_path = signature_path_for_archive(&archive.path);
        entries.push(CleanupEntry {
            path: archive.path.clone(),
            bytes: metadata.len(),
            category: CleanupCategory::SupersededPackage,
            reason: format!(
                "superseded {} {} package archive (keeping newest {})",
                archive.name, archive.version, options.keep_versions
            ),
            identity: FileIdentity::from_metadata(&metadata),
        });

        match fs::symlink_metadata(&signature_path) {
            Ok(signature_metadata)
                if signature_metadata.is_file() && !signature_metadata.file_type().is_symlink() =>
            {
                entries.push(CleanupEntry {
                    path: signature_path,
                    bytes: signature_metadata.len(),
                    category: CleanupCategory::PackageSignature,
                    reason: format!(
                        "signature sidecar for superseded {} {} package archive",
                        archive.name, archive.version
                    ),
                    identity: FileIdentity::from_metadata(&signature_metadata),
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "cannot stat package signature {}: {error}",
                    signature_path.display()
                ));
            }
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(CleanupBatch {
        root,
        keep_versions: options.keep_versions,
        partial_age_days: options.partial_age_days,
        entries,
    })
}

pub fn protect_installed_versions(
    batch: &mut CleanupBatch,
    installed: &BTreeMap<String, String>,
) -> usize {
    let mut pinned_versions = BTreeSet::new();
    batch.entries.retain(|entry| {
        let archive = match entry.category {
            CleanupCategory::SupersededPackage => parse_package_filename(&entry.path),
            CleanupCategory::PackageSignature => package_signature_archive_path(&entry.path)
                .and_then(|path| parse_package_filename(&path)),
            CleanupCategory::PartialDownload => None,
        };
        let Some(archive) = archive else {
            return true;
        };
        if installed.get(&archive.name) == Some(&archive.version) {
            pinned_versions.insert((archive.name, archive.version));
            false
        } else {
            true
        }
    });
    pinned_versions.len()
}
