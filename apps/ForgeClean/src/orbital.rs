use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub const DEFAULT_GITHUB_REPO: &str = "pattendnd1-png/AETHERFORGE";
pub const DEFAULT_RELEASE_TAG: &str = "wip-post-reset";
pub const ORBIT_TIMER: &str = "aetherforge-orbital-sync.timer";
pub const ORBIT_SERVICE: &str = "aetherforge-orbital-sync.service";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitalStatus {
    pub fields: BTreeMap<String, String>,
}

impl OrbitalStatus {
    pub fn get(&self, key: &str) -> &str {
        self.fields.get(key).map(String::as_str).unwrap_or("")
    }
}

pub fn parse_status(text: &str) -> OrbitalStatus {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || key.starts_with("AETHERFORGE_") {
            continue;
        }
        fields.insert(key.to_string(), value.trim().to_string());
    }
    OrbitalStatus { fields }
}

pub fn status_path(home: &Path) -> PathBuf {
    home.join("Downloads/AETHERFORGE-ORBITAL-SYNC-STATUS.txt")
}

pub fn read_status(home: &Path) -> io::Result<OrbitalStatus> {
    fs::read_to_string(status_path(home)).map(|text| parse_status(&text))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrbitalAction {
    Status,
    Sync,
    Full,
    Retry,
    Pause,
    Resume,
    Diagnostic,
}

impl OrbitalAction {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "status" => Some(Self::Status),
            "sync" | "now" => Some(Self::Sync),
            "full" => Some(Self::Full),
            "retry" => Some(Self::Retry),
            "pause" => Some(Self::Pause),
            "resume" => Some(Self::Resume),
            "diagnostic" | "diag" => Some(Self::Diagnostic),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationAction {
    Status,
    Scan,
    Upload,
    Verify,
    Restore,
    Sync,
    Inventory,
    Stage,
}

impl MigrationAction {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "status" => Some(Self::Status),
            "scan" => Some(Self::Scan),
            "upload" => Some(Self::Upload),
            "verify" => Some(Self::Verify),
            "restore" => Some(Self::Restore),
            "sync" => Some(Self::Sync),
            "inventory" => Some(Self::Inventory),
            "stage" => Some(Self::Stage),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageSummary {
    pub repo: String,
    pub release_tag: String,
    pub indexed_artifacts: usize,
    pub externalized_large_files: usize,
    pub restore_helper_present: bool,
}

impl StorageSummary {
    pub fn to_text(&self) -> String {
        format!(
            "FORGECLEAN_GH_EXTERNAL_STORAGE=PASS\nrepo={}\nrelease_tag={}\nindexed_artifacts={}\nexternalized_large_files={}\nrestore_helper_present={}\n",
            self.repo,
            self.release_tag,
            self.indexed_artifacts,
            self.externalized_large_files,
            self.restore_helper_present
        )
    }
}

fn non_header_lines(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|text| {
            text.lines()
                .skip(1)
                .filter(|line| !line.trim().is_empty())
                .count()
        })
        .unwrap_or(0)
}

pub fn local_storage_summary(home: &Path) -> StorageSummary {
    let root = home.join("Downloads/AETHERFORGE");
    StorageSummary {
        repo: std::env::var("AETHERFORGE_GITHUB_REPO")
            .unwrap_or_else(|_| DEFAULT_GITHUB_REPO.to_string()),
        release_tag: std::env::var("AETHERFORGE_RELEASE_TAG")
            .unwrap_or_else(|_| DEFAULT_RELEASE_TAG.to_string()),
        indexed_artifacts: non_header_lines(&root.join("artifacts/INDEX.tsv")),
        externalized_large_files: non_header_lines(&root.join("artifacts/LARGE-FILES.tsv")),
        restore_helper_present: root.join("artifacts/restore-large-assets.sh").is_file(),
    }
}

fn command_output(mut command: Command) -> Result<Output, Box<dyn Error>> {
    Ok(command.output()?)
}

fn output_text(output: Output, label: &str) -> Result<String, Box<dyn Error>> {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() {
        Ok(if stdout.trim().is_empty() {
            stderr
        } else {
            stdout
        })
    } else {
        Err(format!("{label} failed: {}", stderr.trim()).into())
    }
}

fn run_systemctl(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut command = Command::new("systemctl");
    command.arg("--user").args(args);
    output_text(command_output(command)?, "systemctl")
}

fn orbit_worker(home: &Path) -> PathBuf {
    home.join(".local/bin/aetherforge-orbital-sync")
}

fn run_worker(home: &Path, action: &str) -> Result<String, Box<dyn Error>> {
    let worker = orbit_worker(home);
    if !worker.is_file() {
        return Err(format!("orbital worker missing: {}", worker.display()).into());
    }
    let mut command = Command::new(worker);
    command.arg(action);
    output_text(command_output(command)?, "orbital worker")
}

pub fn run_orbital_action(home: &Path, action: OrbitalAction) -> Result<String, Box<dyn Error>> {
    match action {
        OrbitalAction::Status => match run_worker(home, "status") {
            Ok(text) => Ok(text),
            Err(_) => Ok(fs::read_to_string(status_path(home))?),
        },
        OrbitalAction::Sync => run_worker(home, "orbit"),
        OrbitalAction::Full => run_worker(home, "full"),
        OrbitalAction::Retry => run_worker(home, "fast"),
        OrbitalAction::Pause => {
            let _ = run_systemctl(&["disable", "--now", ORBIT_TIMER])?;
            Ok("FORGECLEAN_ORBITAL_PAUSE=PASS\n".to_string())
        }
        OrbitalAction::Resume => {
            let _ = run_systemctl(&["daemon-reload"])?;
            let _ = run_systemctl(&["enable", "--now", ORBIT_TIMER])?;
            Ok("FORGECLEAN_ORBITAL_RESUME=PASS\n".to_string())
        }
        OrbitalAction::Diagnostic => {
            let mut out = String::new();
            out.push_str("=== ORBITAL STATUS ===\n");
            out.push_str(
                &run_orbital_action(home, OrbitalAction::Status).unwrap_or_else(|e| e.to_string()),
            );
            out.push_str("\n=== TIMER ===\n");
            out.push_str(
                &run_systemctl(&["status", ORBIT_TIMER, "--no-pager"])
                    .unwrap_or_else(|e| e.to_string()),
            );
            out.push_str("\n=== SERVICE ===\n");
            out.push_str(
                &run_systemctl(&["status", ORBIT_SERVICE, "--no-pager"])
                    .unwrap_or_else(|e| e.to_string()),
            );
            Ok(out)
        }
    }
}

fn repo_root(home: &Path) -> PathBuf {
    home.join("Downloads/AETHERFORGE")
}

pub fn latest_forgeclean_source(home: &Path) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("FORGECLEAN_SOURCE_ROOT").map(PathBuf::from) {
        return path.is_dir().then_some(path);
    }
    let downloads = home.join("Downloads");
    let mut best: Option<([u64; 3], PathBuf)> = None;
    let entries = fs::read_dir(downloads).ok()?;
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(version) = name.strip_prefix("ForgeClean-v") else {
            continue;
        };
        let values: Vec<_> = version
            .split('.')
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default();
        if values.len() != 3 || !entry.path().join("Cargo.toml").is_file() {
            continue;
        }
        let parsed = [values[0], values[1], values[2]];
        if best.as_ref().map(|(v, _)| parsed > *v).unwrap_or(true) {
            best = Some((parsed, entry.path()));
        }
    }
    best.map(|(_, path)| path)
}

