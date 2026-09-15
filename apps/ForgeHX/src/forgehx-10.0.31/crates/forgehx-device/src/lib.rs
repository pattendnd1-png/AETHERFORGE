pub mod classify;
pub mod doctor;
pub mod drivers;
pub mod group;
pub mod hid;
pub mod power;
pub mod usb;

use forgehx_core::{Capability, DeviceInfo, DeviceInterface, ForgeHxError, InterfaceSource};
use thiserror::Error;

pub use doctor::doctor_reports;
pub use hid::{HidBackend, HidDescriptor, SystemHidBackend};
pub use power::{BatteryDescriptor, PowerBackend, SystemPowerBackend};
pub use usb::{SystemUsbBackend, UsbBackend, UsbDescriptor};

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("HID enumeration failed: {0}")]
    Hid(String),
    #[error("USB enumeration failed: {0}")]
    Usb(String),
    #[error("power metadata enumeration failed: {0}")]
    Power(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawInterface {
    pub source: InterfaceSource,
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub interface_number: Option<i32>,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
    pub audio_node_id: Option<u32>,
    pub usb_parent: Option<String>,
    pub usb_class_codes: Vec<u8>,
}

impl From<&RawInterface> for DeviceInterface {
    fn from(raw: &RawInterface) -> Self {
        Self {
            source: raw.source,
            path: raw.path.clone(),
            vendor_id: (raw.vendor_id != 0).then_some(raw.vendor_id),
            product_id: (raw.product_id != 0).then_some(raw.product_id),
            interface_number: raw.interface_number,
            usage_page: raw.usage_page,
            usage: raw.usage,
            audio_node_id: raw.audio_node_id,
            usb_parent: raw.usb_parent.clone(),
        }
    }
}

impl From<HidDescriptor> for RawInterface {
    fn from(value: HidDescriptor) -> Self {
        Self {
            source: InterfaceSource::Hid,
            path: value.path,
            vendor_id: value.vendor_id,
            product_id: value.product_id,
            manufacturer: value.manufacturer,
            product: value.product,
            serial: value.serial,
            interface_number: Some(value.interface_number),
            usage_page: Some(value.usage_page),
            usage: Some(value.usage),
            audio_node_id: None,
            usb_parent: value.usb_parent,
            usb_class_codes: Vec::new(),
        }
    }
}

impl From<UsbDescriptor> for RawInterface {
    fn from(value: UsbDescriptor) -> Self {
        Self {
            source: InterfaceSource::Usb,
            path: value.path,
            vendor_id: value.vendor_id,
            product_id: value.product_id,
            manufacturer: value.manufacturer,
            product: value.product,
            serial: value.serial,
            interface_number: None,
            usage_page: None,
            usage: None,
            audio_node_id: None,
            usb_parent: Some(value.usb_parent),
            usb_class_codes: value.class_codes,
        }
    }
}

pub trait DiscoveryBackend: Send + Sync {
    fn enumerate_hid(&self) -> Result<Vec<HidDescriptor>, DeviceError>;
    fn enumerate_usb(&self) -> Result<Vec<UsbDescriptor>, DeviceError>;
    fn enumerate_batteries(&self) -> Result<Vec<BatteryDescriptor>, DeviceError> {
        Ok(Vec::new())
    }
}

#[derive(Debug, Default, Clone)]
pub struct SystemDiscoveryBackend {
    hid: SystemHidBackend,
    usb: SystemUsbBackend,
    power: SystemPowerBackend,
}

impl DiscoveryBackend for SystemDiscoveryBackend {
    fn enumerate_hid(&self) -> Result<Vec<HidDescriptor>, DeviceError> {
        self.hid.enumerate()
    }

    fn enumerate_usb(&self) -> Result<Vec<UsbDescriptor>, DeviceError> {
        self.usb.enumerate()
    }

    fn enumerate_batteries(&self) -> Result<Vec<BatteryDescriptor>, DeviceError> {
        self.power.enumerate()
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    pub info: DeviceInfo,
    pub raw_interfaces: Vec<RawInterface>,
}

pub fn discover_devices(
    backend: &dyn DiscoveryBackend,
) -> Result<Vec<DiscoveredDevice>, DeviceError> {
    let hid = backend.enumerate_hid();
    let usb = backend.enumerate_usb();
    if hid.is_err() && usb.is_err() {
        return Err(DeviceError::Usb(format!(
            "all discovery sources failed; HID: {}; USB: {}",
            hid.as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_default(),
            usb.as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_default()
        )));
    }
    let mut raw = hid
        .unwrap_or_default()
        .into_iter()
        .map(RawInterface::from)
        .collect::<Vec<_>>();
    raw.extend(usb.unwrap_or_default().into_iter().map(RawInterface::from));
    let mut devices = group::group_interfaces(raw);
    if let Ok(batteries) = backend.enumerate_batteries() {
        attach_battery_metadata(&mut devices, &batteries);
    }
    Ok(devices)
}

fn attach_battery_metadata(devices: &mut [DiscoveredDevice], batteries: &[BatteryDescriptor]) {
    for battery in batteries {
        let model = battery
            .model_name
            .as_deref()
            .map(normalize_identity)
            .unwrap_or_default();
        let serial = battery
            .serial
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let Some(device) = devices.iter_mut().find(|device| {
            let serial_match = serial
                .zip(device.info.serial.as_deref())
                .map(|(left, right)| left.eq_ignore_ascii_case(right.trim()))
                .unwrap_or(false);
            let name = normalize_identity(&device.info.name);
            serial_match
                || (!model.is_empty()
                    && !name.is_empty()
                    && (model.contains(&name) || name.contains(&model)))
        }) else {
            continue;
        };
        device.info.battery_percent = battery.capacity;
        device.info.battery_state = battery.status.clone();
        if !device
            .info
            .generic_capabilities
            .contains(&Capability::BatteryStatus)
        {
            device
                .info
                .generic_capabilities
                .push(Capability::BatteryStatus);
        }
    }
}

fn normalize_identity(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

pub fn hyperx_devices(devices: &[DiscoveredDevice]) -> Vec<DiscoveredDevice> {
    devices
        .iter()
        .filter(|device| device.info.is_hyperx())
        .cloned()
        .collect()
}

pub fn ensure_write_allowed(info: &DeviceInfo, capability: Capability) -> Result<(), ForgeHxError> {
    if !info.supports_vendor(capability) {
        return Err(ForgeHxError::Unsupported(capability));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::{DeviceClass, SupportLevel, VendorFamily};

    #[derive(Debug, Clone)]
    struct FakeDiscovery {
        hid: Vec<HidDescriptor>,
        usb: Vec<UsbDescriptor>,
    }

    impl DiscoveryBackend for FakeDiscovery {
        fn enumerate_hid(&self) -> Result<Vec<HidDescriptor>, DeviceError> {
            Ok(self.hid.clone())
        }
        fn enumerate_usb(&self) -> Result<Vec<UsbDescriptor>, DeviceError> {
            Ok(self.usb.clone())
        }
    }

    #[derive(Debug, Clone)]
    struct FakeBatteryDiscovery {
        hid: Vec<HidDescriptor>,
        batteries: Vec<BatteryDescriptor>,
    }

    impl DiscoveryBackend for FakeBatteryDiscovery {
        fn enumerate_hid(&self) -> Result<Vec<HidDescriptor>, DeviceError> {
            Ok(self.hid.clone())
        }
        fn enumerate_usb(&self) -> Result<Vec<UsbDescriptor>, DeviceError> {
            Ok(Vec::new())
        }
        fn enumerate_batteries(&self) -> Result<Vec<BatteryDescriptor>, DeviceError> {
            Ok(self.batteries.clone())
        }
    }

    fn hid(
        vendor_id: u16,
        product_id: u16,
        manufacturer: &str,
        product: &str,
        serial: &str,
        usage: u16,
    ) -> HidDescriptor {
        HidDescriptor {
            path: format!("/dev/hidraw-{serial}-{usage}"),
            vendor_id,
            product_id,
            manufacturer: Some(manufacturer.into()),
            product: Some(product.into()),
            serial: Some(serial.into()),
            interface_number: usage as i32,
            usage_page: 0x01,
            usage,
            usb_parent: None,
        }
    }

    #[test]
    fn universal_discovery_keeps_non_hyperx_hid() {
        let backend = FakeDiscovery {
            hid: vec![hid(
                0x046d,
                0xc539,
                "Logitech",
                "Gaming Mouse",
                "LOGI1",
                0x02,
            )],
            usb: Vec::new(),
        };
        let devices = discover_devices(&backend).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].info.vendor_family, VendorFamily::Logitech);
        assert_eq!(devices[0].info.device_class, DeviceClass::Mouse);
    }

    #[test]
    fn hp_device_requires_hyperx_evidence() {
        let ordinary = RawInterface::from(hid(0x03f0, 0x1111, "HP", "USB Keyboard", "HP1", 0x06));
        let hyperx =
            RawInterface::from(hid(0x03f0, 0x2222, "HP", "HyperX Alloy Test", "HX1", 0x06));
        assert_eq!(classify::vendor_family(&ordinary), VendorFamily::Hp);
        assert_eq!(classify::vendor_family(&hyperx), VendorFamily::HyperX);
    }

    #[test]
    fn same_serial_groups_multiple_interfaces() {
        let backend = FakeDiscovery {
            hid: vec![
                hid(0x0951, 0x1234, "HyperX", "HyperX Test", "SERIAL", 0x06),
                hid(0x0951, 0x1234, "HyperX", "HyperX Test", "SERIAL", 0x01),
            ],
            usb: Vec::new(),
        };
        let devices = discover_devices(&backend).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].info.interfaces.len(), 2);
    }

    #[test]
    fn shared_usb_parent_groups_hid_and_usb_interfaces() {
        let mut h = hid(0x0951, 0x3333, "HyperX", "HyperX Composite", "", 0x06);
        h.serial = None;
        h.usb_parent = Some("1-4".into());
        let backend = FakeDiscovery {
            hid: vec![h],
            usb: vec![UsbDescriptor {
                path: "/sys/bus/usb/devices/1-4".into(),
                usb_parent: "1-4".into(),
                vendor_id: 0x0951,
                product_id: 0x3333,
                manufacturer: Some("HyperX".into()),
                product: Some("HyperX Composite".into()),
                serial: None,
                class_codes: vec![0x03],
            }],
        };
        let devices = discover_devices(&backend).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].info.interfaces.len(), 2);
        assert_eq!(
            devices[0].info.connection_kind,
            forgehx_core::ConnectionKind::Composite
        );
    }

    #[test]
    fn distinct_serials_do_not_merge_same_model() {
        let backend = FakeDiscovery {
            hid: vec![
                hid(0x0951, 0x1234, "HyperX", "HyperX Test", "ONE", 0x06),
                hid(0x0951, 0x1234, "HyperX", "HyperX Test", "TWO", 0x06),
            ],
            usb: Vec::new(),
        };
        assert_eq!(discover_devices(&backend).unwrap().len(), 2);
    }

    #[test]
    fn matching_power_supply_adds_safe_battery_status() {
        let backend = FakeBatteryDiscovery {
            hid: vec![hid(
                0x046d,
                0xc539,
                "Logitech",
                "Gaming Mouse",
                "BAT123",
                0x02,
            )],
            batteries: vec![BatteryDescriptor {
                path: "/sys/class/power_supply/hid-test-battery".into(),
                name: "hid-test-battery".into(),
                manufacturer: Some("Logitech".into()),
                model_name: Some("Gaming Mouse".into()),
                serial: Some("BAT123".into()),
                capacity: Some(64),
                status: Some("Discharging".into()),
            }],
        };
        let device = discover_devices(&backend).unwrap().remove(0);
        assert_eq!(device.info.battery_percent, Some(64));
        assert_eq!(device.info.battery_state.as_deref(), Some("Discharging"));
        assert!(device.info.supports_generic(Capability::BatteryStatus));
    }

    #[test]
    fn unknown_driver_is_diagnostic_only_and_write_protected() {
        let backend = FakeDiscovery {
            hid: vec![hid(0x0951, 0x1234, "HyperX", "HyperX Test", "SERIAL", 0x06)],
            usb: Vec::new(),
        };
        let device = discover_devices(&backend).unwrap().remove(0);
        assert_eq!(device.info.support_level, SupportLevel::DiagnosticOnly);
        assert!(ensure_write_allowed(&device.info, Capability::Lighting).is_err());
    }
}
