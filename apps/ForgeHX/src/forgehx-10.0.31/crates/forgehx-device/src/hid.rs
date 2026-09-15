use crate::DeviceError;
use hidapi::HidApi;
use std::ffi::CStr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidDescriptor {
    pub path: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
    pub usb_parent: Option<String>,
}

impl HidDescriptor {
    pub fn new(vendor_id: u16, product_id: u16, product: impl Into<String>) -> Self {
        Self {
            path: format!("test:{vendor_id:04x}:{product_id:04x}"),
            vendor_id,
            product_id,
            manufacturer: None,
            product: Some(product.into()),
            serial: None,
            interface_number: 0,
            usage_page: 0,
            usage: 0,
            usb_parent: None,
        }
    }
}

pub trait HidBackend: Send + Sync {
    fn enumerate(&self) -> Result<Vec<HidDescriptor>, DeviceError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemHidBackend;

impl HidBackend for SystemHidBackend {
    fn enumerate(&self) -> Result<Vec<HidDescriptor>, DeviceError> {
        let api = HidApi::new().map_err(|e| DeviceError::Hid(e.to_string()))?;
        Ok(api
            .device_list()
            .map(|device| {
                let path = cstr_to_string(device.path());
                HidDescriptor {
                    usb_parent: usb_parent_for_hid_path(&path),
                    path,
                    vendor_id: device.vendor_id(),
                    product_id: device.product_id(),
                    manufacturer: device.manufacturer_string().map(str::to_owned),
                    product: device.product_string().map(str::to_owned),
                    serial: device.serial_number().map(str::to_owned),
                    interface_number: device.interface_number(),
                    usage_page: device.usage_page(),
                    usage: device.usage(),
                }
            })
            .collect())
    }
}

fn cstr_to_string(value: &CStr) -> String {
    value.to_string_lossy().into_owned()
}

fn usb_parent_for_hid_path(path: &str) -> Option<String> {
    let name = Path::new(path).file_name()?.to_str()?;
    if !name.starts_with("hidraw") {
        return None;
    }
    let start = PathBuf::from("/sys/class/hidraw").join(name).join("device");
    let canonical = std::fs::canonicalize(start).ok()?;
    for ancestor in canonical.ancestors() {
        if ancestor.join("idVendor").is_file() && ancestor.join("idProduct").is_file() {
            return ancestor.file_name()?.to_str().map(str::to_owned);
        }
    }
    None
}
