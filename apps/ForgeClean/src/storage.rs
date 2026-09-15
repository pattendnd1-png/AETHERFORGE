use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockDeviceRow {
    pub name: String,
    pub path: PathBuf,
    pub parent_name: Option<String>,
    pub kind: String,
    pub removable: bool,
    pub hotplug: bool,
    pub transport: Option<String>,
    pub mount_point: Option<PathBuf>,
    pub uuid: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalDrive {
    pub mount_point: PathBuf,
    pub device_path: PathBuf,
    pub uuid: Option<String>,
    pub transport: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AutoMode {
    CleanOnly,
    Offload(ExternalDrive),
}

pub fn parse_lsblk_pairs(text: &str) -> Result<Vec<BlockDeviceRow>, String> {
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let fields = parse_pairs_line(line)?;
        let name = required(&fields, "NAME")?.to_owned();
        let path = PathBuf::from(required(&fields, "PATH")?);
        let parent_name = optional(&fields, "PKNAME").map(ToOwned::to_owned);
        let kind = required(&fields, "TYPE")?.to_owned();
        let removable = required(&fields, "RM")? == "1";
        let hotplug = required(&fields, "HOTPLUG")? == "1";
        let transport = optional(&fields, "TRAN").map(ToOwned::to_owned);
        let mount_point = optional(&fields, "MOUNTPOINT").map(PathBuf::from);
        let uuid = optional(&fields, "UUID").map(ToOwned::to_owned);
        rows.push(BlockDeviceRow {
            name,
            path,
            parent_name,
            kind,
            removable,
            hotplug,
            transport,
            mount_point,
            uuid,
        });
    }
    Ok(rows)
}

pub fn select_auto_mode(rows: &[BlockDeviceRow], source_device: Option<u64>) -> AutoMode {
    eligible_external_drives(rows, source_device)
        .into_iter()
        .next()
        .map(AutoMode::Offload)
        .unwrap_or(AutoMode::CleanOnly)
}

pub fn detect_external_drives(source_root: &Path) -> Result<Vec<ExternalDrive>, String> {
    let source_device = fs::metadata(source_root)
        .ok()
        .map(|metadata| metadata.dev());
    let output = Command::new("/usr/bin/lsblk")
        .args([
            "--pairs",
            "--output",
            "NAME,PATH,PKNAME,TYPE,RM,HOTPLUG,TRAN,MOUNTPOINT,UUID",
        ])
        .output()
        .map_err(|e| format!("cannot execute /usr/bin/lsblk: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "lsblk failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|e| format!("lsblk output is not valid UTF-8: {e}"))?;
    let rows = parse_lsblk_pairs(&text)?;
    Ok(eligible_external_drives(&rows, source_device))
}

fn eligible_external_drives(
    rows: &[BlockDeviceRow],
    source_device: Option<u64>,
) -> Vec<ExternalDrive> {
    let by_name: BTreeMap<&str, &BlockDeviceRow> =
        rows.iter().map(|row| (row.name.as_str(), row)).collect();
    let mut seen_mounts = BTreeSet::new();
    let mut drives = Vec::new();

    for row in rows {
        let Some(mount_point) = row.mount_point.as_ref() else {
            continue;
        };
        if mount_point == Path::new("/")
            || mount_point == Path::new("/boot")
            || mount_point == Path::new("/boot/efi")
        {
            continue;
        }
        if !has_external_ancestor(row, &by_name) {
            continue;
        }
        if let Some(source_device) = source_device {
            let Ok(metadata) = fs::metadata(mount_point) else {
                continue;
            };
            if metadata.dev() == source_device {
                continue;
            }
        }
        if !seen_mounts.insert(mount_point.clone()) {
            continue;
        }
        let inherited_transport = inherited_transport(row, &by_name);
        drives.push(ExternalDrive {
            mount_point: mount_point.clone(),
            device_path: row.path.clone(),
            uuid: row.uuid.clone(),
            transport: inherited_transport,
        });
    }
    drives.sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    drives
}

fn has_external_ancestor<'a>(
    start: &'a BlockDeviceRow,
    by_name: &BTreeMap<&'a str, &'a BlockDeviceRow>,
) -> bool {
    let mut current = start;
    for _ in 0..16 {
        if is_external(current) {
            return true;
        }
        let Some(parent) = current.parent_name.as_deref() else {
            return false;
        };
        let Some(next) = by_name.get(parent).copied() else {
            return false;
        };
        current = next;
    }
    false
}

fn inherited_transport<'a>(
    start: &'a BlockDeviceRow,
    by_name: &BTreeMap<&'a str, &'a BlockDeviceRow>,
) -> Option<String> {
    let mut current = start;
    for _ in 0..16 {
        if let Some(transport) = current.transport.as_ref().filter(|value| !value.is_empty()) {
            return Some(transport.clone());
        }
        let parent = current.parent_name.as_deref()?;
        let next = by_name.get(parent).copied()?;
        current = next;
    }
    None
}

fn is_external(row: &BlockDeviceRow) -> bool {
    if row.removable || row.hotplug {
        return true;
    }
    let Some(transport) = row.transport.as_deref() else {
        return false;
    };
    matches!(
        transport.to_ascii_lowercase().as_str(),
        "usb" | "firewire" | "ieee1394" | "thunderbolt"
    )
}

fn parse_pairs_line(line: &str) -> Result<BTreeMap<String, String>, String> {
    let mut fields = BTreeMap::new();
    for token in line.split_whitespace() {
        let (key, raw_value) = token
            .split_once('=')
            .ok_or_else(|| format!("malformed lsblk pair: {token}"))?;
        if raw_value.len() < 2 || !raw_value.starts_with('"') || !raw_value.ends_with('"') {
            return Err(format!("malformed quoted lsblk value: {token}"));
        }
        let inner = &raw_value[1..raw_value.len() - 1];
        fields.insert(key.to_owned(), decode_lsblk_value(inner)?);
    }
    Ok(fields)
}

fn decode_lsblk_value(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 3 < bytes.len() && bytes[i + 1] == b'x' {
            let high = hex_nibble(bytes[i + 2])?;
            let low = hex_nibble(bytes[i + 3])?;
            out.push((high << 4) | low);
            i += 4;
        } else if bytes[i] == b'\\' && i + 1 < bytes.len() {
            out.push(bytes[i + 1]);
            i += 2;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|e| format!("lsblk field is not UTF-8: {e}"))
}

fn required<'a>(fields: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    fields
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("lsblk row missing {key}"))
}

fn optional<'a>(fields: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    fields
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid lsblk hex escape: {}", byte as char)),
    }
}
