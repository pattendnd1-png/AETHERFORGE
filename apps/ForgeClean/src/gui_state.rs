use crate::coldpack_gc::{ColdPackStoreReport, coldpack_status};
use crate::organizer::ForgeLayout;
use crate::storage::detect_external_drives;
use crate::system_scan::SYSTEM_PACMAN_CACHE;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub const GUI_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_STABLE_SECONDS: u64 = 30;
pub const DEFAULT_POLL_SECONDS: u64 = 2;
pub const DEFAULT_MAINTENANCE_HOURS: u64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuiSettings {
    pub organizer_enabled: bool,
    pub stable_seconds: u64,
    pub poll_seconds: u64,
    pub coldpack_maintenance_enabled: bool,
    pub coldpack_maintenance_hours: u64,
    pub notifications_enabled: bool,
    pub compatibility_symlinks: bool,
    pub diagnostics_visible: bool,
    pub logging_level: String,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            organizer_enabled: true,
            stable_seconds: DEFAULT_STABLE_SECONDS,
            poll_seconds: DEFAULT_POLL_SECONDS,
            coldpack_maintenance_enabled: true,
            coldpack_maintenance_hours: DEFAULT_MAINTENANCE_HOURS,
            notifications_enabled: true,
            compatibility_symlinks: true,
            diagnostics_visible: true,
            logging_level: "normal".to_owned(),
        }
    }
}