fn sync_forgeclean_source(home: &Path) -> Result<(), Box<dyn Error>> {
    let source = latest_forgeclean_source(home)
        .ok_or_else(|| "no ForgeClean-vX.Y.Z source tree found in ~/Downloads".to_string())?;
    let destination = repo_root(home).join("apps/ForgeClean");
    fs::create_dir_all(&destination)?;
    let source_arg = format!("{}/", source.display());
    let destination_arg = format!("{}/", destination.display());
    let mut command = Command::new("rsync");
    command
        .args(["-a", "--delete-delay"])
        .args(["--exclude=.git/", "--exclude=target/", "--exclude=.cache/"])
        .arg(source_arg)
        .arg(destination_arg);
    output_text(command_output(command)?, "ForgeClean source sync")?;
    Ok(())
}

fn git_capture(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git").arg("-C").arg(root).args(args).output();
    match output {
        Ok(value) if value.status.success() => {
            String::from_utf8_lossy(&value.stdout).trim().to_string()
        }
        _ => "UNAVAILABLE".to_string(),
    }
}

pub fn migration_status(home: &Path) -> String {
    let root = repo_root(home);
    let summary = local_storage_summary(home);
    let local_head = git_capture(&root, &["rev-parse", "HEAD"]);
    let remote_head = git_capture(&root, &["rev-parse", "origin/main"]);
    let dirty = git_capture(&root, &["status", "--porcelain"])
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    format!(
        "{}local_head={}\nremote_head={}\ndirty_files={}\nexternal_storage=github\n",
        summary.to_text(),
        local_head,
        remote_head,
        dirty
    )
}

