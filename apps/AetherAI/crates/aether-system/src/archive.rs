#![forbid(unsafe_code)]

use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::SystemError;

const MAX_ARCHIVE_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 100_000;
const MAX_ENTRY_NAME_BYTES: usize = 4096;
const MAX_PATH_COMPONENT_BYTES: usize = 255;
const MAX_SINGLE_FILE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_TOTAL_UNCOMPRESSED_BYTES: u64 = 32 * 1024 * 1024 * 1024;
const MAX_COMPRESSION_RATIO: u64 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArchiveEntryMeta {
    kind: char,
    uncompressed_bytes: u64,
    compressed_bytes: u64,
}

pub fn validate_archive_entry(entry: &str) -> Result<(), SystemError> {
    if entry.is_empty()
        || entry.len() > MAX_ENTRY_NAME_BYTES
        || entry.contains('\0')
        || entry.chars().any(|ch| ch.is_control())
    {
        return Err(SystemError::Platform(
            "archive entry contains an unsupported name".into(),
        ));
    }

    if entry.contains('\\') || entry.contains(':') || entry.starts_with('/') {
        return Err(SystemError::Platform(format!(
            "archive entry is not a portable relative path: {entry}"
        )));
    }

    let trimmed = entry.strip_suffix('/').unwrap_or(entry);
    if trimmed.is_empty() || trimmed.contains("//") {
        return Err(SystemError::Platform(format!(
            "archive entry contains an ambiguous path: {entry}"
        )));
    }

    for component in trimmed.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.len() > MAX_PATH_COMPONENT_BYTES
            || component.ends_with('.')
            || component.ends_with(' ')
            || windows_reserved_component(component)
        {
            return Err(SystemError::Platform(format!(
                "archive entry contains an unsafe component: {entry}"
            )));
        }
    }

    Ok(())
}

fn windows_reserved_component(component: &str) -> bool {
    let base = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();

    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || matches!(
            base.as_str(),
            "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
}

pub fn list_zip_archive(archive: &Path) -> Result<Vec<String>, SystemError> {
    ensure_archive_file_budget(archive)?;

    let output = platform_list_zip(archive)?;
    if !output.status.success() {
        return Err(SystemError::Platform(format!(
            "archive listing failed with status {}",
            output.status
        )));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| SystemError::Platform("archive listing is not UTF-8".into()))?;

    let mut entries = Vec::new();
    let mut portable_keys = BTreeSet::new();

    for line in stdout.lines() {
        let entry = line.trim_end_matches('\r');
        if entry.is_empty() {
            continue;
        }
        validate_archive_entry(entry)?;

        let key = entry.trim_end_matches('/').to_ascii_lowercase();
        if !portable_keys.insert(key) {
            return Err(SystemError::Platform(format!(
                "archive contains a duplicate/colliding path: {entry}"
            )));
        }

        entries.push(entry.to_string());
        if entries.len() > MAX_ARCHIVE_ENTRIES {
            return Err(SystemError::Platform(format!(
                "archive exceeds entry limit of {MAX_ARCHIVE_ENTRIES}"
            )));
        }
    }

    if entries.is_empty() {
        return Err(SystemError::Platform("archive contains no entries".into()));
    }

    Ok(entries)
}

pub fn extract_zip_archive(archive: &Path, destination: &Path) -> Result<(), SystemError> {
    let entries = list_zip_archive(archive)?;
    let archive_bytes = ensure_archive_file_budget(archive)?;
    let metadata = inspect_zip_archive(archive)?;

    if metadata.len() != entries.len() {
        return Err(SystemError::Platform(format!(
            "archive metadata count mismatch: names={} metadata={}",
            entries.len(),
            metadata.len()
        )));
    }
    enforce_archive_limits(archive_bytes, &metadata)?;

    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let canonical_parent = parent.canonicalize()?;
    ensure_empty_destination(destination)?;

    let destination_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SystemError::Platform("archive destination has no portable name".into()))?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| SystemError::Platform(error.to_string()))?
        .as_nanos();
    let staging = canonical_parent.join(format!(
        ".{destination_name}.aetherai-extract-{}-{nonce}",
        std::process::id()
    ));

    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir(&staging)?;

    let extraction = platform_extract_zip(archive, &staging);
    let status = match extraction {
        Ok(status) => status,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };

    if !status.success() {
        let _ = fs::remove_dir_all(&staging);
        return Err(SystemError::Platform(format!(
            "archive extraction failed with status {status}"
        )));
    }

    if let Err(error) = verify_extracted_tree(&staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    if destination.exists() {
        fs::remove_dir(destination)?;
    }
    if let Err(error) = fs::rename(&staging, destination) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error.into());
    }

    let canonical_destination = destination.canonicalize()?;
    if !canonical_destination.starts_with(&canonical_parent) {
        let _ = fs::remove_dir_all(destination);
        return Err(SystemError::Platform(
            "archive destination resolved outside its canonical parent".into(),
        ));
    }

    Ok(())
}

