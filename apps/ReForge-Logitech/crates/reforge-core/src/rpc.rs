use crate::{ControlValue, DeviceRuntimeInfo, DeviceSummary, DeviceTelemetry, DiagnosticSnapshot, DpiInfo, FirmwareDevice, FirmwareInstallResult, FirmwareRelease, HidppSessionStatus, IntegrationStatus, LightingState, Profile, ServiceKind, ServiceStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum RpcRequest {
    Health,
    ListDevices,
    SetDpi { key: String, dpi: u16 },
    GetLighting { key: String },
    SetLighting { key: String, state: LightingState },
    SetKeyColor { key: String, key_id: u8, color: crate::RgbColor },
    GetControl { key: String, control_id: String },
    SetControl { key: String, control_id: String, value: ControlValue },
    ListProfiles,
    SaveProfile { profile: Profile },
    ApplyProfile { profile_name: String, key: String },
    IntegrationStatus,
    RescanDevices,
    ListDeviceRuntime,
    GetDeviceTelemetry { key: String },
    RenameProfile { old_name: String, new_name: String },
    DeleteProfile { name: String },
    CloneProfile { source_name: String, new_name: String },
    ImportProfiles { json: String, replace: bool },
    ExportProfiles { names: Vec<String> },
    GetDiagnostics,
    ClearDiagnostics,
    ListServices,
    ReconnectService { service: ServiceKind },
    GetSessionStatus { key: String },
    ListFirmwareDevices,
    RefreshFirmwareMetadata,
    GetFirmwareReleases { device_id: String },
    InstallFirmwareUpdate { device_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum RpcData {
    Health { version: String },
    Devices(Vec<DeviceSummary>),
    Dpi(DpiInfo),
    Lighting(LightingState),
    Control(ControlValue),
    Profiles(Vec<Profile>),
    Integrations(IntegrationStatus),
    DeviceRuntime(Vec<DeviceRuntimeInfo>),
    Telemetry(DeviceTelemetry),
    ExportedProfiles { json: String },
    Diagnostics(DiagnosticSnapshot),
    Services(Vec<ServiceStatus>),
    SessionStatus(HidppSessionStatus),
    FirmwareDevices(Vec<FirmwareDevice>),
    FirmwareReleases(Vec<FirmwareRelease>),
    FirmwareInstall(FirmwareInstallResult),
    Ack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RpcResponse {
    Ok { data: RpcData },
    Err { message: String },
}

impl RpcResponse {
    pub fn ok(data: RpcData) -> Self { Self::Ok { data } }
    pub fn error(message: impl Into<String>) -> Self { Self::Err { message: message.into() } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lighting_rpc_round_trips_as_json() {
        let request = RpcRequest::SetLighting { key: "device".into(), state: LightingState::default() };
        let json = serde_json::to_string(&request).unwrap();
        let decoded: RpcRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn vector_control_rpc_round_trips_as_json() {
        let request = RpcRequest::SetControl {
            key: "headset".into(),
            control_id: "hidpp:eq".into(),
            value: ControlValue::Vector(vec![0, 2, -1, 4]),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(serde_json::from_str::<RpcRequest>(&json).unwrap(), request);
    }

    #[test]
    fn reliability_rpc_variants_round_trip() {
        let requests = vec![
            RpcRequest::RescanDevices,
            RpcRequest::ListDeviceRuntime,
            RpcRequest::GetDeviceTelemetry { key: "device".into() },
            RpcRequest::RenameProfile { old_name: "A".into(), new_name: "B".into() },
            RpcRequest::DeleteProfile { name: "B".into() },
            RpcRequest::CloneProfile { source_name: "A".into(), new_name: "Copy".into() },
            RpcRequest::ImportProfiles { json: "[]".into(), replace: false },
            RpcRequest::ExportProfiles { names: vec!["A".into()] },
            RpcRequest::GetDiagnostics,
            RpcRequest::ClearDiagnostics,
            RpcRequest::ListServices,
            RpcRequest::ReconnectService { service: ServiceKind::Hidpp },
            RpcRequest::GetSessionStatus { key: "device".into() },
            RpcRequest::ListFirmwareDevices,
            RpcRequest::RefreshFirmwareMetadata,
            RpcRequest::GetFirmwareReleases { device_id: "fwupd-id".into() },
            RpcRequest::InstallFirmwareUpdate { device_id: "fwupd-id".into() },
        ];
        for request in requests {
            let json = serde_json::to_string(&request).unwrap();
            assert_eq!(serde_json::from_str::<RpcRequest>(&json).unwrap(), request);
        }
    }

    #[test]
    fn reliability_rpc_data_variants_round_trip() {
        let values = vec![
            RpcData::DeviceRuntime(vec![]),
            RpcData::Telemetry(DeviceTelemetry::default()),
            RpcData::ExportedProfiles { json: "{\"version\":4,\"profiles\":[]}".into() },
            RpcData::Diagnostics(DiagnosticSnapshot::default()),
            RpcData::Services(vec![]),
            RpcData::SessionStatus(HidppSessionStatus {
                key: "device".into(), path: "/dev/hidraw0".into(), device_index: 0xff,
                state: crate::HidppSessionState::Ready, reconnect_count: 0,
                last_success_unix_ms: Some(1), last_error: None,
            }),
            RpcData::FirmwareDevices(vec![]),
            RpcData::FirmwareReleases(vec![]),
            RpcData::FirmwareInstall(FirmwareInstallResult {
                device_id: "fwupd-id".into(), success: true, message: "accepted".into(),
                needs_reboot: false, needs_replug: false,
            }),
        ];
        for value in values {
            let json = serde_json::to_string(&value).unwrap();
            assert_eq!(serde_json::from_str::<RpcData>(&json).unwrap(), value);
        }
    }
}
