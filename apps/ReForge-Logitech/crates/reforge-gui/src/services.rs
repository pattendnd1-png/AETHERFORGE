use reforge_core::{ServiceKind, ServiceState};

pub fn kind_label(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Hidpp => "Logitech Device Service",
        ServiceKind::Udev => "Linux Hotplug Service",
        ServiceKind::Fwupd => "Firmware Service",
        ServiceKind::V4l2 => "Camera Service",
        ServiceKind::Pipewire => "Audio Service",
    }
}

pub fn state_label(state: ServiceState) -> &'static str {
    match state {
        ServiceState::Ready => "READY",
        ServiceState::Degraded => "DEGRADED",
        ServiceState::Unavailable => "UNAVAILABLE",
        ServiceState::Error => "ERROR",
    }
}