fn ensure_archive_file_budget(archive: &Path) -> Result<u64, SystemError> {
    let metadata = fs::metadata(archive).map_err(|error| {
        SystemError::Platform(format!(
            "archive cannot be read ({}): {error}",
            archive.display()
        ))
    })?;
    if !metadata.is_file() {
        return Err(SystemError::Platform(format!(
            "archive is not a regular file: {}",
            archive.display()
        )));
    }
    if metadata.len() == 0 || metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(SystemError::Platform(format!(
            "archive size {} is outside allowed range 1..={MAX_ARCHIVE_BYTES}",
            metadata.len()
        )));
    }
    Ok(metadata.len())
}

fn ensure_empty_destination(destination: &Path) -> Result<(), SystemError> {
    if !destination.exists() {
        return Ok(());
    }

    let metadata = fs::symlink_metadata(destination)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(SystemError::Platform(format!(
            "archive destination is not a plain directory: {}",
            destination.display()
        )));
    }

    if fs::read_dir(destination)?.next().transpose()?.is_some() {
        return Err(SystemError::Platform(format!(
            "archive destination must be empty: {}",
            destination.display()
        )));
    }

    Ok(())
}

fn enforce_archive_limits(
    archive_bytes: u64,
    metadata: &[ArchiveEntryMeta],
) -> Result<(), SystemError> {
    if metadata.is_empty() || metadata.len() > MAX_ARCHIVE_ENTRIES {
        return Err(SystemError::Platform(
            "archive metadata entry count is outside allowed range".into(),
        ));
    }

    let mut total_uncompressed = 0_u64;
    for entry in metadata {
        if !matches!(entry.kind, '-' | 'd' | '?') {
            return Err(SystemError::Platform(
                "archive contains unsupported link/device entry".into(),
            ));
        }

        if entry.uncompressed_bytes > MAX_SINGLE_FILE_BYTES {
            return Err(SystemError::Platform(format!(
                "archive entry exceeds single-file limit of {MAX_SINGLE_FILE_BYTES} bytes"
            )));
        }

        total_uncompressed = total_uncompressed
            .checked_add(entry.uncompressed_bytes)
            .ok_or_else(|| SystemError::Platform("archive size total overflowed".into()))?;

        if entry.uncompressed_bytes > 0
            && (entry.compressed_bytes == 0
                || u128::from(entry.uncompressed_bytes)
                    > u128::from(entry.compressed_bytes) * u128::from(MAX_COMPRESSION_RATIO))
        {
            return Err(SystemError::Platform(format!(
                "archive entry exceeds compression ratio limit of {MAX_COMPRESSION_RATIO}:1"
            )));
        }
    }

    if total_uncompressed > MAX_TOTAL_UNCOMPRESSED_BYTES {
        return Err(SystemError::Platform(format!(
            "archive exceeds total uncompressed limit of {MAX_TOTAL_UNCOMPRESSED_BYTES} bytes"
        )));
    }

    if total_uncompressed > 0
        && u128::from(total_uncompressed)
            > u128::from(archive_bytes) * u128::from(MAX_COMPRESSION_RATIO)
    {
        return Err(SystemError::Platform(format!(
            "archive exceeds total compression ratio limit of {MAX_COMPRESSION_RATIO}:1"
        )));
    }

    Ok(())
}

