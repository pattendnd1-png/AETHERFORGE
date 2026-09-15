use crate::{FsLifecycleManager, LifecycleError, StagedRelease, verify::verify_release_dir};

pub(crate) fn rollback(manager: &FsLifecycleManager) -> Result<(), LifecycleError> {
    let previous_link = manager.previous_link();
    if !previous_link.is_symlink() {
        return Err(LifecycleError::NoPreviousRelease);
    }

    let previous_target = std::fs::read_link(&previous_link)?;
    verify_release_dir(&previous_target, manager.expected_arch())?;

    let manifest = crate::verify::read_manifest(&previous_target)?;
    let previous = StagedRelease {
        root: previous_target,
        manifest,
    };

    manager.activate_existing(&previous)
}
