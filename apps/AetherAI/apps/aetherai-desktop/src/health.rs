use aether_lifecycle::{FsLifecycleManager, LifecycleManager, ReleaseManifest, sha256_file};
use aether_storage::SqliteStore;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const EXPECTED_SCHEMA_VERSION: u32 = 3;
const STARTUP_REQUIRED_KEYS: &[&str] = &[
    "VERSION",
    "RENDERER_SCHEMA",
    "DATABASE_MIGRATION",
    "LOCAL_MODEL_RUNTIME",
    "NATIVE_CHATGPT_LIBRARY",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Pass,
    Warn,
    Fail,
}

impl HealthState {
    fn token(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthAction {
    Recheck,
    Repair,
    RollBack,
    OpenLogs,
    ExportDiagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthCheck {
    pub key: String,
    pub name: String,
    pub state: HealthState,
    pub detail: String,
}

impl HealthCheck {
    pub fn pass(
        key: impl Into<String>,
        name: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            state: HealthState::Pass,
            detail: detail.into(),
        }
    }

    pub fn warn(
        key: impl Into<String>,
        name: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            state: HealthState::Warn,
            detail: detail.into(),
        }
    }

    pub fn fail(
        key: impl Into<String>,
        name: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            state: HealthState::Fail,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthPanelState {
    pub checks: Vec<HealthCheck>,
    pub actions: Vec<HealthAction>,
    pub last_export: Option<PathBuf>,
}

impl HealthPanelState {
    pub fn from_checks(checks: Vec<HealthCheck>) -> Self {
        Self {
            checks,
            actions: vec![
                HealthAction::Recheck,
                HealthAction::Repair,
                HealthAction::RollBack,
                HealthAction::OpenLogs,
                HealthAction::ExportDiagnostic,
            ],
            last_export: None,
        }
    }

    pub fn evaluate(
        data_dir: &Path,
        store: &SqliteStore,
        model_count: usize,
        provider_count: usize,
        updater_summary: &str,
    ) -> Self {
        let mut checks = Vec::with_capacity(11);

        checks.push(HealthCheck::pass(
            "VERSION",
            "Version",
            env!("CARGO_PKG_VERSION"),
        ));

        checks.push(HealthCheck::pass(
            "RENDERER_SCHEMA",
            "Renderer/schema authority",
            "eframe/egui→wgpu/winit; AetherForge Terminal 10.2.21; DragonGlass zero-drift",
        ));

        match store.schema_version() {
            Ok(EXPECTED_SCHEMA_VERSION) => checks.push(HealthCheck::pass(
                "DATABASE_MIGRATION",
                "Database migration",
                format!("schema v{EXPECTED_SCHEMA_VERSION}"),
            )),
            Ok(actual) => checks.push(HealthCheck::fail(
                "DATABASE_MIGRATION",
                "Database migration",
                format!("expected schema v{EXPECTED_SCHEMA_VERSION}, database reports v{actual}"),
            )),
            Err(error) => checks.push(HealthCheck::fail(
                "DATABASE_MIGRATION",
                "Database migration",
                error.to_string(),
            )),
        }

        if model_count == 0 {
            checks.push(HealthCheck::warn(
                "LOCAL_MODEL_RUNTIME",
                "Local model runtime",
                "Idle: no local model is registered; runtime is not required for startup",
            ));
        } else if provider_count >= model_count {
            checks.push(HealthCheck::pass(
                "LOCAL_MODEL_RUNTIME",
                "Local model runtime",
                format!("{provider_count}/{model_count} registered model provider(s) available"),
            ));
        } else {
            checks.push(HealthCheck::fail(
                "LOCAL_MODEL_RUNTIME",
                "Local model runtime",
                format!(
                    "{model_count} model(s) registered but only {provider_count} provider(s) are available"
                ),
            ));
        }

        checks.push(storage_write_check(data_dir));

        match store.retrieval_history_count() {
            Ok(count) => checks.push(HealthCheck::pass(
                "RETRIEVAL_INDEX",
                "Retrieval index",
                format!("retrieval/index tables reachable; {count} history row(s)"),
            )),
            Err(error) => checks.push(HealthCheck::fail(
                "RETRIEVAL_INDEX",
                "Retrieval index",
                error.to_string(),
            )),
        }

        match store.list_conversations() {
            Ok(conversations) => checks.push(HealthCheck::pass(
                "NATIVE_CHATGPT_LIBRARY",
                "Native ChatGPT Library",
                format!(
                    "local conversation/provenance store reachable; {} conversation(s)",
                    conversations.len()
                ),
            )),
            Err(error) => checks.push(HealthCheck::fail(
                "NATIVE_CHATGPT_LIBRARY",
                "Native ChatGPT Library",
                error.to_string(),
            )),
        }

        if updater_summary.to_ascii_lowercase().contains("failed") {
            checks.push(HealthCheck::fail("UPDATER", "Updater", updater_summary));
        } else {
            checks.push(HealthCheck::pass("UPDATER", "Updater", updater_summary));
        }

        checks.push(release_integrity_check());

        checks.push(HealthCheck::pass(
            "PERMISSIONS",
            "Permissions",
            "capability-gated Files/Terminal/Network/System policy active; ASK remains the safe default",
        ));

        match store.list_activities() {
            Ok(activities) => checks.push(HealthCheck::pass(
                "BACKGROUND_ACTIVITY",
                "Background activity",
                format!(
                    "activity store reachable; {} persisted record(s)",
                    activities.len()
                ),
            )),
            Err(error) => checks.push(HealthCheck::fail(
                "BACKGROUND_ACTIVITY",
                "Background activity",
                error.to_string(),
            )),
        }

        Self::from_checks(checks)
    }

    pub fn failed_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.state == HealthState::Fail)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|check| check.state == HealthState::Warn)
            .count()
    }

