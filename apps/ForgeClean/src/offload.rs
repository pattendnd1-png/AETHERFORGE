use crate::manifest::{CleanupBatch, CleanupCategory};
use crate::purge::{validate_entry_against_batch, verify_batch};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OffloadRecord {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OffloadReport {
    pub offloaded_files: usize,
    pub offloaded_bytes: u64,
    pub records: Vec<OffloadRecord>,
    pub failures: Vec<String>,
}

impl OffloadReport {
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

pub fn partition_external_batch(batch: &CleanupBatch) -> (CleanupBatch, CleanupBatch) {
    let mut offload = batch.clone();
    offload.entries.retain(|entry| {
        matches!(
            entry.category,
            CleanupCategory::SupersededPackage | CleanupCategory::PackageSignature
        )
    });
    let mut cleanup = batch.clone();
    cleanup
        .entries
        .retain(|entry| matches!(entry.category, CleanupCategory::PartialDownload));
    (offload, cleanup)
}

pub fn prepare_destination_under_mount(mount_point: &Path) -> Result<PathBuf, String> {
    let mount_metadata = fs::symlink_metadata(mount_point).map_err(|e| {
        format!(
            "cannot inspect external mount {}: {e}",
            mount_point.display()
        )
    })?;
    if mount_metadata.file_type().is_symlink() || !mount_metadata.is_dir() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED external mount is not a real directory: {}",
            mount_point.display()
        ));
    }
    let canonical_mount = mount_point.canonicalize().map_err(|e| {
        format!(
            "cannot canonicalize external mount {}: {e}",
            mount_point.display()
        )
    })?;
    let mut current = canonical_mount.clone();
    for component in ["AetherForge", "ForgeClean", "Pacman"] {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(format!(
                        "AETHER_GUARD=BLOCKED offload path component is not a real directory: {}",
                        current.display()
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|e| {
                    format!("cannot create offload directory {}: {e}", current.display())
                })?;
            }
            Err(error) => {
                return Err(format!(
                    "cannot inspect offload path component {}: {error}",
                    current.display()
                ));
            }
        }
    }
    let canonical_destination = current.canonicalize().map_err(|e| {
        format!(
            "cannot canonicalize offload destination {}: {e}",
            current.display()
        )
    })?;
    if !canonical_destination.starts_with(&canonical_mount) {
        return Err(format!(
            "AETHER_GUARD=BLOCKED offload destination escaped external mount: {}",
            canonical_destination.display()
        ));
    }
    Ok(canonical_destination)
}

pub fn offload_batch(batch: &CleanupBatch, destination_root: &Path) -> OffloadReport {
    let mut report = OffloadReport::default();
    let preflight = verify_batch(batch);
    if !preflight.is_ok() {
        report.failures.extend(preflight.issues);
        return report;
    }

    if let Err(error) = prepare_destination_root(destination_root) {
        report.failures.push(error);
        return report;
    }

    for entry in &batch.entries {
        match offload_entry(batch, entry, destination_root) {
            Ok(record) => {
                report.offloaded_files += 1;
                report.offloaded_bytes = report.offloaded_bytes.saturating_add(record.bytes);
                report.records.push(record);
            }
            Err(error) => report.failures.push(error),
        }
    }
    report
}

fn prepare_destination_root(destination_root: &Path) -> Result<(), String> {
    match fs::symlink_metadata(destination_root) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "AETHER_GUARD=BLOCKED destination root is a symlink: {}",
                    destination_root.display()
                ));
            }
            if !metadata.is_dir() {
                return Err(format!(
                    "destination root is not a directory: {}",
                    destination_root.display()
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(destination_root).map_err(|e| {
                format!(
                    "cannot create offload destination {}: {e}",
                    destination_root.display()
                )
            })?;
        }
        Err(error) => {
            return Err(format!(
                "cannot inspect offload destination {}: {error}",
                destination_root.display()
            ));
        }
    }
    Ok(())
}

