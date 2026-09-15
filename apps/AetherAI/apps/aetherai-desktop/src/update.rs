use aether_lifecycle::{
    FsLifecycleManager, LifecycleError, LifecycleManager, ReleaseManifest, StagedRelease,
    UpdateStatus, VerificationReport,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum UpdateUiState {
    Idle,
    Checking,
    Available(aether_lifecycle::ReleaseInfo),
    Downloading { progress_percent: u8 },
    Verifying,
    ReadyToRestart(StagedRelease),
    Recovering,
    Failed(String),
}

pub struct UpdatePanelState {
    manager: FsLifecycleManager,
    pub state: UpdateUiState,
    pub last_verification: Option<VerificationReport>,
    pub recovery_notice: Option<String>,
}
impl UpdatePanelState {
    pub fn for_user() -> Result<Self, LifecycleError> {
        Ok(Self::new(FsLifecycleManager::for_user()?))
    }
    pub fn new(manager: FsLifecycleManager) -> Self {
        Self {
            manager,
            state: UpdateUiState::Idle,
            last_verification: None,
            recovery_notice: None,
        }
    }
    pub fn current_version(&self) -> Option<String> {
        self.manager.current_version().ok().flatten()
    }
    pub fn can_prepare(&self) -> bool {
        matches!(self.state, UpdateUiState::Available(_))
    }
    pub fn can_restart_and_update(&self) -> bool {
        matches!(self.state, UpdateUiState::ReadyToRestart(_))
    }
    pub fn summary(&self) -> String {
        match &self.state {
            UpdateUiState::Idle => self.current_version().map_or_else(
                || "Lifecycle ready — no self-managed release activated yet".into(),
                |v| format!("Installed {v} — lifecycle ready"),
            ),
            UpdateUiState::Checking => "Checking local incoming releases…".into(),
            UpdateUiState::Available(r) => format!("Update available: {}", r.version),
            UpdateUiState::Downloading { progress_percent } => {
                format!("Staging verified release payload: {progress_percent}%")
            }
            UpdateUiState::Verifying => {
                "Verifying manifest, hashes, architecture, contract and health…".into()
            }
            UpdateUiState::ReadyToRestart(s) => {
                format!("Verified {} — Restart & Update ready", s.manifest.version)
            }
            UpdateUiState::Recovering => "Recovering previous verified release…".into(),
            UpdateUiState::Failed(e) => format!("Update failed: {e}"),
        }
    }
    pub fn check_for_updates(&mut self) -> Result<(), LifecycleError> {
        self.state = UpdateUiState::Checking;
        self.recovery_notice = None;
        self.state = match self.manager.check_for_update()? {
            UpdateStatus::Available(r) => UpdateUiState::Available(r),
            UpdateStatus::UpToDate { current } => {
                self.recovery_notice = Some(format!("AetherAI {current} is up to date."));
                UpdateUiState::Idle
            }
            UpdateStatus::NotInstalled => {
                self.recovery_notice = Some(
                    "Self-managing release channel is ready for its first verified release.".into(),
                );
                UpdateUiState::Idle
            }
        };
        Ok(())
    }
    pub fn prepare_update(&mut self) -> Result<(), LifecycleError> {
        let r = match &self.state {
            UpdateUiState::Available(r) => r.clone(),
            _ => return Err(LifecycleError::NoStagedUpdate),
        };
        self.state = UpdateUiState::Downloading {
            progress_percent: 0,
        };
        let s = self.manager.stage_update(&r)?;
        self.state = UpdateUiState::Verifying;
        self.last_verification = Some(self.manager.verify_staged(&s)?);
        self.state = UpdateUiState::ReadyToRestart(s);
        Ok(())
    }
    pub fn restart_and_update(&mut self) -> Result<PathBuf, LifecycleError> {
        let s = match &self.state {
            UpdateUiState::ReadyToRestart(s) => s.clone(),
            _ => return Err(LifecycleError::NoStagedUpdate),
        };
        self.manager.verify_staged(&s)?;
        let v = s.manifest.version.clone();
        let rel = s.manifest.binary.clone();
        self.manager.activate(&s)?;
        let b = self.manager.releases_dir().join(v).join(rel);
        if !b.is_file() {
            return Err(LifecycleError::Integrity(
                "activated release binary is missing".into(),
            ));
        }
        self.state = UpdateUiState::Idle;
        Ok(b)
    }
    pub fn rollback(&mut self) -> Result<PathBuf, LifecycleError> {
        self.state = UpdateUiState::Recovering;
        self.manager.rollback()?;
        let cur = self
            .manager
            .current_release()?
            .ok_or(LifecycleError::NoPreviousRelease)?;
        let m: ReleaseManifest =
            serde_json::from_slice(&std::fs::read(cur.source.join("release.json"))?)
                .map_err(|e| LifecycleError::InvalidManifest(e.to_string()))?;
        let b = cur.source.join(m.binary);
        if !b.is_file() {
            return Err(LifecycleError::Integrity(
                "rollback release binary is missing".into(),
            ));
        }
        self.recovery_notice = Some(format!(
            "Update recovery completed\nRolled back to {}\n[View Report]",
            cur.version
        ));
        self.state = UpdateUiState::Idle;
        Ok(b)
    }
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.state = UpdateUiState::Failed(error.into());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use aether_lifecycle::{
        FileDigest, FsLifecycleManager, LifecycleManager, ReleaseInfo, ReleaseManifest, sha256_file,
    };
    use std::path::{Path, PathBuf};
    #[cfg(unix)]
    fn bin(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path,"#!/bin/sh\necho AETHERAI_DESKTOP=LIVE\necho AETHERAI_NORMAL_USER_TERMINAL_REQUIRED=NO\nexit 0\n").unwrap();
        let mut p = std::fs::metadata(path).unwrap().permissions();
        p.set_mode(0o755);
        std::fs::set_permissions(path, p).unwrap();
    }
    #[cfg(not(unix))]
    fn bin(path: &Path) {
        std::fs::write(path, "test binary").unwrap();
    }
    fn release(parent: &Path, v: &str) -> PathBuf {
        let r = parent.join(v);
        std::fs::create_dir_all(r.join("bin")).unwrap();
        let rel = if cfg!(windows) {
            "bin/aetherai-desktop.exe"
        } else {
            "bin/aetherai-desktop"
        };
        let b = r.join(rel);
        bin(&b);
        let m = ReleaseManifest {
            schema: 1,
            version: v.into(),
            architecture: std::env::consts::ARCH.into(),
            binary: rel.into(),
            browser_webview: false,
            openai_api_required: false,
            paid_service_required: false,
            files: vec![FileDigest {
                path: rel.into(),
                sha256: sha256_file(&b).unwrap(),
            }],
        };
        std::fs::write(
            r.join("release.json"),
            serde_json::to_vec_pretty(&m).unwrap(),
        )
        .unwrap();
        let h = sha256_file(&r.join("release.json")).unwrap();
        std::fs::write(r.join("release.json.sha256"), format!("{h}\n")).unwrap();
        r
    }
    fn tmp() -> PathBuf {
        let r = std::env::temp_dir().join(format!(
            "aetherai-task6b-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&r).unwrap();
        r
    }
    #[test]
    fn verified_candidate_is_only_restart_path() {
        let r = tmp();
        let mgr = FsLifecycleManager::new(r.join("install"), std::env::consts::ARCH);
        let base = release(&r.join("seed"), "0.3.6");
        let info = ReleaseInfo {
            version: "0.3.6".into(),
            architecture: std::env::consts::ARCH.into(),
            source: base,
        };
        let s = mgr.stage_update(&info).unwrap();
        mgr.activate(&s).unwrap();
        std::fs::create_dir_all(mgr.incoming_dir()).unwrap();
        release(&mgr.incoming_dir(), "0.3.7");
        let mut p = UpdatePanelState::new(mgr);
        p.check_for_updates().unwrap();
        assert!(matches!(p.state, UpdateUiState::Available(_)));
        assert!(p.restart_and_update().is_err());
        p.prepare_update().unwrap();
        assert!(p.can_restart_and_update());
        let b = p.restart_and_update().unwrap();
        assert!(b.is_file());
        assert_eq!(p.current_version().as_deref(), Some("0.3.7"));
        let rb = p.rollback().unwrap();
        assert!(rb.is_file());
        assert_eq!(p.current_version().as_deref(), Some("0.3.6"));
        assert!(
            p.recovery_notice
                .as_deref()
                .unwrap_or_default()
                .contains("Update recovery completed")
        );
        let _ = std::fs::remove_dir_all(r);
    }
}
