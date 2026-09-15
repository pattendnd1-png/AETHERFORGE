use crate::DeviceError;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbDescriptor {
    pub path: String,
    pub usb_parent: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub class_codes: Vec<u8>,
}

pub trait UsbBackend: Send + Sync {
    fn enumerate(&self) -> Result<Vec<UsbDescriptor>, DeviceError>;
}

#[derive(Debug, Default, Clone)]
pub struct SystemUsbBackend {
    pub root: Option<PathBuf>,
}

impl SystemUsbBackend {
    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
        }
    }
}

impl UsbBackend for SystemUsbBackend {
    fn enumerate(&self) -> Result<Vec<UsbDescriptor>, DeviceError> {
        enumerate_usb_root(
            self.root
                .as_deref()
                .unwrap_or(Path::new("/sys/bus/usb/devices")),
        )
    }
}

pub fn enumerate_usb_root(root: &Path) -> Result<Vec<UsbDescriptor>, DeviceError> {
    let entries =
        fs::read_dir(root).map_err(|e| DeviceError::Usb(format!("{}: {e}", root.display())))?;
    let mut devices = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.contains(':') || name.starts_with("usb") {
            continue;
        }
        let Some(vendor_id) = read_hex_u16(path.join("idVendor")) else {
            continue;
        };
        let Some(product_id) = read_hex_u16(path.join("idProduct")) else {
            continue;
        };
        let mut class_codes = Vec::new();
        if let Some(code) = read_hex_u8(path.join("bDeviceClass")) {
            if code != 0 {
                class_codes.push(code);
            }
        }
        if let Ok(children) = fs::read_dir(root) {
            let prefix = format!("{name}:");
            for child in children.flatten() {
                let child_path = child.path();
                let child_name = child_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();
                if child_name.starts_with(&prefix) {
                    if let Some(code) = read_hex_u8(child_path.join("bInterfaceClass")) {
                        class_codes.push(code);
                    }
                }
            }
        }
        class_codes.sort_unstable();
        class_codes.dedup();
        devices.push(UsbDescriptor {
            path: path.to_string_lossy().into_owned(),
            usb_parent: name.to_owned(),
            vendor_id,
            product_id,
            manufacturer: read_trimmed(path.join("manufacturer")),
            product: read_trimmed(path.join("product")),
            serial: read_trimmed(path.join("serial")),
            class_codes,
        });
    }
    devices.sort_by(|a, b| a.usb_parent.cmp(&b.usb_parent));
    Ok(devices)
}

fn read_trimmed(path: PathBuf) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn read_hex_u16(path: PathBuf) -> Option<u16> {
    u16::from_str_radix(read_trimmed(path)?.trim_start_matches("0x"), 16).ok()
}

fn read_hex_u8(path: PathBuf) -> Option<u8> {
    u8::from_str_radix(read_trimmed(path)?.trim_start_matches("0x"), 16).ok()
}