impl GuiSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.poll_seconds == 0 {
            return Err("poll_seconds must be at least 1".to_owned());
        }
        if self.coldpack_maintenance_hours == 0 {
            return Err("coldpack_maintenance_hours must be at least 1".to_owned());
        }
        if !matches!(self.logging_level.as_str(), "quiet" | "normal" | "verbose") {
            return Err("logging_level must be quiet, normal, or verbose".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildSnapshot {
    pub version: String,
    pub path: PathBuf,
    pub files: Vec<PathBuf>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectSnapshot {
    pub name: String,
    pub path: PathBuf,
    pub active_path: PathBuf,
    pub active_exists: bool,
    pub builds: Vec<BuildSnapshot>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InboxItem {
    pub path: PathBuf,
    pub incomplete: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DashboardSnapshot {
    pub service_active: bool,
    pub service_enabled: bool,
    pub projects: Vec<ProjectSnapshot>,
    pub inbox: Vec<InboxItem>,
    pub coldpack: Option<ColdPackStoreReport>,
    pub coldpack_error: Option<String>,
    pub external_drives: usize,
    pub storage_mode: String,
    pub organizer_root: PathBuf,
    pub registry_path: PathBuf,
    pub coldpack_store: PathBuf,
    pub activity: Vec<String>,
}

impl DashboardSnapshot {
    pub fn project_count(&self) -> usize {
        self.projects.len()
    }

    pub fn build_count(&self) -> usize {
        self.projects
            .iter()
            .map(|project| project.builds.len())
            .sum()
    }

    pub fn build_file_count(&self) -> usize {
        self.projects
            .iter()
            .flat_map(|project| &project.builds)
            .map(|build| build.files.len())
            .sum()
    }
}

#[derive(Debug, Clone)]
pub enum CoreAction {
    OrganizeNow,
    ScanSystem,
    StorageStatus,
    ColdpackAudit,
    ColdpackGcPreview,
    ColdpackGcApply,
    AutoCleanupWithPrivilege,
    RestartService,
    SetServiceEnabled(bool),
    OpenPath(PathBuf),
    RunArtifact(PathBuf),
}

#[derive(Debug, Clone, Default)]
pub struct ActionResult {
    pub label: String,
    pub success: bool,
    pub output: String,
}

pub fn settings_path() -> Result<PathBuf, String> {
    if let Ok(config) = env::var("XDG_CONFIG_HOME")
        && !config.trim().is_empty()
    {
        return Ok(PathBuf::from(config).join("forgeclean/gui-settings.json"));
    }
    Ok(user_home()?.join(".config/forgeclean/gui-settings.json"))
}

pub fn load_settings() -> Result<GuiSettings, String> {
    load_settings_from(&settings_path()?)
}

pub fn load_settings_from(path: &Path) -> Result<GuiSettings, String> {
    if !path.exists() {
        return Ok(GuiSettings::default());
    }
    let bytes =
        fs::read(path).map_err(|e| format!("cannot read settings {}: {e}", path.display()))?;
    let settings: GuiSettings = serde_json::from_slice(&bytes)
        .map_err(|e| format!("cannot parse settings {}: {e}", path.display()))?;
    settings.validate()?;
    Ok(settings)
}

pub fn save_settings(settings: &GuiSettings) -> Result<PathBuf, String> {
    let path = settings_path()?;
    save_settings_to(&path, settings)?;
    Ok(path)
}

pub fn save_settings_to(path: &Path, settings: &GuiSettings) -> Result<(), String> {
    settings.validate()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create settings directory {}: {e}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&tmp, bytes).map_err(|e| format!("cannot write settings {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path)
        .map_err(|e| format!("cannot commit settings {}: {e}", path.display()))?;
    Ok(())
}

pub fn collect_project_snapshots(layout: &ForgeLayout) -> Result<Vec<ProjectSnapshot>, String> {
    let projects_root = layout.root.join("Projects");
    if !projects_root.exists() {
        return Ok(Vec::new());
    }
    let mut projects = Vec::new();
    for project_path in sorted_children(&projects_root)? {
        let meta = fs::symlink_metadata(&project_path).map_err(|e| e.to_string())?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            continue;
        }
        let Some(name) = project_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let active_path = project_path.join("Active");
        let builds_root = project_path.join("Builds");
        let mut builds = Vec::new();
        if builds_root.is_dir() {
            for build_path in sorted_children(&builds_root)? {
                let build_meta = fs::symlink_metadata(&build_path).map_err(|e| e.to_string())?;
                if !build_meta.is_dir() || build_meta.file_type().is_symlink() {
                    continue;
                }
                let Some(version) = build_path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };
                let mut files = Vec::new();
                let mut total_bytes = 0u64;
                for file in sorted_children(&build_path)? {
                    let file_meta = fs::symlink_metadata(&file).map_err(|e| e.to_string())?;
                    if file_meta.is_file() {
                        total_bytes = total_bytes.saturating_add(file_meta.len());
                        files.push(file);
                    }
                }
                builds.push(BuildSnapshot {
                    version: version.to_owned(),
                    path: build_path,
                    files,
                    total_bytes,
                });
            }
        }
        builds.sort_by_key(|build| std::cmp::Reverse(build.version.clone()));
        projects.push(ProjectSnapshot {
            name: name.to_owned(),
            path: project_path,
            active_exists: active_path.is_dir(),
            active_path,
            builds,
        });
    }
    projects.sort_by_key(|project| project.name.to_ascii_lowercase());
    Ok(projects)
}

pub fn collect_dashboard_snapshot(downloads: &Path) -> Result<DashboardSnapshot, String> {
    let layout = ForgeLayout::new(downloads);
    layout.ensure()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let (coldpack, coldpack_error) =
        match coldpack_status(&layout.root, &layout.coldpack_store(), now) {
            Ok(report) => (Some(report), None),
            Err(error) => (None, Some(error)),
        };
    let external_drives = detect_external_drives(Path::new(SYSTEM_PACMAN_CACHE));
    let (external_count, storage_mode) = match external_drives {
        Ok(drives) if drives.is_empty() => (0, "AUTO_CLEAN".to_owned()),
        Ok(drives) => (drives.len(), "AUTO_OFFLOAD".to_owned()),
        Err(error) => (0, format!("DETECTION_BLOCKED: {error}")),
    };
    Ok(DashboardSnapshot {
        service_active: systemctl_check("is-active"),
        service_enabled: systemctl_check("is-enabled"),
        projects: collect_project_snapshots(&layout)?,
        inbox: collect_inbox(downloads, &layout)?,
        coldpack,
        coldpack_error,
        external_drives: external_count,
        storage_mode,
        organizer_root: layout.root.clone(),
        registry_path: layout.registry_path(),
        coldpack_store: layout.coldpack_store(),
        activity: recent_activity(),
    })
}

pub fn run_core_action(action: CoreAction, downloads: &Path) -> ActionResult {
    match action {
        CoreAction::OrganizeNow => {
            let downloads = downloads.to_string_lossy().into_owned();
            run_forgeclean(
                "Organize now",
                &["organize-once", "--downloads", &downloads],
            )
        }
        CoreAction::ScanSystem => run_forgeclean("Cleanup preview", &["scan-system"]),
        CoreAction::StorageStatus => run_forgeclean("Storage status", &["storage-status"]),
        CoreAction::ColdpackAudit => {
            run_forgeclean_with_downloads("ColdPack audit", "coldpack-audit", downloads)
        }
        CoreAction::ColdpackGcPreview => {
            let downloads = downloads.to_string_lossy().into_owned();
            run_forgeclean_args(
                "ColdPack GC preview",
                &["coldpack-gc", "--preview", "--downloads", &downloads],
            )
            .with_suffix_if_success("\nUse Apply only after reviewing this preview.")
        }
        CoreAction::ColdpackGcApply => {
            let downloads = downloads.to_string_lossy().into_owned();
            run_forgeclean_args(
                "ColdPack GC apply",
                &["coldpack-gc", "--apply", "--downloads", &downloads],
            )
        }
        CoreAction::AutoCleanupWithPrivilege => run_privileged_cleanup(),
        CoreAction::RestartService => run_command(
            "Restart organizer service",
            Command::new("systemctl").args(["--user", "restart", "forgeclean-organizer.service"]),
        ),
        CoreAction::SetServiceEnabled(enabled) => {
            let verb = if enabled { "enable" } else { "disable" };
            run_command(
                if enabled {
                    "Enable organizer service"
                } else {
                    "Disable organizer service"
                },
                Command::new("systemctl").args([
                    "--user",
                    verb,
                    "--now",
                    "forgeclean-organizer.service",
                ]),
            )
        }
        CoreAction::OpenPath(path) => {
            run_command("Open folder", Command::new("xdg-open").arg(path))
        }
        CoreAction::RunArtifact(path) => run_artifact(&path),
    }
}

trait ActionResultExt {
    fn with_suffix_if_success(self, suffix: &str) -> Self;
}

impl ActionResultExt for ActionResult {
    fn with_suffix_if_success(mut self, suffix: &str) -> Self {
        if self.success {
            self.output.push_str(suffix);
        }
        self
    }
}

fn run_forgeclean_with_downloads(label: &str, command: &str, downloads: &Path) -> ActionResult {
    let downloads = downloads.to_string_lossy().into_owned();
    run_forgeclean_args(label, &[command, "--downloads", &downloads])
}

fn run_forgeclean(label: &str, args: &[&str]) -> ActionResult {
    run_forgeclean_args(label, args)
}

fn run_forgeclean_args(label: &str, args: &[&str]) -> ActionResult {
    let bin = forgeclean_binary();
    let mut command = Command::new(&bin);
    command.args(args);
    run_command(label, &mut command)
}

fn run_privileged_cleanup() -> ActionResult {
    let bin = forgeclean_binary();
    let mut command = Command::new("pkexec");
    command.arg(bin).args(["auto", "--yes"]);
    run_command("Automatic cleanup/offload", &mut command)
}

fn run_artifact(path: &Path) -> ActionResult {
    let lower = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut command = if lower.ends_with(".sh") || lower.ends_with(".bash") {
        let mut command = Command::new("/usr/bin/bash");
        command.arg(path);
        command
    } else if lower.ends_with(".fish") {
        let mut command = Command::new("/usr/bin/fish");
        command.arg(path);
        command
    } else {
        Command::new(path)
    };
    if let Some(parent) = path.parent() {
        command.current_dir(parent);
    }
    run_command("Run build/install artifact", &mut command)
}

fn run_command(label: &str, command: &mut Command) -> ActionResult {
    match command.output() {
        Ok(output) => output_result(label, output),
        Err(error) => ActionResult {
            label: label.to_owned(),
            success: false,
            output: error.to_string(),
        },
    }
}

fn output_result(label: &str, output: Output) -> ActionResult {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&stderr);
    }
    ActionResult {
        label: label.to_owned(),
        success: output.status.success(),
        output: text,
    }
}

fn forgeclean_binary() -> PathBuf {
    if let Ok(bin) = env::var("FORGECLEAN_BIN")
        && !bin.trim().is_empty()
    {
        return PathBuf::from(bin);
    }
    if let Ok(exe) = env::current_exe() {
        let sibling = exe.with_file_name("forgeclean");
        if sibling.is_file() {
            return sibling;
        }
    }
    if let Ok(home) = user_home() {
        let installed = home.join(".local/bin/forgeclean");
        if installed.is_file() {
            return installed;
        }
    }
    PathBuf::from("forgeclean")
}

fn collect_inbox(downloads: &Path, layout: &ForgeLayout) -> Result<Vec<InboxItem>, String> {
    let mut items = Vec::new();
    if downloads.is_dir() {
        for path in sorted_children(downloads)? {
            if path == layout.root
                || fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink())
            {
                continue;
            }
            items.push(InboxItem {
                incomplete: is_incomplete(&path),
                path,
            });
        }
    }
    let temporary = layout.root.join("Temporary");
    if temporary.is_dir() {
        for path in sorted_children(&temporary)? {
            if fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
                continue;
            }
            items.push(InboxItem {
                incomplete: is_incomplete(&path),
                path,
            });
        }
    }
    items.sort_by_key(|item| item.path.clone());
    Ok(items)
}

fn is_incomplete(path: &Path) -> bool {
    let lower = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    lower.ends_with(".part") || lower.ends_with(".partial") || lower.ends_with(".tmp")
}

fn sorted_children(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path).map_err(|e| format!("cannot read {}: {e}", path.display()))? {
        entries.push(entry.map_err(|e| e.to_string())?.path());
    }
    entries.sort();
    Ok(entries)
}

fn systemctl_check(verb: &str) -> bool {
    Command::new("systemctl")
        .args(["--user", verb, "--quiet", "forgeclean-organizer.service"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn recent_activity() -> Vec<String> {
    let output = Command::new("journalctl")
        .args([
            "--user",
            "-u",
            "forgeclean-organizer.service",
            "-n",
            "40",
            "--no-pager",
            "-o",
            "cat",
        ])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn user_home() -> Result<PathBuf, String> {
    env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME is not set".to_owned())
}
