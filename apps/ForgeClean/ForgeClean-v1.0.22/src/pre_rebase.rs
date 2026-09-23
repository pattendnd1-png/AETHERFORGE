use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, Metadata};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 2026-08-18T00:00:00-07:00 (America/Los_Angeles), the user's Garuda rebase boundary.
pub const GARUDA_REBASE_CUTOFF_UNIX_SECS: i64 = 1_787_036_400;
pub const STORAGE_PRESSURE_PERCENT: u8 = 70;
pub const STORAGE_CRITICAL_PERCENT: u8 = 85;
pub const DAILY_SWEEP_INTERVAL_SECS: u64 = 86_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimestampKind {
    Accessed,
    Modified,
    Changed,
    Created,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileIdentity {
    pub dev: u64,
    pub ino: u64,
    pub uid: u32,
    pub len: u64,
    pub mtime: i64,
    pub mtime_nsec: i64,
    pub ctime: i64,
    pub ctime_nsec: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreRebaseCandidate {
    pub path: PathBuf,
    pub size: u64,
    pub old_timestamps: Vec<TimestampKind>,
    pub identity: FileIdentity,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreRebaseScanReport {
    pub roots: Vec<PathBuf>,
    pub candidates: Vec<PreRebaseCandidate>,
    pub candidate_bytes: u64,
    pub protected_entries: u64,
    pub symlinks_skipped: u64,
    pub non_owned_skipped: u64,
    pub scan_errors: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreRebasePurgeReport {
    pub deleted_files: u64,
    pub deleted_bytes: u64,
    pub changed_since_scan: u64,
    pub protected_since_scan: u64,
    pub outside_roots: u64,
    pub delete_errors: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreRebaseState {
    pub last_run_unix_secs: Option<u64>,
    pub last_boot_id: Option<String>,
    pub last_reason: Option<String>,
    pub last_deleted_files: u64,
    pub last_deleted_bytes: u64,
    pub cumulative_deleted_files: u64,
    pub cumulative_deleted_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoReason {
    FirstRun,
    Startup,
    Daily,
    StoragePressure(u8),
}

impl AutoReason {
    pub fn as_label(&self) -> String {
        match self {
            Self::FirstRun => "FIRST_RUN".to_owned(),
            Self::Startup => "STARTUP".to_owned(),
            Self::Daily => "DAILY".to_owned(),
            Self::StoragePressure(value) => format!("STORAGE_PRESSURE_{value}_PERCENT"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PreRebasePolicy {
    pub home: PathBuf,
    pub roots: Vec<PathBuf>,
    pub owner_uid: u32,
    pub cutoff_unix_secs: i64,
}

impl PreRebasePolicy {
    pub fn for_home(home: &Path) -> io::Result<Self> {
        let home = fs::canonicalize(home)?;
        let owner_uid = fs::metadata(&home)?.uid();
        let roots = discover_user_owned_roots(&home, owner_uid)?;
        Ok(Self {
            home,
            roots,
            owner_uid,
            cutoff_unix_secs: GARUDA_REBASE_CUTOFF_UNIX_SECS,
        })
    }

    pub fn with_roots(home: &Path, roots: Vec<PathBuf>) -> io::Result<Self> {
        let home = fs::canonicalize(home)?;
        let owner_uid = fs::metadata(&home)?.uid();
        let mut canonical_roots = Vec::new();
        for root in roots {
            if let Ok(root) = fs::canonicalize(root) {
                canonical_roots.push(root);
            }
        }
        canonical_roots.sort();
        canonical_roots.dedup();
        Ok(Self {
            home,
            roots: canonical_roots,
            owner_uid,
            cutoff_unix_secs: GARUDA_REBASE_CUTOFF_UNIX_SECS,
        })
    }
}

pub fn scan(policy: &PreRebasePolicy) -> PreRebaseScanReport {
    let mut report = PreRebaseScanReport {
        roots: policy.roots.clone(),
        ..Default::default()
    };
    let mut seen_files = BTreeSet::new();
    for root in &policy.roots {
        walk_dir(root, root, policy, &mut seen_files, &mut report);
    }
    report
}

pub fn purge(policy: &PreRebasePolicy, candidates: &[PreRebaseCandidate]) -> PreRebasePurgeReport {
    let mut report = PreRebasePurgeReport::default();
    for candidate in candidates {
        let Some(root) = containing_root(&candidate.path, &policy.roots) else {
            report.outside_roots += 1;
            continue;
        };
        if !parents_are_symlink_free(root, &candidate.path) {
            report.changed_since_scan += 1;
            continue;
        }
        if is_protected_under_root(root, &policy.home, &candidate.path) {
            report.protected_since_scan += 1;
            continue;
        }
        let metadata = match fs::symlink_metadata(&candidate.path) {
            Ok(metadata) => metadata,
            Err(_) => {
                report.changed_since_scan += 1;
                continue;
            }
        };
        if metadata.file_type().is_symlink()
            || !metadata.file_type().is_file()
            || metadata.uid() != policy.owner_uid
            || identity(&metadata) != candidate.identity
            || old_timestamps(&metadata, policy.cutoff_unix_secs).is_empty()
        {
            report.changed_since_scan += 1;
            continue;
        }
        match fs::remove_file(&candidate.path) {
            Ok(()) => {
                report.deleted_files += 1;
                report.deleted_bytes = report.deleted_bytes.saturating_add(candidate.size);
            }
            Err(_) => report.delete_errors += 1,
        }
    }
    report
}

pub fn state_path(home: &Path) -> PathBuf {
    home.join(".local/state/forgeclean/pre-rebase-state.json")
}

pub fn read_state(home: &Path) -> io::Result<PreRebaseState> {
    let path = state_path(home);
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(PreRebaseState::default()),
        Err(error) => Err(error),
    }
}

pub fn write_state(home: &Path, state: &PreRebaseState) -> io::Result<PathBuf> {
    let path = state_path(home);
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("state path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".pre-rebase-state.{}.tmp", std::process::id()));
    let body = serde_json::to_vec_pretty(state).map_err(io::Error::other)?;
    fs::write(&temporary, body)?;
    fs::rename(&temporary, &path)?;
    Ok(path)
}

pub fn current_boot_id() -> Option<String> {
    fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub fn auto_reason(
    state: &PreRebaseState,
    now: u64,
    boot_id: Option<&str>,
    storage_used_percent: u8,
) -> Option<AutoReason> {
    if storage_used_percent >= STORAGE_PRESSURE_PERCENT {
        return Some(AutoReason::StoragePressure(storage_used_percent));
    }
    let Some(last_run) = state.last_run_unix_secs else {
        return Some(AutoReason::FirstRun);
    };
    if boot_id.is_some() && state.last_boot_id.as_deref() != boot_id {
        return Some(AutoReason::Startup);
    }
    if now.saturating_sub(last_run) >= DAILY_SWEEP_INTERVAL_SECS {
        return Some(AutoReason::Daily);
    }
    None
}

pub fn is_protected_path(home: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(home) else {
        return has_protected_component(path);
    };
    if relative.as_os_str().is_empty() {
        return true;
    }
    if has_hidden_component(relative) || has_protected_component(relative) {
        return true;
    }
    let parts: Vec<String> = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_ascii_lowercase()),
            _ => None,
        })
        .collect();
    has_active_project_path(&parts) || is_current_release_source(&parts)
}

pub fn discover_user_owned_roots(home: &Path, owner_uid: u32) -> io::Result<Vec<PathBuf>> {
    let mut roots = vec![home.to_path_buf()];
    let username = home
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mountinfo = fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
    for line in mountinfo.lines() {
        let Some((left, right)) = line.split_once(" - ") else {
            continue;
        };
        let left_fields: Vec<&str> = left.split_whitespace().collect();
        let right_fields: Vec<&str> = right.split_whitespace().collect();
        if left_fields.len() < 5 || right_fields.is_empty() {
            continue;
        }
        let fs_type = right_fields[0];
        if !is_local_data_filesystem(fs_type) {
            continue;
        }
        let mount = PathBuf::from(unescape_mount_field(left_fields[4]));
        if mount == home || mount.starts_with(home) {
            continue;
        }
        if !is_user_mount_prefix(&mount, &username) {
            continue;
        }
        let Ok(metadata) = fs::metadata(&mount) else {
            continue;
        };
        if !metadata.is_dir() || metadata.uid() != owner_uid {
            continue;
        }
        roots.push(mount);
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn walk_dir(
    root: &Path,
    dir: &Path,
    policy: &PreRebasePolicy,
    seen_files: &mut BTreeSet<PathBuf>,
    report: &mut PreRebaseScanReport,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => {
            report.scan_errors += 1;
            return;
        }
    };
    for entry in entries {
        let Ok(entry) = entry else {
            report.scan_errors += 1;
            continue;
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                report.scan_errors += 1;
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            report.symlinks_skipped += 1;
            continue;
        }
        if is_protected_under_root(root, &policy.home, &path) {
            report.protected_entries += 1;
            continue;
        }
        if metadata.file_type().is_dir() {
            if is_vcs_checkout_dir(&path) {
                report.protected_entries += 1;
                continue;
            }
            walk_dir(root, &path, policy, seen_files, report);
            continue;
        }
        if !metadata.file_type().is_file() {
            continue;
        }
        if metadata.uid() != policy.owner_uid {
            report.non_owned_skipped += 1;
            continue;
        }
        if !seen_files.insert(path.clone()) {
            continue;
        }
        let timestamps = old_timestamps(&metadata, policy.cutoff_unix_secs);
        if timestamps.is_empty() {
            continue;
        }
        report.candidate_bytes = report.candidate_bytes.saturating_add(metadata.len());
        report.candidates.push(PreRebaseCandidate {
            path,
            size: metadata.len(),
            old_timestamps: timestamps,
            identity: identity(&metadata),
        });
    }
}

fn is_protected_under_root(root: &Path, home: &Path, path: &Path) -> bool {
    if path.starts_with(home) {
        return is_protected_path(home, path);
    }
    let Ok(relative) = path.strip_prefix(root) else {
        return true;
    };
    if has_hidden_component(relative) || has_protected_component(relative) {
        return true;
    }
    let parts: Vec<String> = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_ascii_lowercase()),
            _ => None,
        })
        .collect();
    has_active_project_path(&parts) || is_current_release_source(&parts)
}

fn is_vcs_checkout_dir(path: &Path) -> bool {
    [".git", ".hg", ".svn"]
        .iter()
        .any(|marker| fs::symlink_metadata(path.join(marker)).is_ok())
}

fn has_hidden_component(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(value) => value.to_string_lossy().starts_with('.'),
        _ => false,
    })
}

fn has_protected_component(path: &Path) -> bool {
    path.components().any(|component| {
        let Component::Normal(value) = component else {
            return false;
        };
        matches!(
            value.to_string_lossy().to_ascii_lowercase().as_str(),
            "saved games" | "save" | "saves" | "userdata" | "credentials" | "secrets" | "keys"
        )
    })
}

fn containing_root<'a>(path: &Path, roots: &'a [PathBuf]) -> Option<&'a Path> {
    roots
        .iter()
        .filter(|root| path.starts_with(root.as_path()))
        .max_by_key(|root| root.components().count())
        .map(PathBuf::as_path)
}

fn parents_are_symlink_free(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let mut current = root.to_path_buf();
    let components: Vec<_> = relative.components().collect();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        let Component::Normal(value) = component else {
            return false;
        };
        current.push(value);
        let Ok(metadata) = fs::symlink_metadata(&current) else {
            return false;
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return false;
        }
    }
    true
}

fn is_current_release_source(parts: &[String]) -> bool {
    let current = format!("forgeclean-v{}", env!("CARGO_PKG_VERSION")).to_ascii_lowercase();
    parts.iter().any(|part| part == &current)
}

fn has_active_project_path(parts: &[String]) -> bool {
    parts
        .windows(3)
        .any(|window| window[0] == "projects" && window[2] == "active")
}

fn identity(metadata: &Metadata) -> FileIdentity {
    FileIdentity {
        dev: metadata.dev(),
        ino: metadata.ino(),
        uid: metadata.uid(),
        len: metadata.len(),
        mtime: metadata.mtime(),
        mtime_nsec: metadata.mtime_nsec(),
        ctime: metadata.ctime(),
        ctime_nsec: metadata.ctime_nsec(),
    }
}

fn old_timestamps(metadata: &Metadata, cutoff: i64) -> Vec<TimestampKind> {
    let mut kinds = Vec::new();
    if metadata.atime() < cutoff {
        kinds.push(TimestampKind::Accessed);
    }
    if metadata.mtime() < cutoff {
        kinds.push(TimestampKind::Modified);
    }
    if metadata.ctime() < cutoff {
        kinds.push(TimestampKind::Changed);
    }
    if metadata
        .created()
        .ok()
        .and_then(system_time_secs)
        .is_some_and(|created| created < cutoff)
    {
        kinds.push(TimestampKind::Created);
    }
    kinds
}

fn system_time_secs(value: SystemTime) -> Option<i64> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
}

fn is_local_data_filesystem(fs_type: &str) -> bool {
    matches!(
        fs_type,
        "btrfs"
            | "ext2"
            | "ext3"
            | "ext4"
            | "xfs"
            | "f2fs"
            | "vfat"
            | "exfat"
            | "ntfs"
            | "ntfs3"
            | "fuseblk"
            | "zfs"
            | "bcachefs"
    )
}

fn is_user_mount_prefix(path: &Path, username: &str) -> bool {
    path.starts_with(Path::new("/mnt"))
        || (!username.is_empty()
            && (path.starts_with(Path::new("/run/media").join(username))
                || path.starts_with(Path::new("/media").join(username))))
}

fn unescape_mount_field(value: &str) -> String {
    value
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
        .replace("\\134", "\\")
}
