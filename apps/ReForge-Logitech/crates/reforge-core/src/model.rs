use crate::LightingCapabilities;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Usb,
    Bluetooth,
    I2c,
    Spi,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClass {
    Keyboard,
    Mouse,
    Camera,
    Audio,
    Receiver,
    Controller,
    Other,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Hidpp,
    Hid,
    Usb,
    V4l2,
    Pipewire,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    Toggle,
    Range,
    Choice,
    Action,
    Vector,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ControlGroup {
    Overview,
    Assignments,
    Performance,
    Lighting,
    Audio,
    Camera,
    Power,
    Profiles,
    Receiver,
    #[default]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReadbackKind {
    #[default]
    Readable,
    CanonicalWriteOnly,
    Telemetry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum ControlValue {
    Bool(bool),
    Int(i64),
    Vector(Vec<i64>),
    #[default]
    None,
}

impl ControlValue {
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Bool(value) => Some(if *value { 1 } else { 0 }),
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            Self::Int(value) => Some(*value != 0),
            _ => None,
        }
    }

    pub fn as_vector(&self) -> Option<&[i64]> {
        match self {
            Self::Vector(values) => Some(values),
            _ => None,
        }
    }
}

impl From<i64> for ControlValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<bool> for ControlValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlChoice {
    pub value: i64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceControl {
    pub id: String,
    pub label: String,
    pub provider: ProviderKind,
    pub endpoint: String,
    pub backend_id: String,
    pub kind: ControlKind,
    #[serde(default)]
    pub value: ControlValue,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub step: Option<i64>,
    #[serde(default)]
    pub choices: Vec<ControlChoice>,
    #[serde(default)]
    pub writable: bool,
    #[serde(default)]
    pub profile_eligible: bool,
    #[serde(default)]
    pub group: ControlGroup,
    #[serde(default)]
    pub readback: ReadbackKind,
}

impl DeviceControl {
    pub fn scalar_value(&self) -> Option<i64> {
        self.value.as_i64()
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DevicePresence {
    #[default]
    Online,
    Offline,
    Reconnecting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRuntimeInfo {
    pub summary: DeviceSummary,
    pub presence: DevicePresence,
    pub first_seen_unix_ms: u64,
    pub last_seen_unix_ms: u64,
    pub reconnect_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DeviceTelemetry {
    pub key: String,
    #[serde(default)]
    pub presence: DevicePresence,
    pub battery_percent: Option<u8>,
    pub charging: Option<bool>,
    pub active_dpi: Option<u16>,
    #[serde(default)]
    pub provider_health: BTreeMap<ProviderKind, bool>,
    pub updated_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DesiredDeviceState {
    pub dpi: Option<u16>,
    pub lighting: Option<crate::LightingState>,
    #[serde(default)]
    pub controls: BTreeMap<String, ControlValue>,
    pub profile_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureInfo {
    pub index: u8,
    pub feature_id: u16,
    pub name: String,
    pub metadata: u8,
    #[serde(default)]
    pub version: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DpiInfo {
    pub sensor: u8,
    pub min: u16,
    pub max: u16,
    pub step: u16,
    pub current: u16,
    pub default: u16,
    pub supported: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSummary {
    pub key: String,
    pub path: String,
    pub device_index: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub product: String,
    pub serial: Option<String>,
    #[serde(default)]
    pub hardware_id: Option<String>,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
    pub transport: TransportKind,
    pub hidpp: bool,
    #[serde(default)]
    pub device_class: DeviceClass,
    #[serde(default)]
    pub providers: Vec<ProviderKind>,
    #[serde(default)]
    pub controls: Vec<DeviceControl>,
    pub features: Vec<FeatureInfo>,
    pub dpi: Option<DpiInfo>,
    #[serde(default)]
    pub lighting: Option<LightingCapabilities>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_values_round_trip() {
        for value in [
            ControlValue::Bool(true),
            ControlValue::Int(42),
            ControlValue::Vector(vec![100, 0, -2, 250]),
            ControlValue::None,
        ] {
            let json = serde_json::to_string(&value).unwrap();
            assert_eq!(serde_json::from_str::<ControlValue>(&json).unwrap(), value);
        }
    }

    #[test]
    fn numeric_values_remain_legacy_compatible() {
        assert_eq!(serde_json::from_str::<ControlValue>("42").unwrap(), ControlValue::Int(42));
    }

    #[test]
    fn legacy_device_summary_json_keeps_defaulted_additive_fields() {
        let json = r#"{"key":"legacy","path":"/dev/hidraw0","device_index":255,"vendor_id":1133,"product_id":1,"product":"Keyboard","serial":null,"interface_number":0,"usage_page":0,"usage":0,"transport":"usb","hidpp":true,"features":[],"dpi":null,"error":null}"#;
        let summary: DeviceSummary = serde_json::from_str(json).unwrap();
        assert_eq!(summary.key, "legacy");
        assert!(summary.providers.is_empty());
        assert!(summary.controls.is_empty());
        assert!(summary.hardware_id.is_none());
        assert!(summary.lighting.is_none());
    }

    #[test]
    fn runtime_and_telemetry_round_trip() {
        let summary = DeviceSummary {
            key: "device".into(), path: "/dev/hidraw0".into(), device_index: 0xff, vendor_id: 0x046d,
            product_id: 1, product: "Keyboard".into(), serial: None, hardware_id: None, interface_number: 0,
            usage_page: 0, usage: 0, transport: TransportKind::Usb, hidpp: true, device_class: DeviceClass::Keyboard,
            providers: vec![ProviderKind::Hidpp], controls: vec![], features: vec![], dpi: None, lighting: None, error: None,
        };
        let runtime = DeviceRuntimeInfo {
            summary, presence: DevicePresence::Reconnecting, first_seen_unix_ms: 1, last_seen_unix_ms: 2, reconnect_count: 3,
        };
        let json = serde_json::to_string(&runtime).unwrap();
        assert_eq!(serde_json::from_str::<DeviceRuntimeInfo>(&json).unwrap(), runtime);

        let telemetry = DeviceTelemetry {
            key: "device".into(), presence: DevicePresence::Online, battery_percent: Some(87), charging: Some(false),
            active_dpi: Some(1600), provider_health: BTreeMap::from([(ProviderKind::Hidpp, true)]), updated_unix_ms: 4,
        };
        let json = serde_json::to_string(&telemetry).unwrap();
        assert_eq!(serde_json::from_str::<DeviceTelemetry>(&json).unwrap(), telemetry);
    }
}
