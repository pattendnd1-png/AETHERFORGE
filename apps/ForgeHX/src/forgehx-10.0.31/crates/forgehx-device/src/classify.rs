use crate::RawInterface;
use forgehx_core::{DeviceClass, VendorFamily};

pub fn vendor_family(raw: &RawInterface) -> VendorFamily {
    let branding = format!(
        "{} {}",
        raw.manufacturer.as_deref().unwrap_or_default(),
        raw.product.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();

    if branding.contains("hyperx") {
        return VendorFamily::HyperX;
    }

    match raw.vendor_id {
        0x0951 => VendorFamily::Kingston,
        0x03f0 => VendorFamily::Hp,
        0x046d => VendorFamily::Logitech,
        0x1b1c => VendorFamily::Corsair,
        0x1532 => VendorFamily::Razer,
        0x1038 => VendorFamily::SteelSeries,
        _ if branding.contains("logitech") => VendorFamily::Logitech,
        _ if branding.contains("corsair") => VendorFamily::Corsair,
        _ if branding.contains("razer") => VendorFamily::Razer,
        _ if branding.contains("steelseries") => VendorFamily::SteelSeries,
        _ => raw
            .manufacturer
            .as_ref()
            .filter(|name| !name.trim().is_empty())
            .map(|name| VendorFamily::Generic(name.clone()))
            .unwrap_or(VendorFamily::Unknown),
    }
}

pub fn device_class(raw: &RawInterface) -> DeviceClass {
    if raw.usage_page == Some(0x01) {
        match raw.usage {
            Some(0x06) => return DeviceClass::Keyboard,
            Some(0x02) => return DeviceClass::Mouse,
            Some(0x04) | Some(0x05) => return DeviceClass::Controller,
            _ => {}
        }
    }

    let text = format!(
        "{} {}",
        raw.manufacturer.as_deref().unwrap_or_default(),
        raw.product.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase();
    if text.contains("keyboard") || text.contains("alloy") {
        return DeviceClass::Keyboard;
    }
    if text.contains("mouse") || text.contains("pulsefire") {
        return DeviceClass::Mouse;
    }
    if text.contains("headset") || text.contains("cloud") {
        return DeviceClass::Headset;
    }
    if text.contains("microphone") || text.contains("quadcast") || text.contains("solocast") {
        return DeviceClass::Microphone;
    }
    if text.contains("controller") || text.contains("gamepad") || text.contains("clutch") {
        return DeviceClass::Controller;
    }
    if text.contains("webcam") || text.contains("camera") || raw.usb_class_codes.contains(&0x0e) {
        return DeviceClass::Webcam;
    }
    if text.contains("mousepad") || text.contains("mouse pad") || text.contains("mat") {
        return DeviceClass::Mousepad;
    }
    if text.contains("monitor") || text.contains("display") {
        return DeviceClass::Monitor;
    }
    if raw.usb_class_codes.contains(&0x01) {
        return DeviceClass::UsbAudio;
    }
    if raw.source == forgehx_core::InterfaceSource::Hid {
        DeviceClass::GenericHid
    } else {
        DeviceClass::Other
    }
}

pub fn is_hyperx(raw: &RawInterface) -> bool {
    vendor_family(raw) == VendorFamily::HyperX
}
