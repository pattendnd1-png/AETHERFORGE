use crate::{classify, drivers, DiscoveredDevice, RawInterface};
use forgehx_core::{
    Capability, ConnectionKind, DeviceClass, DeviceId, DeviceInfo, DeviceInterface,
    InterfaceSource, SupportLevel, VendorFamily,
};
use std::collections::BTreeMap;

pub fn group_interfaces(interfaces: Vec<RawInterface>) -> Vec<DiscoveredDevice> {
    let mut groups: BTreeMap<String, Vec<RawInterface>> = BTreeMap::new();
    for raw in interfaces {
        groups.entry(group_key(&raw)).or_default().push(raw);
    }

    let mut devices = groups.into_values().map(build_device).collect::<Vec<_>>();
    devices.sort_by(|a, b| {
        a.info
            .name
            .cmp(&b.info.name)
            .then(a.info.id.0.cmp(&b.info.id.0))
    });
    devices
}

fn group_key(raw: &RawInterface) -> String {
    if let Some(serial) = raw.serial.as_deref().filter(|s| !s.trim().is_empty()) {
        return format!(
            "serial:{:04x}:{:04x}:{}",
            raw.vendor_id,
            raw.product_id,
            sanitize(serial)
        );
    }
    if let Some(parent) = raw.usb_parent.as_deref().filter(|s| !s.trim().is_empty()) {
        return format!(
            "parent:{:04x}:{:04x}:{}",
            raw.vendor_id,
            raw.product_id,
            sanitize(parent)
        );
    }
    format!(
        "path:{:04x}:{:04x}:{}",
        raw.vendor_id,
        raw.product_id,
        sanitize(&raw.path)
    )
}

fn build_device(mut raws: Vec<RawInterface>) -> DiscoveredDevice {
    raws.sort_by(|a, b| a.path.cmp(&b.path));
    let primary = raws
        .iter()
        .max_by_key(|raw| score_class(classify::device_class(raw)))
        .cloned()
        .unwrap_or_else(|| raws[0].clone());
    let driver = drivers::driver_for(primary.vendor_id, primary.product_id);
    let mut capabilities = vec![Capability::Diagnostics];
    if let Some(driver) = driver.as_ref() {
        capabilities.extend(driver.capabilities.iter().copied());
    }
    capabilities.sort();
    capabilities.dedup();

    let vendor_family = merge_vendor(&raws);
    let device_class = merge_class(&raws);
    let connection_kind = connection_kind(&raws);
    let interfaces = raws.iter().map(DeviceInterface::from).collect::<Vec<_>>();
    let serial = raws.iter().find_map(|raw| raw.serial.clone());
    let manufacturer = raws.iter().find_map(|raw| raw.manufacturer.clone());
    let name = raws
        .iter()
        .find_map(|raw| raw.product.clone().filter(|name| !name.trim().is_empty()))
        .or_else(|| manufacturer.clone())
        .unwrap_or_else(|| format!("USB {:04x}:{:04x}", primary.vendor_id, primary.product_id));
    let id_seed = serial
        .clone()
        .or_else(|| raws.iter().find_map(|raw| raw.usb_parent.clone()))
        .unwrap_or_else(|| primary.path.clone());
    let support_level = if let Some(driver) = driver.as_ref() {
        if driver.complete {
            SupportLevel::FullySupported
        } else {
            SupportLevel::PartiallySupported
        }
    } else {
        SupportLevel::DiagnosticOnly
    };

    DiscoveredDevice {
        info: DeviceInfo {
            id: DeviceId(format!(
                "{:04x}:{:04x}:{}",
                primary.vendor_id,
                primary.product_id,
                sanitize(&id_seed)
            )),
            name,
            manufacturer,
            vendor_id: primary.vendor_id,
            product_id: primary.product_id,
            serial,
            vendor_family,
            device_class,
            support_level,
            connection_kind,
            interfaces,
            capabilities,
            generic_capabilities: Vec::new(),
            capability_owners: Vec::new(),
            battery_percent: None,
            battery_state: None,
            protocol: driver.as_ref().map(|driver| driver.id.to_owned()),
        },
        raw_interfaces: raws,
    }
}

fn merge_vendor(raws: &[RawInterface]) -> VendorFamily {
    let families = raws.iter().map(classify::vendor_family).collect::<Vec<_>>();
    if families
        .iter()
        .any(|family| family == &VendorFamily::HyperX)
    {
        return VendorFamily::HyperX;
    }
    families.into_iter().next().unwrap_or(VendorFamily::Unknown)
}

fn merge_class(raws: &[RawInterface]) -> DeviceClass {
    raws.iter()
        .map(classify::device_class)
        .max_by_key(|class| score_class(*class))
        .unwrap_or(DeviceClass::Other)
}

fn score_class(class: DeviceClass) -> u8 {
    match class {
        DeviceClass::Keyboard
        | DeviceClass::Mouse
        | DeviceClass::Headset
        | DeviceClass::Microphone => 5,
        DeviceClass::Controller
        | DeviceClass::Webcam
        | DeviceClass::Mousepad
        | DeviceClass::Monitor => 4,
        DeviceClass::AudioInterface | DeviceClass::UsbAudio => 3,
        DeviceClass::GenericHid => 2,
        DeviceClass::Other => 1,
    }
}

fn connection_kind(raws: &[RawInterface]) -> ConnectionKind {
    let has_hid = raws.iter().any(|raw| raw.source == InterfaceSource::Hid);
    let has_usb = raws.iter().any(|raw| raw.source == InterfaceSource::Usb);
    let has_audio = raws.iter().any(|raw| raw.source == InterfaceSource::Audio);
    match (has_hid, has_usb, has_audio) {
        (true, false, false) => ConnectionKind::Hid,
        (false, true, false) => ConnectionKind::Usb,
        (false, false, true) => ConnectionKind::Audio,
        _ => ConnectionKind::Composite,
    }
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
