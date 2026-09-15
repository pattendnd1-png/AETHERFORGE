use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    Hidpp,
    Udev,
    Fwupd,
    V4l2,
    Pipewire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Ready,
    Degraded,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub kind: ServiceKind,
    pub state: ServiceState,
    pub backend: String,
    pub version: Option<String>,
    pub message: Option<String>,
    pub last_success_unix_ms: Option<u64>,
}

impl ServiceStatus {
    pub fn unavailable(kind: ServiceKind, backend: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind,
            state: ServiceState::Unavailable,
            backend: backend.into(),
            version: None,
            message: Some(message.into()),
            last_success_unix_ms: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HidppSessionState {
    Connecting,
    Ready,
    Degraded,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HidppSessionStatus {
    pub key: String,
    pub path: String,
    pub device_index: u8,
    pub state: HidppSessionState,
    pub reconnect_count: u32,
    pub last_success_unix_ms: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FirmwareDevice {
    pub id: String,
    pub name: String,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub plugin: Option<String>,
    #[serde(default)]
    pub guids: Vec<String>,
    #[serde(default)]
    pub instance_ids: Vec<String>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    #[serde(default)]
    pub matched_device_key: Option<String>,
    #[serde(default)]
    pub flags: Vec<String>,
    pub update_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FirmwareRelease {
    pub version: String,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub remote_id: Option<String>,
    pub uri: Option<String>,
    #[serde(default)]
    pub checksums: Vec<String>,
    pub trusted: Option<bool>,
    pub needs_reboot: bool,
    pub needs_replug: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareInstallResult {
    pub device_id: String,
    pub success: bool,
    pub message: String,
    pub needs_reboot: bool,
    pub needs_replug: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_status_round_trips() {
        let value = ServiceStatus {
            kind: ServiceKind::Fwupd,
            state: ServiceState::Ready,
            backend: "fwupdmgr".into(),
            version: Some("2.1.7".into()),
            message: None,
            last_success_unix_ms: Some(7),
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<ServiceStatus>(&json).unwrap(), value);
    }

    #[test]
    fn firmware_models_round_trip() {
        let device = FirmwareDevice {
            id: "abc".into(),
            name: "Logitech Receiver".into(),
            vendor: Some("Logitech".into()),
            version: Some("1.2".into()),
            plugin: Some("logitech_hidpp".into()),
            guids: vec!["guid".into()],
            instance_ids: vec!["USB\\VID_046D&PID_C548".into()],
            vendor_id: Some(0x046d),
            product_id: Some(0xc548),
            matched_device_key: Some("receiver".into()),
            flags: vec!["updatable".into()],
            update_available: true,
        };
        assert_eq!(serde_json::from_str::<FirmwareDevice>(&serde_json::to_string(&device).unwrap()).unwrap(), device);

        let release = FirmwareRelease {
            version: "2.0".into(),
            trusted: Some(true),
            needs_replug: true,
            ..Default::default()
        };
        assert_eq!(serde_json::from_str::<FirmwareRelease>(&serde_json::to_string(&release).unwrap()).unwrap(), release);
    }
}