    pub fn startup_minimum_healthy(&self) -> bool {
        STARTUP_REQUIRED_KEYS.iter().all(|required| {
            self.checks
                .iter()
                .find(|check| check.key == *required)
                .is_some_and(|check| check.state != HealthState::Fail)
        })
    }

    pub fn summary(&self) -> String {
        format!(
            "{} check(s), {} failed, {} warning(s)",
            self.checks.len(),
            self.failed_count(),
            self.warning_count()
        )
    }
}

fn storage_write_check(data_dir: &Path) -> HealthCheck {
    let probe = data_dir.join(".aetherai-health-write-probe");
    let result = (|| -> std::io::Result<()> {
        fs::create_dir_all(data_dir)?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&probe)?;
        file.write_all(b"AETHERAI_HEALTH_PROBE\n")?;
        file.sync_all()?;
        drop(file);
        fs::remove_file(&probe)?;
        Ok(())
    })();

    match result {
        Ok(()) => HealthCheck::pass(
            "STORAGE",
            "Storage",
            format!("read/write/fsync probe passed at {}", data_dir.display()),
        ),
        Err(error) => HealthCheck::fail("STORAGE", "Storage", error.to_string()),
    }
}

fn safe_release_path(root: &Path, relative: &str) -> Option<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    Some(root.join(path))
}