fn verify_extracted_tree(destination: &Path) -> Result<(), SystemError> {
    let root = destination.canonicalize()?;
    let mut stack = vec![destination.to_path_buf()];
    let mut entries = 0_usize;
    let mut total_bytes = 0_u64;

    while let Some(current) = stack.pop() {
        for item in fs::read_dir(&current)? {
            let item = item?;
            entries += 1;
            if entries > MAX_ARCHIVE_ENTRIES {
                return Err(SystemError::Platform(
                    "extracted tree exceeds entry limit".into(),
                ));
            }

            let file_type = item.file_type()?;
            if file_type.is_symlink() {
                return Err(SystemError::Platform(format!(
                    "archive extraction produced a symbolic link: {}",
                    item.path().display()
                )));
            }
            if !file_type.is_file() && !file_type.is_dir() {
                return Err(SystemError::Platform(format!(
                    "archive extraction produced an unsupported filesystem entry: {}",
                    item.path().display()
                )));
            }

            let canonical = item.path().canonicalize()?;
            if !canonical.starts_with(&root) {
                return Err(SystemError::Platform(format!(
                    "archive extraction escaped destination: {}",
                    canonical.display()
                )));
            }

            if file_type.is_dir() {
                stack.push(item.path());
                continue;
            }

            let size = item.metadata()?.len();
            if size > MAX_SINGLE_FILE_BYTES {
                return Err(SystemError::Platform(
                    "extracted file exceeds single-file limit".into(),
                ));
            }
            total_bytes = total_bytes
                .checked_add(size)
                .ok_or_else(|| SystemError::Platform("extracted size overflowed".into()))?;
            if total_bytes > MAX_TOTAL_UNCOMPRESSED_BYTES {
                return Err(SystemError::Platform(
                    "extracted tree exceeds total byte limit".into(),
                ));
            }

            #[cfg(unix)]
            strip_executable_bits(&canonical)?;
        }
    }

    Ok(())
}

#[cfg(unix)]
fn strip_executable_bits(path: &Path) -> Result<(), SystemError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    let mode = permissions.mode();
    permissions.set_mode(mode & !0o111);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn platform_list_zip(archive: &Path) -> Result<std::process::Output, SystemError> {
    let primary = Command::new("bsdtar").args(["-tf"]).arg(archive).output();
    match primary {
        Ok(output) if output.status.success() => Ok(output),
        Ok(_) | Err(_) => Ok(Command::new("unzip").arg("-Z1").arg(archive).output()?),
    }
}

#[cfg(target_os = "linux")]
fn inspect_zip_archive(archive: &Path) -> Result<Vec<ArchiveEntryMeta>, SystemError> {
    let output = Command::new("zipinfo").arg("-l").arg(archive).output()?;
    if !output.status.success() {
        return Err(SystemError::Platform(format!(
            "zip metadata inspection failed with status {}",
            output.status
        )));
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|_| SystemError::Platform("zip metadata listing is not UTF-8".into()))?;
    let mut metadata = Vec::new();

    for line in text.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 9 || !fields[1].contains('.') || fields[2].len() != 3 {
            continue;
        }

        let Ok(uncompressed_bytes) = fields[3].parse::<u64>() else {
            continue;
        };
        let Ok(compressed_bytes) = fields[5].parse::<u64>() else {
            continue;
        };
        let kind = fields[0].chars().next().unwrap_or('!');
        metadata.push(ArchiveEntryMeta {
            kind,
            uncompressed_bytes,
            compressed_bytes,
        });
    }

    if metadata.is_empty() {
        return Err(SystemError::Platform(
            "zip metadata inspection produced no entries".into(),
        ));
    }
    Ok(metadata)
}

#[cfg(target_os = "linux")]
fn platform_extract_zip(
    archive: &Path,
    destination: &Path,
) -> Result<std::process::ExitStatus, SystemError> {
    match Command::new("bsdtar")
        .args(["--no-same-owner", "--no-same-permissions", "-xf"])
        .arg(archive)
        .arg("-C")
        .arg(destination)
        .status()
    {
        Ok(status) => Ok(status),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Command::new("unzip")
            .arg("-qq")
            .arg(archive)
            .arg("-d")
            .arg(destination)
            .status()?),
        Err(error) => Err(error.into()),
    }
}

