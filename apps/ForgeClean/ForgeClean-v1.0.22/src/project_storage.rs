use crate::organizer::build_identity_from_entry;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectStorageProject {
    pub name: String,
    pub allocated_bytes: u64,
    pub logical_bytes: u64,
    pub artifact_paths: usize,
    pub unique_files: usize,
    pub deduplicated_paths: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectStorageReport {
    pub allocated_bytes: u64,
    pub logical_bytes: u64,
    pub artifact_paths: usize,
    pub unique_files: usize,
    pub deduplicated_paths: usize,
    pub unrouted_artifact_paths: usize,
    pub excluded_generated_dirs: usize,
    pub symlinks_skipped: usize,
    pub scan_errors: usize,
    pub projects: Vec<ProjectStorageProject>,
}

#[derive(Debug, Default)]
struct ProjectAccumulator {
    allocated_bytes: u64,
    logical_bytes: u64,
    artifact_paths: usize,
    unique_files: usize,
    deduplicated_paths: usize,
}

pub fn scan_project_download_storage(downloads: &Path) -> Result<ProjectStorageReport, String> {
    let mut report = ProjectStorageReport::default();
    if !downloads.exists() {
        return Ok(report);
    }

    let mut projects = BTreeMap::<String, ProjectAccumulator>::new();
    let mut seen = HashSet::<(u64, u64)>::new();
    let projects_root = downloads.join("ForgeClean/Projects");

    if projects_root.is_dir() {
        for project_path in sorted_children(&projects_root, &mut report)? {
            let Ok(meta) = fs::symlink_metadata(&project_path) else {
                report.scan_errors += 1;
                continue;
            };
            if meta.file_type().is_symlink() {
                report.symlinks_skipped += 1;
                continue;
            }
            if !meta.is_dir() {
                continue;
            }
            let Some(name) = project_path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            for scope in ["Builds", "Downloads", "Releases"] {
                let root = project_path.join(scope);
                if root.is_dir() {
                    scan_tree(&root, name, &mut projects, &mut seen, &mut report);
                }
            }
        }
    }

    for path in sorted_children(downloads, &mut report)? {
        if path.file_name().is_some_and(|name| name == "ForgeClean") {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&path) else {
            report.scan_errors += 1;
            continue;
        };
        if meta.file_type().is_symlink() {
            report.symlinks_skipped += 1;
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        let Some(identity) = build_identity_from_entry(&path) else {
            continue;
        };
        report.unrouted_artifact_paths += 1;
        account_file(
            &identity.project,
            &meta,
            &mut projects,
            &mut seen,
            &mut report,
        );
    }

    report.projects = projects
        .into_iter()
        .map(|(name, acc)| ProjectStorageProject {
            name,
            allocated_bytes: acc.allocated_bytes,
            logical_bytes: acc.logical_bytes,
            artifact_paths: acc.artifact_paths,
            unique_files: acc.unique_files,
            deduplicated_paths: acc.deduplicated_paths,
        })
        .collect();
    report.projects.sort_by(|a, b| {
        b.allocated_bytes.cmp(&a.allocated_bytes).then_with(|| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        })
    });
    Ok(report)
}

fn scan_tree(
    root: &Path,
    project: &str,
    projects: &mut BTreeMap<String, ProjectAccumulator>,
    seen: &mut HashSet<(u64, u64)>,
    report: &mut ProjectStorageReport,
) {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => {
                report.scan_errors += 1;
                continue;
            }
        };
        let mut paths = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect::<Vec<PathBuf>>();
        paths.sort();
        for path in paths.into_iter().rev() {
            let meta = match fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(_) => {
                    report.scan_errors += 1;
                    continue;
                }
            };
            if meta.file_type().is_symlink() {
                report.symlinks_skipped += 1;
                continue;
            }
            if meta.is_dir() {
                if is_generated_dir(&path) {
                    report.excluded_generated_dirs += 1;
                } else {
                    stack.push(path);
                }
            } else if meta.is_file() {
                account_file(project, &meta, projects, seen, report);
            }
        }
    }
}

fn account_file(
    project: &str,
    meta: &fs::Metadata,
    projects: &mut BTreeMap<String, ProjectAccumulator>,
    seen: &mut HashSet<(u64, u64)>,
    report: &mut ProjectStorageReport,
) {
    report.artifact_paths += 1;
    let project_report = projects.entry(project.to_owned()).or_default();
    project_report.artifact_paths += 1;

    let identity = (meta.dev(), meta.ino());
    if !seen.insert(identity) {
        report.deduplicated_paths += 1;
        project_report.deduplicated_paths += 1;
        return;
    }

    let allocated = meta.blocks().saturating_mul(512);
    let logical = meta.len();
    report.allocated_bytes = report.allocated_bytes.saturating_add(allocated);
    report.logical_bytes = report.logical_bytes.saturating_add(logical);
    report.unique_files += 1;
    project_report.allocated_bytes = project_report.allocated_bytes.saturating_add(allocated);
    project_report.logical_bytes = project_report.logical_bytes.saturating_add(logical);
    project_report.unique_files += 1;
}

fn is_generated_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
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
            | ".git"
    ) || name.starts_with("cmake-build-")
        || name.starts_with("build-")
}

fn sorted_children(path: &Path, report: &mut ProjectStorageReport) -> Result<Vec<PathBuf>, String> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let mut children = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => children.push(entry.path()),
            Err(_) => report.scan_errors += 1,
        }
    }
    children.sort();
    Ok(children)
}