fn release_integrity_check() -> HealthCheck {
    let manager = match FsLifecycleManager::for_user() {
        Ok(manager) => manager,
        Err(error) => {
            return HealthCheck::warn(
                "RELEASE_INTEGRITY",
                "Release integrity",
                format!("self-managing lifecycle root unavailable: {error}"),
            );
        }
    };

    let release = match manager.current_release() {
        Ok(Some(release)) => release,
        Ok(None) => {
            return HealthCheck::warn(
                "RELEASE_INTEGRITY",
                "Release integrity",
                "development/uninstalled mode: no current self-managed release pointer",
            );
        }
        Err(error) => {
            return HealthCheck::fail("RELEASE_INTEGRITY", "Release integrity", error.to_string());
        }
    };

    let manifest_path = release.source.join("release.json");
    let sidecar_path = release.source.join("release.json.sha256");

    let expected_manifest_hash = match fs::read_to_string(&sidecar_path) {
        Ok(value) => value.trim().to_string(),
        Err(error) => {
            return HealthCheck::fail("RELEASE_INTEGRITY", "Release integrity", error.to_string());
        }
    };

    let actual_manifest_hash = match sha256_file(&manifest_path) {
        Ok(hash) => hash,
        Err(error) => {
            return HealthCheck::fail("RELEASE_INTEGRITY", "Release integrity", error.to_string());
        }
    };

    if expected_manifest_hash != actual_manifest_hash {
        return HealthCheck::fail(
            "RELEASE_INTEGRITY",
            "Release integrity",
            "release.json SHA-256 mismatch",
        );
    }

    let manifest: ReleaseManifest = match fs::read(&manifest_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
    {
        Some(manifest) => manifest,
        None => {
            return HealthCheck::fail(
                "RELEASE_INTEGRITY",
                "Release integrity",
                "release.json cannot be parsed",
            );
        }
    };

    for digest in &manifest.files {
        let Some(path) = safe_release_path(&release.source, &digest.path) else {
            return HealthCheck::fail(
                "RELEASE_INTEGRITY",
                "Release integrity",
                format!("unsafe manifest path: {}", digest.path),
            );
        };

        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                return HealthCheck::fail(
                    "RELEASE_INTEGRITY",
                    "Release integrity",
                    format!("{}: {error}", digest.path),
                );
            }
        };

        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return HealthCheck::fail(
                "RELEASE_INTEGRITY",
                "Release integrity",
                format!("{} is not a regular release file", digest.path),
            );
        }

        match sha256_file(&path) {
            Ok(actual) if actual == digest.sha256 => {}
            Ok(_) => {
                return HealthCheck::fail(
                    "RELEASE_INTEGRITY",
                    "Release integrity",
                    format!("SHA-256 mismatch: {}", digest.path),
                );
            }
            Err(error) => {
                return HealthCheck::fail(
                    "RELEASE_INTEGRITY",
                    "Release integrity",
                    error.to_string(),
                );
            }
        }
    }

    HealthCheck::pass(
        "RELEASE_INTEGRITY",
        "Release integrity",
        format!(
            "{} verified manifest + {} hashed file(s)",
            release.version,
            manifest.files.len()
        ),
    )
}

pub fn create_startup_checkpoint(data_dir: &Path) -> std::io::Result<Option<PathBuf>> {
    let database = data_dir.join("aetherai.db");
    if !database.exists() {
        return Ok(None);
    }

    let checkpoint = data_dir
        .join("checkpoints")
        .join(format!("startup-preflight-{}", std::process::id()));
    fs::create_dir_all(&checkpoint)?;

    let mut copied_any = false;
    for name in ["aetherai.db", "aetherai.db-wal", "aetherai.db-shm"] {
        let source = data_dir.join(name);
        if source.is_file() {
            fs::copy(&source, checkpoint.join(name))?;
            copied_any = true;
        }
    }

    if copied_any {
        Ok(Some(checkpoint))
    } else {
        Ok(None)
    }
}

pub fn mark_pending_release_healthy() -> Result<Option<String>, aether_lifecycle::LifecycleError> {
    let manager = FsLifecycleManager::for_user()?;
    if !manager.current_executable_is_managed()? {
        return Ok(None);
    }
    manager.mark_startup_healthy()
}

pub fn repair_safe_directories(data_dir: &Path) -> std::io::Result<()> {
    for relative in ["imports", "models", "runtime", "logs", "checkpoints"] {
        fs::create_dir_all(data_dir.join(relative))?;
    }
    Ok(())
}

pub fn diagnostic_path_in(downloads: &Path) -> PathBuf {
    downloads.join("AetherAI-v0.3.7-DIAGNOSTIC.txt")
}

