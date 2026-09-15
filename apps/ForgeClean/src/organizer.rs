use crate::coldstore::archive_to_coldpack;
use crate::registry::{ProjectRegistry, create_legacy_alias};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Packages,
    Documents,
    Images,
    Video,
    Audio,
    Archives,
    Logs,
    Installers,
    Temporary,
    Other,
}

impl Category {
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Packages => "Packages",
            Self::Documents => "Documents",
            Self::Images => "Images",
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::Archives => "Archives",
            Self::Logs => "Logs",
            Self::Installers => "Installers",
            Self::Temporary => "Temporary",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ForgeLayout {
    pub downloads: PathBuf,
    pub root: PathBuf,
}

impl ForgeLayout {
    pub fn new(downloads: impl Into<PathBuf>) -> Self {
        let downloads = downloads.into();
        let root = downloads.join("ForgeClean");
        Self { downloads, root }
    }

    pub fn ensure(&self) -> Result<(), String> {
        fs::create_dir_all(self.root.join("Projects")).map_err(|e| e.to_string())?;
        fs::create_dir_all(self.root.join("ColdStorage")).map_err(|e| e.to_string())?;
        fs::create_dir_all(self.coldpack_store()).map_err(|e| e.to_string())?;
        fs::create_dir_all(self.root.join("Restored")).map_err(|e| e.to_string())?;
        for category in [
            Category::Packages,
            Category::Documents,
            Category::Images,
            Category::Video,
            Category::Audio,
            Category::Archives,
            Category::Logs,
            Category::Installers,
            Category::Temporary,
            Category::Other,
        ] {
            fs::create_dir_all(self.root.join(category.dir_name())).map_err(|e| e.to_string())?;
            fs::create_dir_all(self.root.join("ColdStorage").join(category.dir_name()))
                .map_err(|e| e.to_string())?;
            fs::create_dir_all(self.root.join("Restored").join(category.dir_name()))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn project_root(&self, project: &str) -> PathBuf {
        self.root.join("Projects").join(project)
    }

    pub fn active_dir(&self, project: &str) -> PathBuf {
        self.project_root(project).join("Active")
    }

    pub fn project_builds(&self, project: &str) -> PathBuf {
        self.project_root(project).join("Builds")
    }

    pub fn project_build(&self, project: &str, version: &str) -> PathBuf {
        self.project_builds(project).join(version)
    }

    pub fn project_downloads(&self, project: &str) -> PathBuf {
        self.project_root(project).join("Downloads")
    }

    pub fn project_releases(&self, project: &str) -> PathBuf {
        self.project_root(project).join("Releases")
    }

    pub fn project_restored(&self, project: &str) -> PathBuf {
        self.project_root(project).join("Restored")
    }

    pub fn project_cold(&self, project: &str) -> PathBuf {
        self.project_root(project).join("Cold")
    }

    pub fn category_cold(&self, category: Category) -> PathBuf {
        self.root.join("ColdStorage").join(category.dir_name())
    }

    pub fn coldpack_store(&self) -> PathBuf {
        self.root.join("ColdStorage").join(".coldpack-store")
    }

    pub fn category_restored(&self, category: Category) -> PathBuf {
        self.root.join("Restored").join(category.dir_name())
    }

    pub fn registry_path(&self) -> PathBuf {
        self.root.join("registry.tsv")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildIdentity {
    pub project: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ClassifiedDownload {
    pub category: Category,
    pub project: Option<String>,
    pub build: Option<BuildIdentity>,
    pub is_project_tree: bool,
}

#[derive(Debug, Clone, Default)]
pub struct OrganizerReport {
    pub activated_projects: usize,
    pub sorted_entries: usize,
    pub cold_archives: usize,
    pub skipped_entries: usize,
    pub actions: Vec<String>,
    pub issues: Vec<String>,
}

impl OrganizerReport {
    pub fn is_ok(&self) -> bool {
        self.issues.is_empty()
    }
}

pub fn classify_download(path: &Path) -> ClassifiedDownload {
    let is_project_tree = path.is_dir() && is_project_directory(path);
    let build = if is_project_tree {
        None
    } else {
        build_identity_from_entry(path)
    };
    let project = build
        .as_ref()
        .map(|identity| identity.project.clone())
        .or_else(|| {
            project_name_from_entry(path).or_else(|| {
                if is_project_tree {
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .map(clean_project_name)
                } else {
                    None
                }
            })
        });
    let category = if is_project_tree {
        Category::Archives
    } else {
        category_from_path(path)
    };
    ClassifiedDownload {
        category,
        project,
        build,
        is_project_tree,
    }
}

pub fn is_project_directory(path: &Path) -> bool {
    path.join("Cargo.toml").is_file()
        || path.join("CMakeLists.txt").is_file()
        || path.join("meson.build").is_file()
        || path.join("Makefile").is_file()
        || path.join("package.json").is_file()
}

pub fn build_identity_from_entry(path: &Path) -> Option<BuildIdentity> {
    let name = path.file_name()?.to_str()?;
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".part") || lower.ends_with(".partial") || lower.ends_with(".tmp") {
        return None;
    }
    let bytes = name.as_bytes();
    for separator in 1..bytes.len() {
        if bytes[separator] != b'-' {
            continue;
        }
        let start = separator + 1;
        if start >= bytes.len() {
            continue;
        }
        let mut cursor = start;
        let has_v = matches!(bytes[cursor], b'v' | b'V');
        if has_v {
            cursor += 1;
        }
        let first_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == first_start {
            continue;
        }

        let mut components = vec![&name[first_start..cursor]];
        loop {
            if cursor >= bytes.len() || !matches!(bytes[cursor], b'.' | b'_') {
                break;
            }
            let digits_start = cursor + 1;
            if digits_start >= bytes.len() || !bytes[digits_start].is_ascii_digit() {
                break;
            }
            cursor = digits_start;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            components.push(&name[digits_start..cursor]);
        }
        if components.len() < 2 {
            continue;
        }
        if cursor < bytes.len()
            && bytes[cursor] != b'-'
            && bytes[cursor] != b'.'
            && bytes[cursor] != b'('
        {
            continue;
        }

        let project = clean_project_name(&name[..separator]);
        if project.is_empty() {
            continue;
        }
        let normalized = components.join(".");
        let version = if has_v {
            format!("v{normalized}")
        } else {
            normalized
        };
        return Some(BuildIdentity { project, version });
    }
    None
}

pub fn project_name_from_entry(path: &Path) -> Option<String> {
    if let Some(identity) = build_identity_from_entry(path) {
        return Some(identity.project);
    }
    let name = path.file_name()?.to_str()?;
    let mut base = name.to_owned();
    for suffix in [
        ".tar.zst", ".tar.gz", ".tar.xz", ".tar.bz2", ".zip", ".7z", ".rar", ".tgz", ".zst", ".txt",
    ] {
        if base.to_ascii_lowercase().ends_with(suffix) {
            base.truncate(base.len() - suffix.len());
            break;
        }
    }
    let bytes = base.as_bytes();
    let mut cut = None;
    for i in 1..bytes.len().saturating_sub(1) {
        if bytes[i] != b'-' {
            continue;
        }
        let rest = &base[i + 1..];
        if rest.starts_with('v') && rest[1..].chars().next().is_some_and(|c| c.is_ascii_digit()) {
            cut = Some(i);
            break;
        }
        if rest.chars().next().is_some_and(|c| c.is_ascii_digit()) && rest.contains('.') {
            cut = Some(i);
            break;
        }
    }
    let candidate = cut.map(|i| &base[..i]).unwrap_or(&base);
    if candidate.is_empty() || (cut.is_none() && !candidate.contains('-')) {
        None
    } else {
        Some(clean_project_name(candidate))
    }
}

fn clean_project_name(input: &str) -> String {
    input
        .trim_matches(|c: char| c == '-' || c == '_' || c.is_whitespace())
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn category_from_path(path: &Path) -> Category {
    let lower = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if lower.ends_with(".pkg.tar.zst") || lower.ends_with(".pkg.tar.xz") || lower.ends_with(".sig")
    {
        return Category::Packages;
    }
    if lower.ends_with(".part") || lower.ends_with(".partial") || lower.ends_with(".tmp") {
        return Category::Temporary;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "txt" | "md" | "pdf" | "doc" | "docx" | "odt" | "rtf" => Category::Documents,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "heic" => Category::Images,
        "mp4" | "mkv" | "webm" | "mov" | "avi" => Category::Video,
        "mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac" => Category::Audio,
        "zip" | "7z" | "rar" | "gz" | "xz" | "zst" | "bz2" | "tgz" => Category::Archives,
        "log" => Category::Logs,
        "iso" | "img" | "appimage" | "exe" | "msi" | "apk" | "sh" | "bash" | "fish" | "run" => {
            Category::Installers
        }
        _ => Category::Other,
    }
}

pub fn entry_is_stable(path: &Path, stable_for: Duration) -> Result<bool, String> {
    let newest = newest_modified(path)?;
    let age = SystemTime::now()
        .duration_since(newest)
        .unwrap_or(Duration::ZERO);
    Ok(age >= stable_for)
}

fn newest_modified(path: &Path) -> Result<SystemTime, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err(format!(
            "refusing symlink download entry: {}",
            path.display()
        ));
    }
    let mut newest = meta.modified().map_err(|e| e.to_string())?;
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let child = entry.map_err(|e| e.to_string())?.path();
            let child_modified = newest_modified(&child)?;
            if child_modified > newest {
                newest = child_modified;
            }
        }
    }
    Ok(newest)
}

pub fn sorted_destination(
    layout: &ForgeLayout,
    source: &Path,
    classified: &ClassifiedDownload,
) -> Result<PathBuf, String> {
    let name = source
        .file_name()
        .ok_or_else(|| "download entry has no file name".to_owned())?;
    if let Some(build) = &classified.build {
        return Ok(layout
            .project_build(&build.project, &build.version)
            .join(name));
    }
    if let Some(project) = &classified.project {
        let base = if looks_like_release_artifact(source) {
            layout.project_releases(project)
        } else {
            layout.project_downloads(project)
        };
        Ok(base.join(name))
    } else {
        Ok(layout.root.join(classified.category.dir_name()).join(name))
    }
}

pub fn move_without_overwrite(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() || fs::symlink_metadata(destination).is_ok() {
        return Err(format!(
            "destination already exists: {}",
            destination.display()
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| "destination has no parent".to_owned())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    fs::rename(source, destination).map_err(|e| {
        format!(
            "cannot move {} -> {}: {e}",
            source.display(),
            destination.display()
        )
    })
}

pub fn legacy_build_artifact_candidates(layout: &ForgeLayout) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    for category in [
        Category::Packages,
        Category::Documents,
        Category::Images,
        Category::Video,
        Category::Audio,
        Category::Archives,
        Category::Logs,
        Category::Installers,
        Category::Temporary,
        Category::Other,
    ] {
        let root = layout.root.join(category.dir_name());
        if root.is_dir() {
            roots.push(root);
        }
    }

    let projects = layout.root.join("Projects");
    if projects.is_dir() {
        for entry in fs::read_dir(&projects).map_err(|e| e.to_string())? {
            let project = entry.map_err(|e| e.to_string())?.path();
            let meta = match fs::symlink_metadata(&project) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if meta.file_type().is_symlink() || !meta.is_dir() {
                continue;
            }
            for legacy_dir in [project.join("Downloads"), project.join("Releases")] {
                if legacy_dir.is_dir() {
                    roots.push(legacy_dir);
                }
            }
        }
    }

    let mut candidates = Vec::new();
    for root in roots {
        collect_legacy_build_artifacts(&root, &mut candidates)?;
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn collect_legacy_build_artifacts(
    root: &Path,
    candidates: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|e| format!("cannot scan {}: {e}", root.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        let meta = match fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if build_identity_from_entry(&path).is_some() {
            candidates.push(path);
            continue;
        }
        if meta.is_dir() {
            collect_legacy_build_artifacts(&path, candidates)?;
        }
    }
    Ok(())
}

fn reconcile_existing_build_artifacts(
    layout: &ForgeLayout,
    stable_for: Duration,
    create_aliases: bool,
    report: &mut OrganizerReport,
) {
    let candidates = match legacy_build_artifact_candidates(layout) {
        Ok(value) => value,
        Err(error) => {
            report.issues.push(error);
            return;
        }
    };

    for source in candidates {
        match entry_is_stable(&source, stable_for) {
            Ok(true) => {}
            Ok(false) => {
                report.skipped_entries += 1;
                continue;
            }
            Err(error) => {
                report.issues.push(error);
                continue;
            }
        }
        let classified = classify_download(&source);
        if classified.build.is_none() {
            continue;
        }
        match route_build_artifact(layout, &source, &classified, create_aliases) {
            Ok(Action::RoutedBuild(message)) => {
                report.sorted_entries += 1;
                report.actions.push(format!("RECONCILED_OLD_{message}"));
            }
            Ok(_) => report.issues.push(format!(
                "unexpected reconciliation action for {}",
                source.display()
            )),
            Err(error) => report.issues.push(error),
        }
    }
}

pub fn top_level_downloads(layout: &ForgeLayout) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(&layout.downloads).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path == layout.root {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        let meta = match fs::symlink_metadata(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        paths.push(path);
    }
    paths.sort();
    Ok(paths)
}

pub fn organize_once(layout: &ForgeLayout, stable_for: Duration) -> OrganizerReport {
    organize_once_with_alias_policy(layout, stable_for, true)
}

pub fn organize_once_with_alias_policy(
    layout: &ForgeLayout,
    stable_for: Duration,
    create_aliases: bool,
) -> OrganizerReport {
    let mut report = OrganizerReport::default();
    if let Err(error) = layout.ensure() {
        report.issues.push(error);
        return report;
    }
    let mut registry = match ProjectRegistry::read(&layout.registry_path()) {
        Ok(value) => value,
        Err(error) => {
            report.issues.push(error);
            return report;
        }
    };
    reconcile_existing_build_artifacts(layout, stable_for, create_aliases, &mut report);

    let entries = match top_level_downloads(layout) {
        Ok(value) => value,
        Err(error) => {
            report.issues.push(error);
            return report;
        }
    };

    for source in entries {
        match entry_is_stable(&source, stable_for) {
            Ok(true) => {}
            Ok(false) => {
                report.skipped_entries += 1;
                continue;
            }
            Err(error) => {
                report.issues.push(error);
                continue;
            }
        }
        let classified = classify_download(&source);
        let action = if classified.is_project_tree {
            activate_downloaded_project(layout, &mut registry, &source, &classified, create_aliases)
        } else if classified.build.is_some() {
            route_build_artifact(layout, &source, &classified, create_aliases)
        } else {
            sort_and_archive_download(layout, &source, &classified)
        };
        match action {
            Ok(Action::Activated(message)) => {
                report.activated_projects += 1;
                report.actions.push(message);
            }
            Ok(Action::RoutedBuild(message)) => {
                report.sorted_entries += 1;
                report.actions.push(message);
            }
            Ok(Action::Archived(message)) => {
                report.sorted_entries += 1;
                report.cold_archives += 1;
                report.actions.push(message);
            }
            Err(error) => report.issues.push(error),
        }
    }

    if let Err(error) = registry.write(&layout.registry_path()) {
        report.issues.push(error);
    }
    report
}

enum Action {
    Activated(String),
    RoutedBuild(String),
    Archived(String),
}

fn activate_downloaded_project(
    layout: &ForgeLayout,
    registry: &mut ProjectRegistry,
    source: &Path,
    classified: &ClassifiedDownload,
    create_aliases: bool,
) -> Result<Action, String> {
    let project = classified
        .project
        .as_deref()
        .ok_or_else(|| format!("project tree has no project name: {}", source.display()))?;
    let project_root = layout.project_root(project);
    fs::create_dir_all(&project_root).map_err(|e| e.to_string())?;
    fs::create_dir_all(layout.project_builds(project)).map_err(|e| e.to_string())?;
    fs::create_dir_all(layout.project_downloads(project)).map_err(|e| e.to_string())?;
    fs::create_dir_all(layout.project_releases(project)).map_err(|e| e.to_string())?;
    fs::create_dir_all(layout.project_restored(project)).map_err(|e| e.to_string())?;
    fs::create_dir_all(layout.project_cold(project)).map_err(|e| e.to_string())?;
    let active = layout.active_dir(project);

    if active.exists() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        archive_to_coldpack(
            &active,
            &layout.project_cold(project),
            &layout.coldpack_store(),
            &format!("Active-{stamp}"),
        )?;
    }

    let legacy = source.to_path_buf();
    move_without_overwrite(source, &active)?;
    if create_aliases {
        create_legacy_alias(&layout.downloads, &legacy, &active)?;
    }
    registry.set_active(
        project,
        active.clone(),
        if create_aliases {
            vec![legacy.clone()]
        } else {
            Vec::new()
        },
    );
    Ok(Action::Activated(format!(
        "PROJECT_ACTIVATED={}\t{}\tLEGACY={}",
        project,
        active.display(),
        legacy.display()
    )))
}

fn create_build_artifact_alias(
    layout: &ForgeLayout,
    legacy: &Path,
    canonical: &Path,
) -> Result<(), String> {
    create_legacy_alias(&layout.downloads, legacy, canonical)
}

fn route_build_artifact(
    layout: &ForgeLayout,
    source: &Path,
    classified: &ClassifiedDownload,
    create_aliases: bool,
) -> Result<Action, String> {
    let build = classified
        .build
        .as_ref()
        .ok_or_else(|| format!("build artifact has no identity: {}", source.display()))?;
    let destination = sorted_destination(layout, source, classified)?;
    move_without_overwrite(source, &destination)?;
    if create_aliases && let Err(error) = create_build_artifact_alias(layout, source, &destination)
    {
        match fs::rename(&destination, source) {
            Ok(()) => return Err(error),
            Err(rollback_error) => {
                return Err(format!(
                    "{error}; rollback failed {} -> {}: {rollback_error}",
                    destination.display(),
                    source.display()
                ));
            }
        }
    }
    Ok(Action::RoutedBuild(format!(
        "BUILD_ARTIFACT_ROUTED={}\tVERSION={}\t{}\tLEGACY={}",
        build.project,
        build.version,
        destination.display(),
        source.display()
    )))
}

fn sort_and_archive_download(
    layout: &ForgeLayout,
    source: &Path,
    classified: &ClassifiedDownload,
) -> Result<Action, String> {
    let destination = sorted_destination(layout, source, classified)?;
    move_without_overwrite(source, &destination)?;
    let cold_dir = classified
        .project
        .as_deref()
        .map(|project| layout.project_cold(project))
        .unwrap_or_else(|| layout.category_cold(classified.category));
    let label = destination
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download");
    let record = archive_to_coldpack(&destination, &cold_dir, &layout.coldpack_store(), label)?;
    Ok(Action::Archived(format!(
        "DOWNLOAD_ARCHIVED={}\t{}",
        record.source.display(),
        record.archive_path.display()
    )))
}

fn looks_like_release_artifact(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    name.contains("VERIFY")
        || name.contains("SHA256")
        || name.contains("PACKAGE")
        || name.contains("SOURCE")
        || name.contains("RELEASE")
}