fn offload_entry(
    batch: &CleanupBatch,
    entry: &crate::manifest::CleanupEntry,
    destination_root: &Path,
) -> Result<OffloadRecord, String> {
    let source = validate_entry_against_batch(batch, entry)?;
    let filename = source
        .file_name()
        .ok_or_else(|| format!("source has no filename: {}", source.display()))?;
    let destination = destination_root.join(filename);

    let source_hash = hash_file(&source)?;
    if destination.exists() || fs::symlink_metadata(&destination).is_ok() {
        return finish_existing_destination(batch, entry, &source, &destination, source_hash);
    }

    let mut destination_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|e| format!("cannot create destination {}: {e}", destination.display()))?;

    let copy_result = copy_source(&source, &mut destination_file);
    if let Err(error) = copy_result {
        drop(destination_file);
        let _ = fs::remove_file(&destination);
        return Err(error);
    }
    destination_file
        .flush()
        .map_err(|e| format!("cannot flush destination {}: {e}", destination.display()))?;
    destination_file
        .sync_all()
        .map_err(|e| format!("cannot fsync destination {}: {e}", destination.display()))?;
    drop(destination_file);

    let destination_hash = hash_file(&destination)?;
    if destination_hash != source_hash {
        let _ = fs::remove_file(&destination);
        return Err(format!(
            "AETHER_GUARD=BLOCKED checksum mismatch; source preserved: {}",
            source.display()
        ));
    }
    sync_directory(destination_root)?;

    if let Err(error) = validate_entry_against_batch(batch, entry) {
        let _ = fs::remove_file(&destination);
        let _ = sync_directory(destination_root);
        return Err(format!(
            "source changed during offload; source preserved: {error}"
        ));
    }

    fs::remove_file(&source).map_err(|e| {
        format!(
            "verified copy retained but source delete failed {}: {e}",
            source.display()
        )
    })?;

    Ok(OffloadRecord {
        source,
        destination,
        bytes: entry.bytes,
        sha256: source_hash,
    })
}

fn finish_existing_destination(
    batch: &CleanupBatch,
    entry: &crate::manifest::CleanupEntry,
    source: &Path,
    destination: &Path,
    source_hash: String,
) -> Result<OffloadRecord, String> {
    let metadata = fs::symlink_metadata(destination).map_err(|e| {
        format!(
            "cannot inspect existing destination {}: {e}",
            destination.display()
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED conflicting destination is not a regular file: {}",
            destination.display()
        ));
    }
    if metadata.len() != entry.bytes || hash_file(destination)? != source_hash {
        return Err(format!(
            "AETHER_GUARD=BLOCKED conflicting destination differs; source preserved: {}",
            destination.display()
        ));
    }
    File::open(destination)
        .and_then(|file| file.sync_all())
        .map_err(|e| {
            format!(
                "cannot fsync existing destination {}: {e}",
                destination.display()
            )
        })?;
    let parent = destination
        .parent()
        .ok_or_else(|| format!("destination has no parent: {}", destination.display()))?;
    sync_directory(parent)?;
    validate_entry_against_batch(batch, entry)?;
    fs::remove_file(source).map_err(|e| {
        format!(
            "verified existing copy retained but source delete failed {}: {e}",
            source.display()
        )
    })?;
    Ok(OffloadRecord {
        source: source.to_path_buf(),
        destination: destination.to_path_buf(),
        bytes: entry.bytes,
        sha256: source_hash,
    })
}

fn copy_source(source: &Path, destination: &mut File) -> Result<(), String> {
    let mut source_file =
        File::open(source).map_err(|e| format!("cannot open source {}: {e}", source.display()))?;
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let read = source_file
            .read(&mut buffer)
            .map_err(|e| format!("cannot read source {}: {e}", source.display()))?;
        if read == 0 {
            break;
        }
        destination
            .write_all(&buffer[..read])
            .map_err(|e| format!("cannot write offload copy for {}: {e}", source.display()))?;
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("cannot hash {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 128 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("cannot hash-read {}: {e}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(out)
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|e| format!("cannot fsync destination directory {}: {e}", path.display()))
}
