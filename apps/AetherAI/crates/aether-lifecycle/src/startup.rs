use crate::{
    FsLifecycleManager, LifecycleError, LifecycleManager, ReleaseManifest,
    verify::{read_manifest, safe_relative},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const PENDING_STARTUP_FILE: &str = "pending-startup.json";
const RECOVERY_REPORT_FILE: &str = "startup-recovery-report.txt";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PendingStartup {
    version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupRecovery {
    RolledBack {
        failed_version: String,
        restored_version: String,
        binary: PathBuf,
        report: PathBuf,
    },
}

impl FsLifecycleManager {
    pub fn pending_startup_path(&self) -> PathBuf {
        self.root().join(PENDING_STARTUP_FILE)
    }

    pub fn recovery_report_path(&self) -> PathBuf {
        self.root().join(RECOVERY_REPORT_FILE)
    }

    pub fn pending_startup_version(&self) -> Result<Option<String>, LifecycleError> {
        let path = self.pending_startup_path();
        if !path.is_file() {
            return Ok(None);
        }

        let pending: PendingStartup = serde_json::from_slice(&std::fs::read(path)?)
            .map_err(|error| LifecycleError::InvalidManifest(error.to_string()))?;
        Ok(Some(pending.version))
    }

    pub(crate) fn mark_pending_startup(&self, version: &str) -> Result<(), LifecycleError> {
        self.ensure_layout()?;
        let target = self.pending_startup_path();
        let temp = self
            .root()
            .join(format!(".pending-startup.{}.tmp", std::process::id()));

        let payload = serde_json::to_vec_pretty(&PendingStartup {
            version: version.into(),
        })
        .map_err(|error| LifecycleError::InvalidManifest(error.to_string()))?;

        std::fs::write(&temp, payload)?;
        if target.exists() {
            std::fs::remove_file(&target)?;
        }
        std::fs::rename(temp, target)?;
        Ok(())
    }

    pub(crate) fn clear_pending_startup(&self) -> Result<(), LifecycleError> {
        let path = self.pending_startup_path();
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn mark_startup_healthy(&self) -> Result<Option<String>, LifecycleError> {
        let Some(pending) = self.pending_startup_version()? else {
            return Ok(None);
        };

        let current = self.current_version()?.ok_or_else(|| {
            LifecycleError::Contract("pending startup exists without current release".into())
        })?;

        if current != pending {
            return Err(LifecycleError::Contract(format!(
                "pending startup version {pending} does not match current release {current}"
            )));
        }

        self.clear_pending_startup()?;
        Ok(Some(pending))
    }

    pub fn current_binary(&self) -> Result<Option<PathBuf>, LifecycleError> {
        let Some(release) = self.current_release()? else {
            return Ok(None);
        };
        let manifest = read_manifest(&release.source)?;
        let relative = safe_relative(&manifest.binary)?;
        Ok(Some(release.source.join(relative)))
    }

    pub fn current_executable_is_managed(&self) -> Result<bool, LifecycleError> {
        let Some(binary) = self.current_binary()? else {
            return Ok(false);
        };
        let running = std::env::current_exe()?.canonicalize().ok();
        let managed = binary.canonicalize().ok();
        Ok(running.is_some() && running == managed)
    }

    pub fn recover_pending_startup_failure(
        &self,
        reason: &str,
    ) -> Result<Option<StartupRecovery>, LifecycleError> {
        let Some(failed_version) = self.pending_startup_version()? else {
            return Ok(None);
        };

        let current = self.current_version()?.ok_or_else(|| {
            LifecycleError::Contract("pending startup exists without current release".into())
        })?;

        if current != failed_version {
            return Err(LifecycleError::Contract(format!(
                "refusing recovery: pending version {failed_version} != current {current}"
            )));
        }

        crate::rollback::rollback(self)?;
        self.clear_pending_startup()?;

        let restored = self
            .current_release()?
            .ok_or(LifecycleError::NoPreviousRelease)?;
        let manifest: ReleaseManifest = read_manifest(&restored.source)?;
        let relative = safe_relative(&manifest.binary)?;
        let binary = restored.source.join(relative);

        if !binary.is_file() {
            return Err(LifecycleError::Integrity(
                "restored release binary is missing".into(),
            ));
        }

        let report = self.recovery_report_path();
        let text = format!(
            "AETHERAI_STARTUP_RECOVERY=ROLLED_BACK\n\
             AETHERAI_FAILED_VERSION={failed_version}\n\
             AETHERAI_RESTORED_VERSION={}\n\
             AETHERAI_USER_DATA_MUTATED=NO\n\
             AETHERAI_REASON={}\n",
            restored.version,
            one_line(reason),
        );
        std::fs::write(&report, text)?;

        Ok(Some(StartupRecovery::RolledBack {
            failed_version,
            restored_version: restored.version,
            binary,
            report,
        }))
    }
}

fn one_line(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character == '\n' || character == '\r' {
                ' '
            } else {
                character
            }
        })
        .collect()
}
