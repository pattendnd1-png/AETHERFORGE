use reforge_core::{FirmwareDevice, FirmwareRelease};

pub fn device_version(device: &FirmwareDevice) -> &str {
    device.version.as_deref().unwrap_or("unknown")
}

pub fn release_title(release: &FirmwareRelease) -> String {
    match release.name.as_deref() {
        Some(name) if !name.is_empty() => format!("{name} — {}", release.version),
        _ => format!("Firmware {}", release.version),
    }
}