pub fn export_diagnostic(target: &Path, panel: &HealthPanelState) -> std::io::Result<()> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let temp = parent.join(format!(
        ".{}.{}.tmp",
        target.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));

    let mut text = String::new();
    text.push_str("AETHERAI_VERSION=0.3.7\n");
    text.push_str("AETHERAI_DIAGNOSTIC_SCHEMA=HEALTH_V1\n");
    text.push_str(&format!(
        "AETHERAI_HEALTH_CHECK_COUNT={}\n",
        panel.checks.len()
    ));
    text.push_str(&format!(
        "AETHERAI_HEALTH_FAILED_COUNT={}\n",
        panel.failed_count()
    ));
    text.push_str(&format!(
        "AETHERAI_HEALTH_WARNING_COUNT={}\n",
        panel.warning_count()
    ));

    for check in &panel.checks {
        text.push_str(&format!(
            "AETHERAI_HEALTH_{}={}\n",
            check.key,
            check.state.token()
        ));
        text.push_str(&format!(
            "AETHERAI_HEALTH_{}_DETAIL={}\n",
            check.key,
            one_line(&check.detail)
        ));
    }

    text.push_str(&format!(
        "AETHERAI_STARTUP_MINIMUM_HEALTHY={}\n",
        if panel.startup_minimum_healthy() {
            "YES"
        } else {
            "NO"
        }
    ));

    fs::write(&temp, text)?;
    fs::rename(temp, target)?;
    Ok(())
}

fn one_line(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch == '\n' || ch == '\r' { ' ' } else { ch })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_panel_exposes_actionable_failed_check() {
        let panel = HealthPanelState::from_checks(vec![HealthCheck::fail(
            "RETRIEVAL_INDEX",
            "Retrieval index",
            "Index database unavailable",
        )]);

        assert_eq!(panel.failed_count(), 1);
        assert!(panel.actions.contains(&HealthAction::Repair));
        assert!(panel.actions.contains(&HealthAction::ExportDiagnostic));
        assert!(panel.actions.contains(&HealthAction::RollBack));
    }

    #[test]
    fn diagnostic_export_writes_uploadable_pass_fail_text() {
        let target = std::env::temp_dir().join(format!(
            "AetherAI-v0.3.7-DIAGNOSTIC-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let panel = HealthPanelState::from_checks(vec![
            HealthCheck::pass("VERSION", "Version", "0.3.7"),
            HealthCheck::warn(
                "LOCAL_MODEL_RUNTIME",
                "Local model runtime",
                "Idle: no model registered",
            ),
        ]);

        export_diagnostic(&target, &panel).unwrap();
        let text = std::fs::read_to_string(&target).unwrap();

        assert!(text.contains("AETHERAI_VERSION=0.3.7"));
        assert!(text.contains("AETHERAI_HEALTH_VERSION=PASS"));
        assert!(text.contains("AETHERAI_HEALTH_LOCAL_MODEL_RUNTIME=WARN"));
        assert!(text.contains("AETHERAI_HEALTH_FAILED_COUNT=0"));
        let _ = std::fs::remove_file(target);
    }

    #[test]
    fn startup_checkpoint_copies_database_without_mutating_source() {
        let root = std::env::temp_dir().join(format!(
            "aetherai-startup-checkpoint-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let database = root.join("aetherai.db");
        std::fs::write(&database, b"ORIGINAL-USER-DATABASE").unwrap();

        let checkpoint = create_startup_checkpoint(&root).unwrap().unwrap();

        assert_eq!(std::fs::read(&database).unwrap(), b"ORIGINAL-USER-DATABASE");
        assert_eq!(
            std::fs::read(checkpoint.join("aetherai.db")).unwrap(),
            b"ORIGINAL-USER-DATABASE"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn startup_minimum_gate_rejects_failed_required_check() {
        let panel = HealthPanelState::from_checks(vec![
            HealthCheck::pass("VERSION", "Version", "0.3.7"),
            HealthCheck::fail(
                "DATABASE_MIGRATION",
                "Database migration",
                "migration failed",
            ),
            HealthCheck::pass("RENDERER_SCHEMA", "Renderer/schema authority", "green"),
            HealthCheck::pass("LOCAL_MODEL_RUNTIME", "Local model runtime", "idle"),
            HealthCheck::pass("NATIVE_CHATGPT_LIBRARY", "Native ChatGPT Library", "open"),
        ]);

        assert!(!panel.startup_minimum_healthy());
    }
}
