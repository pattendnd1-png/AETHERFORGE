use crate::DeviceError;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatteryDescriptor {
    pub path: String,
    pub name: String,
    pub manufacturer: Option<String>,
    pub model_name: Option<String>,
    pub serial: Option<String>,
    pub capacity: Option<u8>,
    pub status: Option<String>,
}

pub trait PowerBackend: Send + Sync {
    fn enumerate(&self) -> Result<Vec<BatteryDescriptor>, DeviceError>;
}

#[derive(Debug, Default, Clone)]
pub struct SystemPowerBackend {
    pub root: Option<PathBuf>,
}

impl PowerBackend for SystemPowerBackend {
    fn enumerate(&self) -> Result<Vec<BatteryDescriptor>, DeviceError> {
        enumerate_power_root(
            self.root
                .as_deref()
                .unwrap_or(Path::new("/sys/class/power_supply")),
        )
    }
}

pub fn enumerate_power_root(root: &Path) -> Result<Vec<BatteryDescriptor>, DeviceError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let entries =
        fs::read_dir(root).map_err(|e| DeviceError::Power(format!("{}: {e}", root.display())))?;
    let mut batteries = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let supply_type = read_trimmed(path.join("type"));
        if supply_type.as_deref() != Some("Battery") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("battery")
            .to_owned();
        let capacity = read_trimmed(path.join("capacity"))
            .and_then(|value| value.parse::<u8>().ok())
            .map(|value| value.min(100));
        batteries.push(BatteryDescriptor {
            path: path.to_string_lossy().into_owned(),
            name,
            manufacturer: read_trimmed(path.join("manufacturer")),
            model_name: read_trimmed(path.join("model_name")),
            serial: read_trimmed(path.join("serial_number")),
            capacity,
            status: read_trimmed(path.join("status")),
        });
    }
    batteries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(batteries)
}

fn read_trimmed(path: PathBuf) -> Option<String> {
    let value = fs::read_to_string(path).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