#[cfg(target_os = "windows")]
fn platform_list_zip(archive: &Path) -> Result<std::process::Output, SystemError> {
    Ok(Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.IO.Compression.FileSystem; \
             $z=[IO.Compression.ZipFile]::OpenRead($env:AETHERAI_ARCHIVE); \
             try { $z.Entries | ForEach-Object { $_.FullName } } finally { $z.Dispose() }",
        ])
        .env("AETHERAI_ARCHIVE", archive)
        .output()?)
}

#[cfg(target_os = "windows")]
fn inspect_zip_archive(archive: &Path) -> Result<Vec<ArchiveEntryMeta>, SystemError> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.IO.Compression.FileSystem; \
             $z=[IO.Compression.ZipFile]::OpenRead($env:AETHERAI_ARCHIVE); \
             try { \
               $z.Entries | ForEach-Object { \
                 $mode=([uint32]$_.ExternalAttributes -shr 16) -band 0xF000; \
                 if ($_.FullName.EndsWith('/')) { $kind='d' } \
                 elseif ($mode -eq 0xA000) { $kind='l' } \
                 elseif (($mode -eq 0) -or ($mode -eq 0x8000)) { $kind='-' } \
                 elseif ($mode -eq 0x4000) { $kind='d' } \
                 else { $kind='x' }; \
                 Write-Output ($kind + \"`t\" + $_.Length + \"`t\" + $_.CompressedLength) \
               } \
             } finally { $z.Dispose() }",
        ])
        .env("AETHERAI_ARCHIVE", archive)
        .output()?;

    if !output.status.success() {
        return Err(SystemError::Platform(format!(
            "zip metadata inspection failed with status {}",
            output.status
        )));
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|_| SystemError::Platform("zip metadata listing is not UTF-8".into()))?;
    let mut metadata = Vec::new();

    for line in text.lines() {
        let mut fields = line.trim_end_matches('\r').split('\t');
        let kind = fields.next().and_then(|value| value.chars().next());
        let uncompressed_bytes = fields.next().and_then(|value| value.parse::<u64>().ok());
        let compressed_bytes = fields.next().and_then(|value| value.parse::<u64>().ok());

        let (Some(kind), Some(uncompressed_bytes), Some(compressed_bytes)) =
            (kind, uncompressed_bytes, compressed_bytes)
        else {
            return Err(SystemError::Platform(
                "zip metadata inspection returned an invalid record".into(),
            ));
        };

        metadata.push(ArchiveEntryMeta {
            kind,
            uncompressed_bytes,
            compressed_bytes,
        });
    }

    if metadata.is_empty() {
        return Err(SystemError::Platform(
            "zip metadata inspection produced no entries".into(),
        ));
    }
    Ok(metadata)
}

#[cfg(target_os = "windows")]
fn platform_extract_zip(
    archive: &Path,
    destination: &Path,
) -> Result<std::process::ExitStatus, SystemError> {
    Ok(Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Expand-Archive -LiteralPath $env:AETHERAI_ARCHIVE \
             -DestinationPath $env:AETHERAI_DESTINATION",
        ])
        .env("AETHERAI_ARCHIVE", archive)
        .env("AETHERAI_DESTINATION", destination)
        .status()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_rejects_extreme_ratio_before_extraction() {
        let metadata = [ArchiveEntryMeta {
            kind: '-',
            uncompressed_bytes: 2_000_000,
            compressed_bytes: 1,
        }];
        assert!(enforce_archive_limits(100, &metadata).is_err());
    }

    #[test]
    fn budget_rejects_unsafe_entry_types() {
        let metadata = [ArchiveEntryMeta {
            kind: 'l',
            uncompressed_bytes: 4,
            compressed_bytes: 4,
        }];
        assert!(enforce_archive_limits(100, &metadata).is_err());
    }

    #[test]
    fn portable_windows_device_names_are_rejected() {
        assert!(validate_archive_entry("NUL.txt").is_err());
        assert!(validate_archive_entry("folder/COM1.log").is_err());
        assert!(validate_archive_entry("folder/normal.log").is_ok());
    }
}
