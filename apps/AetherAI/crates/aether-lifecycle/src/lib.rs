mod activate;
mod health;
mod release;
mod rollback;
mod startup;
mod verify;

pub use activate::FsLifecycleManager;
pub use release::{
    FileDigest, ReleaseInfo, ReleaseManifest, StagedRelease, UpdateStatus, VerificationReport,
};
pub use startup::StartupRecovery;
pub use verify::sha256_file;

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error("I/O error: {0}")]
    Io(String),

    #[error("home directory unavailable")]
    HomeUnavailable,

    #[error("invalid release manifest: {0}")]
    InvalidManifest(String),

    #[error("release integrity failure: {0}")]
    Integrity(String),

    #[error("release architecture mismatch: expected {expected}, got {actual}")]
    Architecture { expected: String, actual: String },

    #[error("runtime contract violation: {0}")]
    Contract(String),

    #[error("health diagnostic failure: {0}")]
    Health(String),

    #[error("release already installed: {0}")]
    AlreadyInstalled(String),

    #[error("no previous verified release is available")]
    NoPreviousRelease,

    #[error("no staged update is available")]
    NoStagedUpdate,
}

impl From<std::io::Error> for LifecycleError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub trait LifecycleManager {
    fn current_release(&self) -> Result<Option<ReleaseInfo>, LifecycleError>;

    fn check_for_update(&self) -> Result<UpdateStatus, LifecycleError>;

    fn stage_update(&self, release: &ReleaseInfo) -> Result<StagedRelease, LifecycleError>;

    fn verify_staged(&self, staged: &StagedRelease) -> Result<VerificationReport, LifecycleError>;

    fn activate(&self, staged: &StagedRelease) -> Result<(), LifecycleError>;

    fn rollback(&self) -> Result<(), LifecycleError>;
}

impl LifecycleManager for FsLifecycleManager {
    fn current_release(&self) -> Result<Option<ReleaseInfo>, LifecycleError> {
        let Some(version) = self.current_version()? else {
            return Ok(None);
        };

        let source = self.releases_dir().join(&version);
        let manifest = verify::read_manifest(&source)?;

        Ok(Some(ReleaseInfo {
            version,
            architecture: manifest.architecture,
            source,
        }))
    }

    fn check_for_update(&self) -> Result<UpdateStatus, LifecycleError> {
        if let Some(release) = self.discover_incoming()? {
            return Ok(UpdateStatus::Available(release));
        }

        match self.current_version()? {
            Some(current) => Ok(UpdateStatus::UpToDate { current }),
            None => Ok(UpdateStatus::NotInstalled),
        }
    }

    fn stage_update(&self, release: &ReleaseInfo) -> Result<StagedRelease, LifecycleError> {
        if release.architecture != self.expected_arch() {
            return Err(LifecycleError::Architecture {
                expected: self.expected_arch().into(),
                actual: release.architecture.clone(),
            });
        }

        self.ensure_layout()?;

        let destination = self.staging_dir().join(&release.version);
        if destination.exists() {
            std::fs::remove_dir_all(&destination)?;
        }

        activate::copy_tree(&release.source, &destination)?;
        let staged = verify::staged_from_root(destination)?;

        if staged.manifest.version != release.version {
            return Err(LifecycleError::InvalidManifest(
                "release info version does not match manifest".into(),
            ));
        }

        Ok(staged)
    }

    fn verify_staged(&self, staged: &StagedRelease) -> Result<VerificationReport, LifecycleError> {
        self.verify_release(staged)
    }

    fn activate(&self, staged: &StagedRelease) -> Result<(), LifecycleError> {
        self.verify_staged(staged)?;
        let had_current = self.current_version()?.is_some();
        let installed = self.install_staged(staged)?;
        self.activate_existing(&installed)?;

        if had_current {
            if let Err(error) = self.mark_pending_startup(&installed.manifest.version) {
                let _ = rollback::rollback(self);
                return Err(error);
            }
        }

        Ok(())
    }

    fn rollback(&self) -> Result<(), LifecycleError> {
        rollback::rollback(self)?;
        self.clear_pending_startup()
    }
}

pub fn default_lifecycle_root() -> Result<PathBuf, LifecycleError> {
    Ok(FsLifecycleManager::for_user()?.root().to_path_buf())
}
