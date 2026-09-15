use crate::BackendCandidate;
use forgehx_core::{BackendKind, Capability, CapabilityOwner};

fn priority(kind: BackendKind) -> u8 {
    match kind {
        BackendKind::ForgeHxNative => 0,
        BackendKind::ForgeHxDsp => 1,
        BackendKind::LinuxStandard => 2,
        BackendKind::OpenRgb | BackendKind::Ratbag => 3,
        BackendKind::Diagnostic => 4,
    }
}

pub fn select_owner(
    capability: Capability,
    candidates: &[CapabilityOwner],
) -> Option<CapabilityOwner> {
    let mut candidates = candidates
        .iter()
        .filter(|candidate| candidate.capability == capability)
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| {
        (
            priority(candidate.backend),
            !candidate.writable,
            !candidate.verified,
        )
    });
    candidates.into_iter().next()
}

pub fn select_owner_with_health(
    capability: Capability,
    candidates: &[BackendCandidate],
) -> Option<CapabilityOwner> {
    let ready = candidates
        .iter()
        .filter(|candidate| candidate.health.is_ready())
        .map(|candidate| candidate.owner.clone())
        .collect::<Vec<_>>();
    select_owner(capability, &ready)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::{BackendHealth, BackendKind};

    fn owner(backend: BackendKind) -> CapabilityOwner {
        CapabilityOwner {
            capability: Capability::Lighting,
            backend,
            backend_device_id: Some("x".into()),
            verified: backend == BackendKind::ForgeHxNative,
            writable: true,
            detail: None,
        }
    }

    #[test]
    fn native_beats_compatibility_backend() {
        let selected = select_owner(
            Capability::Lighting,
            &[
                owner(BackendKind::OpenRgb),
                owner(BackendKind::ForgeHxNative),
            ],
        )
        .unwrap();
        assert_eq!(selected.backend, BackendKind::ForgeHxNative);
    }

    #[test]
    fn unhealthy_backend_is_rejected() {
        let selected = select_owner_with_health(
            Capability::Lighting,
            &[
                BackendCandidate {
                    owner: owner(BackendKind::OpenRgb),
                    health: BackendHealth::Disconnected,
                },
                BackendCandidate {
                    owner: owner(BackendKind::Diagnostic),
                    health: BackendHealth::Ready,
                },
            ],
        )
        .unwrap();
        assert_eq!(selected.backend, BackendKind::Diagnostic);
    }
}
