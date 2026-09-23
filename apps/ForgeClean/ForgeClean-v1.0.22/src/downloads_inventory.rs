use crate::project_routing::{ProjectIndex, has_project_marker, project_class_for_name};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadsInventorySummary {
    pub entry_paths: usize,
    pub file_paths: usize,
    pub directory_paths: usize,
    pub symlink_paths: usize,
    pub other_paths: usize,
    pub unique_inodes: usize,
    pub deduplicated_paths: usize,
    pub allocated_bytes: u64,
    pub logical_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadsProjectSummary {
    pub name: String,
    pub summary: DownloadsInventorySummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadsInventoryEntry {
    pub relative_path: PathBuf,
    pub kind: String,
    pub extension: String,
    pub allocated_bytes: u64,
    pub logical_bytes: u64,
    pub modified_unix: i64,
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub symlink_target: Option<PathBuf>,
    pub project: Option<String>,
    pub project_class: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DownloadsInventoryReport {
    pub root: PathBuf,
    pub total: DownloadsInventorySummary,
    pub general: DownloadsInventorySummary,
    pub project_files: DownloadsInventorySummary,
    pub projects: Vec<DownloadsProjectSummary>,
    pub entries: Vec<DownloadsInventoryEntry>,
    pub scan_errors: usize,
    pub scan_threads: usize,
}

#[derive(Debug, Default)]
struct ProjectAccumulator {
    summary: DownloadsInventorySummary,
}

#[derive(Debug, Default)]
struct PartialReport {
    total: DownloadsInventorySummary,
    general: DownloadsInventorySummary,
    project_files: DownloadsInventorySummary,
    projects: BTreeMap<String, ProjectAccumulator>,
    entries: Vec<DownloadsInventoryEntry>,
    scan_errors: usize,
}

#[derive(Debug, Clone)]
struct WorkItem {
    dir: PathBuf,
    inherited_project: Option<String>,
}

pub fn scan_downloads_inventory(downloads: &Path) -> Result<DownloadsInventoryReport, String> {
    scan_parallel(downloads, true, usize::MAX)
}

pub fn scan_downloads_inventory_summary(
    downloads: &Path,
) -> Result<DownloadsInventoryReport, String> {
    scan_parallel(downloads, true, 512)
}

pub fn configured_scan_threads() -> usize {
    if let Ok(value) = std::env::var("FORGECLEAN_SCAN_THREADS")
        && let Ok(parsed) = value.parse::<usize>()
        && parsed > 0
    {
        return parsed.min(64);
    }
    let cpus = thread::available_parallelism().map_or(4, |value| value.get());
    cpus.saturating_mul(2).clamp(4, 64)
}

fn scan_parallel(
    downloads: &Path,
    collect_entries: bool,
    entry_limit: usize,
) -> Result<DownloadsInventoryReport, String> {
    if !downloads.exists() {
        return Ok(DownloadsInventoryReport {
            root: downloads.to_path_buf(),
            scan_threads: configured_scan_threads(),
            ..DownloadsInventoryReport::default()
        });
    }
    if !downloads.is_dir() {
        return Err(format!(
            "downloads root is not a directory: {}",
            downloads.display()
        ));
    }

    let project_index = Arc::new(ProjectIndex::discover(downloads));
    let thread_count = configured_scan_threads();
    let (sender, receiver) = mpsc::channel::<WorkItem>();
    let receiver = Arc::new(Mutex::new(receiver));
    let pending = Arc::new(AtomicUsize::new(1));
    let partials = Arc::new(Mutex::new(Vec::<PartialReport>::new()));
    sender
        .send(WorkItem {
            dir: downloads.to_path_buf(),
            inherited_project: None,
        })
        .map_err(|error| error.to_string())?;

    thread::scope(|scope| {
        for _ in 0..thread_count {
            let receiver = Arc::clone(&receiver);
            let sender = sender.clone();
            let pending = Arc::clone(&pending);
            let partials = Arc::clone(&partials);
            let project_index = Arc::clone(&project_index);
            scope.spawn(move || {
                let mut partial = PartialReport::default();
                loop {
                    let received = {
                        let lock = receiver
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        lock.recv_timeout(Duration::from_millis(100))
                    };
                    match received {
                        Ok(work) => {
                            process_directory(
                                downloads,
                                work,
                                &project_index,
                                &sender,
                                &pending,
                                &mut partial,
                                collect_entries,
                                entry_limit,
                            );
                            pending.fetch_sub(1, Ordering::AcqRel);
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if pending.load(Ordering::Acquire) == 0 {
                                break;
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                partials
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(partial);
            });
        }
    });

    let mut report = DownloadsInventoryReport {
        root: downloads.to_path_buf(),
        scan_threads: thread_count,
        ..DownloadsInventoryReport::default()
    };
    let parts = Arc::try_unwrap(partials)
        .map_err(|_| "inventory worker state still shared".to_owned())?
        .into_inner()
        .map_err(|_| "inventory worker state poisoned".to_owned())?;
    let mut project_map = BTreeMap::<String, DownloadsInventorySummary>::new();
    for mut part in parts {
        merge_summary(&mut report.total, &part.total);
        merge_summary(&mut report.general, &part.general);
        merge_summary(&mut report.project_files, &part.project_files);
        report.scan_errors = report.scan_errors.saturating_add(part.scan_errors);
        for (name, acc) in part.projects {
            merge_summary(project_map.entry(name).or_default(), &acc.summary);
        }
        if collect_entries && report.entries.len() < entry_limit {
            let remaining = entry_limit.saturating_sub(report.entries.len());
            if part.entries.len() > remaining {
                part.entries.truncate(remaining);
            }
            report.entries.extend(part.entries);
        }
    }
    report
        .entries
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    report.projects = project_map
        .into_iter()
        .map(|(name, summary)| DownloadsProjectSummary { name, summary })
        .collect();
    report.projects.sort_by(|a, b| {
        b.summary
            .allocated_bytes
            .cmp(&a.summary.allocated_bytes)
            .then_with(|| {
                a.name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase())
            })
    });
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn process_directory(
    downloads: &Path,
    work: WorkItem,
    project_index: &ProjectIndex,
    sender: &mpsc::Sender<WorkItem>,
    pending: &AtomicUsize,
    partial: &mut PartialReport,
    collect_entries: bool,
    entry_limit: usize,
) {
    let entries = match fs::read_dir(&work.dir) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                partial.scan_errors += 1;
            }
            return;
        }
    };
    for child in entries {
        let path = match child {
            Ok(child) => child.path(),
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    partial.scan_errors += 1;
                }
                continue;
            }
        };
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(error) => {
                if error.kind() != std::io::ErrorKind::NotFound {
                    partial.scan_errors += 1;
                }
                continue;
            }
        };
        let project = project_for_path(
            downloads,
            &path,
            &meta,
            work.inherited_project.as_deref(),
            project_index,
        );

        account_summary(&mut partial.total, &meta);
        if let Some(project_name) = &project {
            account_summary(&mut partial.project_files, &meta);
            let acc = partial.projects.entry(project_name.clone()).or_default();
            account_summary(&mut acc.summary, &meta);
        } else {
            account_summary(&mut partial.general, &meta);
        }

        if collect_entries && partial.entries.len() < entry_limit {
            let relative_path = path
                .strip_prefix(downloads)
                .unwrap_or(path.as_path())
                .to_path_buf();
            let project_class = project
                .as_deref()
                .map(|_| project_class_for_path(&relative_path, &path, &meta));
            let symlink_target = if meta.file_type().is_symlink() {
                match fs::read_link(&path) {
                    Ok(target) => Some(target),
                    Err(error) => {
                        if error.kind() != std::io::ErrorKind::NotFound {
                            partial.scan_errors += 1;
                        }
                        None
                    }
                }
            } else {
                None
            };
            partial.entries.push(DownloadsInventoryEntry {
                relative_path,
                kind: entry_kind(&meta).to_owned(),
                extension: path
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("")
                    .to_owned(),
                allocated_bytes: meta.blocks().saturating_mul(512),
                logical_bytes: meta.len(),
                modified_unix: meta.mtime(),
                uid: meta.uid(),
                gid: meta.gid(),
                mode: meta.mode() & 0o7777,
                symlink_target,
                project: project.clone(),
                project_class,
            });
        }

        if meta.is_dir() && !meta.file_type().is_symlink() {
            pending.fetch_add(1, Ordering::AcqRel);
            if sender
                .send(WorkItem {
                    dir: path,
                    inherited_project: project,
                })
                .is_err()
            {
                pending.fetch_sub(1, Ordering::AcqRel);
                partial.scan_errors += 1;
            }
        }
    }
}

fn project_for_path(
    downloads: &Path,
    path: &Path,
    meta: &fs::Metadata,
    inherited_project: Option<&str>,
    project_index: &ProjectIndex,
) -> Option<String> {
    if let Some(project) = inherited_project {
        return Some(project.to_owned());
    }
    if let Some(project) = canonical_project(downloads, path) {
        return Some(project);
    }

    let top_level = path.parent().is_some_and(|parent| parent == downloads);
    if !top_level || meta.file_type().is_symlink() {
        return None;
    }
    project_index.family_for_top_level(path)
}

fn canonical_project(downloads: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(downloads).ok()?;
    let mut components = relative.components();
    let first = component_text(components.next()?)?;
    let name = match first {
        "Project Files" => component_text(components.next()?)?,
        "ForgeClean" => {
            if component_text(components.next()?)? != "Projects" {
                return None;
            }
            component_text(components.next()?)?
        }
        _ => return None,
    };
    (!name.is_empty()).then(|| name.to_owned())
}

fn component_text(component: Component<'_>) -> Option<&str> {
    match component {
        Component::Normal(value) => value.to_str(),
        _ => None,
    }
}

fn project_class_for_path(relative: &Path, path: &Path, meta: &fs::Metadata) -> String {
    let lower_components = relative
        .components()
        .filter_map(component_text)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    if lower_components
        .iter()
        .any(|name| is_generated_dir_name(name))
    {
        return "GENERATED".to_owned();
    }
    if lower_components.len() >= 3 && lower_components[0] == "project files" {
        match lower_components[2].as_str() {
            "active" => return "ACTIVE".to_owned(),
            "builds" => return "BUILD".to_owned(),
            "downloads" => return "PROJECT_DOWNLOAD".to_owned(),
            "releases" => return "RELEASE".to_owned(),
            "restored" => return "RESTORED".to_owned(),
            "cold" => return "COLD".to_owned(),
            _ => {}
        }
    }
    if lower_components.len() >= 4
        && lower_components[0] == "forgeclean"
        && lower_components[1] == "projects"
    {
        match lower_components[3].as_str() {
            "active" => return "ACTIVE".to_owned(),
            "builds" => return "BUILD".to_owned(),
            "downloads" => return "PROJECT_DOWNLOAD".to_owned(),
            "releases" => return "RELEASE".to_owned(),
            "restored" => return "RESTORED".to_owned(),
            "cold" => return "COLD".to_owned(),
            _ => {}
        }
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    project_class_for_name(name, meta.is_dir() && has_project_marker(path)).to_owned()
}

fn is_generated_dir_name(name: &str) -> bool {
    matches!(
        name,
        "target"
            | "build"
            | "out"
            | "obj"
            | "node_modules"
            | ".cache"
            | "cache"
            | "__pycache__"
            | ".gradle"
            | ".cargo"
            | ".sccache"
            | ".ccache"
    ) || name.starts_with("cmake-build-")
        || name.starts_with("build-")
}

fn entry_kind(meta: &fs::Metadata) -> &'static str {
    if meta.file_type().is_symlink() {
        "SYMLINK"
    } else if meta.is_file() {
        "FILE"
    } else if meta.is_dir() {
        "DIRECTORY"
    } else {
        "OTHER"
    }
}

fn account_summary(summary: &mut DownloadsInventorySummary, meta: &fs::Metadata) {
    summary.entry_paths += 1;
    if meta.file_type().is_symlink() {
        summary.symlink_paths += 1;
    } else if meta.is_file() {
        summary.file_paths += 1;
    } else if meta.is_dir() {
        summary.directory_paths += 1;
    } else {
        summary.other_paths += 1;
    }
    summary.unique_inodes += 1;
    summary.allocated_bytes = summary
        .allocated_bytes
        .saturating_add(meta.blocks().saturating_mul(512));
    summary.logical_bytes = summary.logical_bytes.saturating_add(meta.len());
}

fn merge_summary(target: &mut DownloadsInventorySummary, source: &DownloadsInventorySummary) {
    target.entry_paths = target.entry_paths.saturating_add(source.entry_paths);
    target.file_paths = target.file_paths.saturating_add(source.file_paths);
    target.directory_paths = target
        .directory_paths
        .saturating_add(source.directory_paths);
    target.symlink_paths = target.symlink_paths.saturating_add(source.symlink_paths);
    target.other_paths = target.other_paths.saturating_add(source.other_paths);
    target.unique_inodes = target.unique_inodes.saturating_add(source.unique_inodes);
    target.deduplicated_paths = target
        .deduplicated_paths
        .saturating_add(source.deduplicated_paths);
    target.allocated_bytes = target
        .allocated_bytes
        .saturating_add(source.allocated_bytes);
    target.logical_bytes = target.logical_bytes.saturating_add(source.logical_bytes);
}
