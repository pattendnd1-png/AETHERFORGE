use crate::FirmwareError;
use forgehx_core::{
    DeviceId, FirmwareSupportLevel, FirmwareTransactionState, FirmwareTransactionStatus,
};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone)]
pub struct FirmwareTransaction {
    pub status: FirmwareTransactionStatus,
    support_level: FirmwareSupportLevel,
}

impl FirmwareTransaction {
    pub fn new(
        transaction_id: String,
        device_id: DeviceId,
        staged_id: String,
        support_level: FirmwareSupportLevel,
    ) -> Self {
        Self {
            status: FirmwareTransactionStatus {
                transaction_id,
                device_id,
                staged_id,
                state: FirmwareTransactionState::Idle,
                message: None,
                progress_percent: Some(0),
            },
            support_level,
        }
    }

    pub fn transition(&mut self, next: FirmwareTransactionState) -> Result<(), FirmwareError> {
        use FirmwareTransactionState as S;
        if matches!(
            next,
            S::EnteringUpdateMode
                | S::AwaitingUpdateDevice
                | S::Flashing
                | S::Finalizing
                | S::AwaitingNormalDevice
                | S::Verifying
                | S::RestoringProfile
        ) && !self.support_level.can_update()
        {
            return Err(FirmwareError::UpdateNotEnabled);
        }
        let allowed = matches!(
            (self.status.state, next),
            (S::Idle, S::PackageStaged)
                | (S::PackageStaged, S::Validated)
                | (S::Validated, S::PreflightPassed)
                | (S::PreflightPassed, S::AudioDetached)
                | (S::AudioDetached, S::EnteringUpdateMode)
                | (S::EnteringUpdateMode, S::AwaitingUpdateDevice)
                | (S::AwaitingUpdateDevice, S::Flashing)
                | (S::Flashing, S::Finalizing)
                | (S::Finalizing, S::AwaitingNormalDevice)
                | (S::AwaitingNormalDevice, S::Verifying)
                | (S::Verifying, S::RestoringProfile)
                | (S::RestoringProfile, S::Completed)
                | (_, S::Rejected)
                | (_, S::PreflightFailed)
                | (_, S::UpdateDeviceMissing)
                | (_, S::FlashFailed)
                | (_, S::NormalDeviceMissing)
                | (_, S::VerificationFailed)
                | (_, S::RecoveryRequired)
        );
        if !allowed {
            return Err(FirmwareError::InvalidTransition(format!(
                "{:?} -> {:?}",
                self.status.state, next
            )));
        }
        self.status.state = next;
        self.status.progress_percent = progress_for(next);
        Ok(())
    }
}

fn progress_for(state: FirmwareTransactionState) -> Option<u8> {
    use FirmwareTransactionState as S;
    match state {
        S::Idle => Some(0),
        S::PackageStaged => Some(5),
        S::Validated => Some(10),
        S::PreflightPassed => Some(15),
        S::AudioDetached => Some(20),
        S::EnteringUpdateMode => Some(25),
        S::AwaitingUpdateDevice => Some(30),
        S::Flashing => Some(50),
        S::Finalizing => Some(80),
        S::AwaitingNormalDevice => Some(85),
        S::Verifying => Some(90),
        S::RestoringProfile => Some(95),
        S::Completed => Some(100),
        _ => None,
    }
}

pub struct FirmwareJournal {
    root: PathBuf,
}

impl FirmwareJournal {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn save(&self, status: &FirmwareTransactionStatus) -> Result<(), FirmwareError> {
        fs::create_dir_all(&self.root).map_err(|e| FirmwareError::Journal(e.to_string()))?;
        let path = self.root.join(format!("{}.json", status.transaction_id));
        let tmp = self.root.join(format!(".{}.tmp", status.transaction_id));
        let bytes =
            serde_json::to_vec_pretty(status).map_err(|e| FirmwareError::Journal(e.to_string()))?;
        fs::write(&tmp, bytes).map_err(|e| FirmwareError::Journal(e.to_string()))?;
        fs::rename(&tmp, &path).map_err(|e| FirmwareError::Journal(e.to_string()))?;
        Ok(())
    }

    pub fn load(&self, transaction_id: &str) -> Result<FirmwareTransactionStatus, FirmwareError> {
        let path = self.root.join(format!("{transaction_id}.json"));
        let bytes = fs::read(path).map_err(|e| FirmwareError::Journal(e.to_string()))?;
        serde_json::from_slice(&bytes).map_err(|e| FirmwareError::Journal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_capable_transaction_accepts_only_ordered_path() {
        let mut tx = FirmwareTransaction::new(
            "tx".into(),
            DeviceId("mic".into()),
            "stage".into(),
            FirmwareSupportLevel::UpdateCapable,
        );
        for state in [
            FirmwareTransactionState::PackageStaged,
            FirmwareTransactionState::Validated,
            FirmwareTransactionState::PreflightPassed,
            FirmwareTransactionState::AudioDetached,
            FirmwareTransactionState::EnteringUpdateMode,
            FirmwareTransactionState::AwaitingUpdateDevice,
            FirmwareTransactionState::Flashing,
            FirmwareTransactionState::Finalizing,
            FirmwareTransactionState::AwaitingNormalDevice,
            FirmwareTransactionState::Verifying,
            FirmwareTransactionState::RestoringProfile,
            FirmwareTransactionState::Completed,
        ] {
            tx.transition(state).unwrap();
        }
        assert_eq!(tx.status.progress_percent, Some(100));
    }

    #[test]
    fn transaction_rejects_skipping_directly_to_flashing() {
        let mut tx = FirmwareTransaction::new(
            "tx".into(),
            DeviceId("mic".into()),
            "stage".into(),
            FirmwareSupportLevel::UpdateCapable,
        );
        tx.transition(FirmwareTransactionState::PackageStaged)
            .unwrap();
        tx.transition(FirmwareTransactionState::Validated).unwrap();
        assert!(tx.transition(FirmwareTransactionState::Flashing).is_err());
    }

    #[test]
    fn inventory_only_transaction_never_enters_update_transport() {
        let mut tx = FirmwareTransaction::new(
            "tx".into(),
            DeviceId("mic".into()),
            "stage".into(),
            FirmwareSupportLevel::InventoryOnly,
        );
        tx.transition(FirmwareTransactionState::PackageStaged)
            .unwrap();
        tx.transition(FirmwareTransactionState::Validated).unwrap();
        tx.transition(FirmwareTransactionState::PreflightPassed)
            .unwrap();
        tx.transition(FirmwareTransactionState::AudioDetached)
            .unwrap();
        assert!(matches!(
            tx.transition(FirmwareTransactionState::EnteringUpdateMode),
            Err(FirmwareError::UpdateNotEnabled)
        ));
    }
}
