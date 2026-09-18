use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsbDevice {
    pub sysfs_path: PathBuf,
    pub manufacturer: String,
    pub product: String,
    pub vendor_id: String,
    pub product_id: String,
    pub serial: String,
    pub bus_number: String,
    pub device_number: String,
}

pub fn discover_beacn_devices() -> io::Result<Vec<UsbDevice>> {
    discover_in(Path::new("/sys/bus/usb/devices"))
}

pub fn discover_in(root: &Path) -> io::Result<Vec<UsbDevice>> {
    let mut devices = Vec::new();
    if !root.exists() {
        return Ok(devices);
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manufacturer = read_trimmed(&path.join("manufacturer"));
        let product = read_trimmed(&path.join("product"));
        let searchable = format!("{manufacturer} {product}").to_ascii_lowercase();
        if !searchable.contains("beacn") {
            continue;
        }

        devices.push(UsbDevice {
            sysfs_path: path.clone(),
            manufacturer,
            product,
            vendor_id: read_trimmed(&path.join("idVendor")),
            product_id: read_trimmed(&path.join("idProduct")),
            serial: read_trimmed(&path.join("serial")),
            bus_number: read_trimmed(&path.join("busnum")),
            device_number: read_trimmed(&path.join("devnum")),
        });
    }

    devices.sort_by(|a, b| a.product.cmp(&b.product));
    Ok(devices)
}

fn read_trimmed(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}