fn verify_external_storage(home: &Path) -> Result<String, Box<dyn Error>> {
    let root = repo_root(home);
    let repo = std::env::var("AETHERFORGE_GITHUB_REPO")
        .unwrap_or_else(|_| DEFAULT_GITHUB_REPO.to_string());
    let tag = std::env::var("AETHERFORGE_RELEASE_TAG")
        .unwrap_or_else(|_| DEFAULT_RELEASE_TAG.to_string());
    let local = git_capture(&root, &["rev-parse", "HEAD"]);

    let mut ls_remote = Command::new("git");
    ls_remote
        .args(["ls-remote", "origin", "refs/heads/main"])
        .current_dir(&root);
    let remote_text = output_text(command_output(ls_remote)?, "git ls-remote")?;
    let remote = remote_text.split_whitespace().next().unwrap_or("");
    if local != remote {
        return Err(format!("remote main mismatch: local={local} remote={remote}").into());
    }

    let manifest = root.join("artifacts/LARGE-FILES.tsv");
    if manifest.is_file() {
        let mut release = Command::new("gh");
        release.args([
            "release",
            "view",
            &tag,
            "-R",
            &repo,
            "--json",
            "assets",
            "--jq",
            ".assets[].name",
        ]);
        let names = output_text(command_output(release)?, "gh release view")?;
        for line in fs::read_to_string(&manifest)?.lines().skip(1) {
            let fields: Vec<&str> = line.split('\t').collect();
            if let Some(asset) = fields.get(4) {
                if !asset.is_empty() && !names.lines().any(|name| name == *asset) {
                    return Err(format!("missing GitHub release asset: {asset}").into());
                }
            }
        }
    }
    Ok("FORGECLEAN_GH_EXTERNAL_STORAGE_VERIFY=PASS\n".to_string())
}

pub fn run_migration_action(
    home: &Path,
    action: MigrationAction,
) -> Result<String, Box<dyn Error>> {
    match action {
        MigrationAction::Status | MigrationAction::Inventory | MigrationAction::Scan => {
            Ok(migration_status(home))
        }
        MigrationAction::Sync => {
            sync_forgeclean_source(home)?;
            run_orbital_action(home, OrbitalAction::Sync)
        }
        MigrationAction::Upload => {
            sync_forgeclean_source(home)?;
            run_orbital_action(home, OrbitalAction::Full)
        }
        MigrationAction::Verify => verify_external_storage(home),
        MigrationAction::Stage => {
            sync_forgeclean_source(home)?;
            Ok("FORGECLEAN_GH_EXTERNAL_STORAGE_STAGE=PASS\n".to_string())
        }
        MigrationAction::Restore => {
            let helper = repo_root(home).join("artifacts/restore-large-assets.sh");
            if !helper.is_file() {
                return Err(format!("restore helper missing: {}", helper.display()).into());
            }
            let mut command = Command::new("bash");
            command.arg(helper).arg(repo_root(home));
            output_text(command_output(command)?, "restore-large-assets")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_home() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("forgeclean-orbital-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn parses_orbital_status_without_banner_lines() {
        let status = parse_status(
            "AETHERFORGE_ORBITAL_SYNC_STATUS=START\nmode=fast\nresult=PASS\nqueue_depth=0\nAETHERFORGE_ORBITAL_SYNC_STATUS=END\n",
        );
        assert_eq!(status.get("mode"), "fast");
        assert_eq!(status.get("result"), "PASS");
        assert_eq!(status.get("queue_depth"), "0");
    }

    #[test]
    fn parses_supported_actions() {
        assert_eq!(OrbitalAction::parse("full"), Some(OrbitalAction::Full));
        assert_eq!(
            MigrationAction::parse("upload"),
            Some(MigrationAction::Upload)
        );
        assert_eq!(
            MigrationAction::parse("stage"),
            Some(MigrationAction::Stage)
        );
        assert_eq!(MigrationAction::parse("bogus"), None);
    }

    #[test]
    fn latest_source_selects_highest_semver() {
        let home = temp_home();
        for version in ["1.0.2", "1.0.3", "1.2.0"] {
            let dir = home
                .join("Downloads")
                .join(format!("ForgeClean-v{version}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("Cargo.toml"), "[package]\nname=\"forgeclean\"\n").unwrap();
        }
        let latest = latest_forgeclean_source(&home).unwrap();
        assert!(latest.ends_with("ForgeClean-v1.2.0"));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn local_storage_summary_counts_manifests() {
        let home = temp_home();
        let artifacts = home.join("Downloads/AETHERFORGE/artifacts");
        fs::create_dir_all(&artifacts).unwrap();
        fs::write(artifacts.join("INDEX.tsv"), "h\na\nb\n").unwrap();
        fs::write(artifacts.join("LARGE-FILES.tsv"), "h\na\n").unwrap();
        fs::write(artifacts.join("restore-large-assets.sh"), "#!/bin/sh\n").unwrap();
        let summary = local_storage_summary(&home);
        assert_eq!(summary.indexed_artifacts, 2);
        assert_eq!(summary.externalized_large_files, 1);
        assert!(summary.restore_helper_present);
        let _ = fs::remove_dir_all(home);
    }
}
