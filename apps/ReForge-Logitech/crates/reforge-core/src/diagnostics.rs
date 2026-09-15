use crate::{DeviceRuntimeInfo, IntegrationStatus, ProviderKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticEvent {
    pub unix_ms: u64,
    pub level: DiagnosticLevel,
    pub component: String,
    pub message: String,
    pub device_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DiagnosticSnapshot {
    pub package_version: String,
    pub daemon_version: String,
    pub runtime_socket_category: String,
    #[serde(default)]
    pub integrations: IntegrationStatus,
    #[serde(default)]
    pub devices: Vec<DeviceRuntimeInfo>,
    #[serde(default)]
    pub provider_health: BTreeMap<ProviderKind, bool>,
    #[serde(default)]
    pub recent_events: Vec<DiagnosticEvent>,
    pub profile_store_version: u32,
    pub profile_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticBundle {
    pub redacted: bool,
    pub snapshot: DiagnosticSnapshot,
}

fn stable_token(value: &str) -> String {
    // Deterministic FNV-1a token; diagnostics only, not cryptographic identity.
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("id-{hash:016x}")
}

fn redact_text(text: &str) -> String {
    if let Ok(home) = std::env::var("HOME")
        && !home.is_empty()
    {
        return text.replace(&home, "$HOME");
    }
    text.to_owned()
}

pub fn redact_snapshot(mut snapshot: DiagnosticSnapshot, include_device_identifiers: bool) -> DiagnosticBundle {
    for runtime in &mut snapshot.devices {
        runtime.summary.path = redact_text(&runtime.summary.path);
        if !include_device_identifiers {
            runtime.summary.key = stable_token(&runtime.summary.key);
            if let Some(serial) = runtime.summary.serial.as_mut() {
                *serial = stable_token(serial);
            }
            if let Some(hardware_id) = runtime.summary.hardware_id.as_mut() {
                *hardware_id = stable_token(hardware_id);
            }
        }
    }
    for event in &mut snapshot.recent_events {
        event.message = redact_text(&event.message);
        if !include_device_identifiers
            && let Some(key) = event.device_key.as_mut()
        {
            *key = stable_token(key);
        }
    }
    DiagnosticBundle {
        redacted: !include_device_identifiers,
        snapshot,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DeviceClass, DevicePresence, DeviceSummary, TransportKind};

    #[test]
    fn redaction_preserves_product_ids_and_hides_identifiers() {
        let summary = DeviceSummary {
            key: "device-key".into(), path: format!("{}/dev/hidraw0", std::env::var("HOME").unwrap_or_default()),
            device_index: 0xff, vendor_id: 0x046d, product_id: 0xc547, product: "G515".into(),
            serial: Some("secret-serial".into()), hardware_id: Some("unit-123".into()), interface_number: 0,
            usage_page: 0, usage: 0, transport: TransportKind::Usb, hidpp: true, device_class: DeviceClass::Keyboard,
            providers: vec![], controls: vec![], features: vec![], dpi: None, lighting: None, error: None,
        };
        let snapshot = DiagnosticSnapshot {
            package_version: "0.5.0".into(), daemon_version: "0.5.0".into(), runtime_socket_category: "$XDG_RUNTIME_DIR".into(),
            devices: vec![DeviceRuntimeInfo { summary, presence: DevicePresence::Online, first_seen_unix_ms: 1, last_seen_unix_ms: 2, reconnect_count: 0 }],
            ..Default::default()
        };
        let bundle = redact_snapshot(snapshot, false);
        let device = &bundle.snapshot.devices[0].summary;
        assert_eq!(device.product_id, 0xc547);
        assert_ne!(device.serial.as_deref(), Some("secret-serial"));
        assert_ne!(device.hardware_id.as_deref(), Some("unit-123"));
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
        {
            assert!(!device.path.contains(&home));
        }
    }
}
