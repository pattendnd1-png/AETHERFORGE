use crate::{unix_ms, SharedState};
use reforge_core::{
    DeviceSummary, FirmwareDevice, FirmwareInstallResult, FirmwareRelease, ServiceKind,
    ServiceState,
};
use serde_json::{Map, Value};
use std::{
    collections::HashSet,
    io::Read,
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

const FWUPD_TIMEOUT: Duration = Duration::from_secs(20);
const FWUPD_UPDATE_TIMEOUT: Duration = Duration::from_secs(10 * 60);

struct CommandResult {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn sanitize_output(bytes: &[u8]) -> String {
    let mut value = String::from_utf8_lossy(bytes).replace(['\r', '\n', '\t'], " ");
    while value.contains("  ") {
        value = value.replace("  ", " ");
    }
    value.truncate(500);
    value
}

fn run_fwupdmgr(args: &[&str], timeout: Duration) -> Result<CommandResult, String> {
    let mut child = Command::new("fwupdmgr")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start fwupdmgr: {error}"))?;

    let mut stdout = child.stdout.take().ok_or_else(|| "fwupdmgr stdout unavailable".to_owned())?;
    let mut stderr = child.stderr.take().ok_or_else(|| "fwupdmgr stderr unavailable".to_owned())?;
    let stdout_reader = thread::spawn(move || {
        let mut data = Vec::new();
        let _ = stdout.read_to_end(&mut data);
        data
    });
    let stderr_reader = thread::spawn(move || {
        let mut data = Vec::new();
        let _ = stderr.read_to_end(&mut data);
        data
    });

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll fwupdmgr: {error}"))?
        {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_reader.join();
            let _ = stderr_reader.join();
            return Err(format!("fwupdmgr timed out after {} seconds", timeout.as_secs()));
        }
        thread::sleep(Duration::from_millis(50));
    };

    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    Ok(CommandResult { status, stdout, stderr })
}

fn parse_json_output(result: CommandResult, action: &str, allow_nothing_to_do: bool) -> Result<Value, String> {
    let code = result.status.code().unwrap_or(1);
    if !result.status.success() && !(allow_nothing_to_do && code == 2) {
        let message = sanitize_output(&result.stderr);
        return Err(if message.is_empty() {
            format!("fwupdmgr {action} failed with exit code {code}")
        } else {
            format!("fwupdmgr {action} failed: {message}")
        });
    }
    if result.stdout.is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    serde_json::from_slice(&result.stdout)
        .map_err(|error| format!("fwupdmgr {action} returned invalid JSON: {error}"))
}

fn get_string(map: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        map.get(*key).and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
    })
}

fn get_strings(map: &Map<String, Value>, keys: &[&str]) -> Vec<String> {
    for key in keys {
        if let Some(value) = map.get(*key) {
            return match value {
                Value::Array(values) => values
                    .iter()
                    .filter_map(|value| match value {
                        Value::String(value) => Some(value.clone()),
                        Value::Number(value) => Some(value.to_string()),
                        _ => None,
                    })
                    .collect(),
                Value::String(value) => vec![value.clone()],
                Value::Number(value) => vec![value.to_string()],
                _ => vec![],
            };
        }
    }
    vec![]
}

fn collect_objects<'a>(value: &'a Value, required_key: &str, out: &mut Vec<&'a Map<String, Value>>) {
    match value {
        Value::Object(map) => {
            if map.contains_key(required_key) {
                out.push(map);
            }
            for value in map.values() {
                collect_objects(value, required_key, out);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_objects(value, required_key, out);
            }
        }
        _ => {}
    }
}

fn parse_usb_ids(values: &[String]) -> (Option<u16>, Option<u16>) {
    for value in values {
        let upper = value.to_ascii_uppercase();
        let Some(vid_pos) = upper.find("VID_") else { continue };
        let Some(pid_pos) = upper.find("PID_") else { continue };
        let vid = upper.get(vid_pos + 4..vid_pos + 8).and_then(|value| u16::from_str_radix(value, 16).ok());
        let pid = upper.get(pid_pos + 4..pid_pos + 8).and_then(|value| u16::from_str_radix(value, 16).ok());
        if vid.is_some() || pid.is_some() {
            return (vid, pid);
        }
    }
    (None, None)
}

fn is_logitech(map: &Map<String, Value>, instance_ids: &[String]) -> bool {
    let vendor = get_string(map, &["Vendor", "VendorName"]).unwrap_or_default().to_ascii_lowercase();
    let name = get_string(map, &["Name", "DeviceName"]).unwrap_or_default().to_ascii_lowercase();
    let plugin = get_string(map, &["Plugin"]).unwrap_or_default().to_ascii_lowercase();
    vendor.contains("logitech")
        || vendor == "logi"
        || name.contains("logitech")
        || name.starts_with("logi ")
        || plugin.contains("logitech")
        || instance_ids.iter().any(|value| value.to_ascii_uppercase().contains("VID_046D"))
}

fn flags_lower(map: &Map<String, Value>) -> Vec<String> {
    get_strings(map, &["Flags", "DeviceFlags", "ReleaseFlags"])
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn normalize_name(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace("logitech", "")
        .replace("logi", "")
        .replace(['-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn match_logitech_device(firmware: &FirmwareDevice, devices: &[DeviceSummary]) -> Option<String> {
    devices
        .iter()
        .filter(|device| device.vendor_id == 0x046d)
        .find(|device| {
            if let Some(pid) = firmware.product_id
                && pid != 0
                && device.product_id != 0
            {
                return pid == device.product_id;
            }
            let firmware_name = normalize_name(&firmware.name);
            let device_name = normalize_name(&device.product);
            !firmware_name.is_empty()
                && !device_name.is_empty()
                && (firmware_name.contains(&device_name) || device_name.contains(&firmware_name))
        })
        .map(|device| device.key.clone())
}

pub fn parse_devices_json(value: &Value, update_ids: &HashSet<String>) -> Vec<FirmwareDevice> {
    let mut objects = Vec::new();
    collect_objects(value, "DeviceId", &mut objects);
    let mut devices = Vec::new();
    let mut seen = HashSet::new();
    for map in objects {
        let Some(id) = get_string(map, &["DeviceId", "DeviceID"]) else { continue };
        if !seen.insert(id.clone()) {
            continue;
        }
        let instance_ids = get_strings(map, &["InstanceIds", "InstanceIDs", "InstanceId", "InstanceID"]);
        if !is_logitech(map, &instance_ids) {
            continue;
        }
        let (vendor_id, product_id) = parse_usb_ids(&instance_ids);
        devices.push(FirmwareDevice {
            id: id.clone(),
            name: get_string(map, &["Name", "DeviceName"]).unwrap_or_else(|| "Logitech device".into()),
            vendor: get_string(map, &["Vendor", "VendorName"]),
            version: get_string(map, &["Version", "CurrentVersion"]),
            plugin: get_string(map, &["Plugin"]),
            guids: get_strings(map, &["Guid", "Guids", "GUID", "GUIDs"]),
            instance_ids,
            vendor_id,
            product_id,
            matched_device_key: None,
            flags: get_strings(map, &["Flags", "DeviceFlags"]),
            update_available: update_ids.contains(&id),
        });
    }
    devices.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    devices
}

pub fn parse_update_ids(value: &Value) -> HashSet<String> {
    let mut objects = Vec::new();
    collect_objects(value, "DeviceId", &mut objects);
    objects
        .into_iter()
        .filter_map(|map| get_string(map, &["DeviceId", "DeviceID"]))
        .collect()
}

pub fn parse_releases_json(value: &Value) -> Vec<FirmwareRelease> {
    let mut objects = Vec::new();
    collect_objects(value, "Version", &mut objects);
    let mut releases = Vec::new();
    let mut seen = HashSet::new();
    for map in objects {
        if map.contains_key("DeviceId") && !map.contains_key("RemoteId") && !map.contains_key("Locations") {
            continue;
        }
        let Some(version) = get_string(map, &["Version"]) else { continue };
        if !seen.insert(version.clone()) {
            continue;
        }
        let flags = flags_lower(map);
        let locations = get_strings(map, &["Locations", "Uri", "URI"]);
        releases.push(FirmwareRelease {
            version,
            name: get_string(map, &["Name"]),
            summary: get_string(map, &["Summary"]),
            description: get_string(map, &["Description"]),
            remote_id: get_string(map, &["RemoteId", "RemoteID"]),
            uri: locations.first().cloned(),
            checksums: get_strings(map, &["Checksums", "Checksum"]),
            trusted: flags.iter().any(|flag| flag.contains("trusted")).then_some(true),
            needs_reboot: flags.iter().any(|flag| flag.contains("reboot")),
            needs_replug: flags.iter().any(|flag| flag.contains("replug")),
        });
    }
    releases
}

fn local_devices(shared: &SharedState) -> Vec<DeviceSummary> {
    shared
        .runtime
        .lock()
        .map(|runtime| runtime.registry.runtime_list().into_iter().map(|item| item.summary).collect())
        .unwrap_or_default()
}

fn set_service_status(
    shared: &SharedState,
    state: ServiceState,
    version: Option<String>,
    message: impl Into<String>,
) {
    if let Ok(mut services) = shared.services.lock() {
        services.set(
            ServiceKind::Fwupd,
            state,
            "fwupdmgr/LVFS",
            version,
            Some(message.into()),
            (state == ServiceState::Ready).then_some(unix_ms()),
        );
    }
}

fn fwupd_version() -> Result<String, String> {
    let result = run_fwupdmgr(&["--version"], FWUPD_TIMEOUT)?;
    if !result.status.success() {
        return Err(format!("fwupdmgr --version failed: {}", sanitize_output(&result.stderr)));
    }
    let text = String::from_utf8_lossy(&result.stdout);
    let version = text
        .lines()
        .find_map(|line| {
            let lower = line.to_ascii_lowercase();
            if lower.contains("org.freedesktop.fwupd") || lower.starts_with("runtime") {
                line.split_whitespace().last().map(ToOwned::to_owned)
            } else {
                None
            }
        })
        .or_else(|| text.lines().find(|line| !line.trim().is_empty()).map(|line| line.trim().to_owned()))
        .unwrap_or_else(|| "unknown".into());
    Ok(version)
}

pub fn probe_and_update_status(shared: &SharedState) -> Result<(), String> {
    match fwupd_version() {
        Ok(version) => {
            set_service_status(shared, ServiceState::Ready, Some(version), "fwupd system service available");
            Ok(())
        }
        Err(error) => {
            let unavailable = error.contains("No such file") || error.contains("not found") || error.contains("failed to start");
            set_service_status(
                shared,
                if unavailable { ServiceState::Unavailable } else { ServiceState::Degraded },
                None,
                &error,
            );
            Err(error)
        }
    }
}

pub fn list_devices(shared: &SharedState) -> Result<Vec<FirmwareDevice>, String> {
    probe_and_update_status(shared)?;
    let devices_json = parse_json_output(
        run_fwupdmgr(&["get-devices", "--json"], FWUPD_TIMEOUT)?,
        "get-devices",
        false,
    )?;
    let update_json = parse_json_output(
        run_fwupdmgr(&["get-updates", "--json"], FWUPD_TIMEOUT)?,
        "get-updates",
        true,
    )?;
    let update_ids = parse_update_ids(&update_json);
    let locals = local_devices(shared);
    let mut devices = parse_devices_json(&devices_json, &update_ids);
    for device in &mut devices {
        device.matched_device_key = match_logitech_device(device, &locals);
    }
    set_service_status(
        shared,
        ServiceState::Ready,
        fwupd_version().ok(),
        format!("{} Logitech firmware device(s) visible through fwupd", devices.len()),
    );
    Ok(devices)
}

pub fn refresh_metadata(shared: &SharedState) -> Result<(), String> {
    probe_and_update_status(shared)?;
    let result = run_fwupdmgr(&["refresh"], Duration::from_secs(120))?;
    let code = result.status.code().unwrap_or(1);
    if !result.status.success() && code != 2 {
        let error = format!("fwupdmgr refresh failed: {}", sanitize_output(&result.stderr));
        set_service_status(shared, ServiceState::Degraded, fwupd_version().ok(), &error);
        return Err(error);
    }
    set_service_status(shared, ServiceState::Ready, fwupd_version().ok(), "LVFS metadata refresh complete");
    Ok(())
}

pub fn releases(shared: &SharedState, device_id: &str) -> Result<Vec<FirmwareRelease>, String> {
    if device_id.trim().is_empty() {
        return Err("firmware device ID is required".into());
    }
    probe_and_update_status(shared)?;
    let value = parse_json_output(
        run_fwupdmgr(&["get-releases", device_id, "--json"], FWUPD_TIMEOUT)?,
        "get-releases",
        true,
    )?;
    Ok(parse_releases_json(&value))
}

pub fn install_update(shared: &SharedState, device_id: &str) -> Result<FirmwareInstallResult, String> {
    if device_id.trim().is_empty() {
        return Err("firmware device ID is required".into());
    }
    let devices = list_devices(shared)?;
    let device = devices
        .iter()
        .find(|device| device.id == device_id)
        .ok_or_else(|| format!("fwupd device is not an enumerated Logitech device: {device_id}"))?;
    if !device.update_available {
        return Err(format!("no firmware update is currently advertised for {}", device.name));
    }
    let result = run_fwupdmgr(&["update", device_id, "-y"], FWUPD_UPDATE_TIMEOUT)?;
    let stdout = sanitize_output(&result.stdout);
    let stderr = sanitize_output(&result.stderr);
    let combined = format!("{stdout} {stderr}").to_ascii_lowercase();
    let needs_reboot = combined.contains("reboot") || combined.contains("restart");
    let needs_replug = combined.contains("replug") || combined.contains("reconnect") || combined.contains("unplug");
    if !result.status.success() {
        let message = if stderr.is_empty() { stdout } else { stderr };
        set_service_status(shared, ServiceState::Degraded, fwupd_version().ok(), &message);
        return Err(format!("fwupdmgr update failed: {message}"));
    }
    let message = if stdout.is_empty() { "firmware update accepted by fwupd".into() } else { stdout };
    set_service_status(shared, ServiceState::Ready, fwupd_version().ok(), "firmware operation completed through fwupd");
    Ok(FirmwareInstallResult {
        device_id: device_id.to_owned(),
        success: true,
        message,
        needs_reboot,
        needs_replug,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_logitech_devices_and_update_state() {
        let devices: Value = serde_json::from_str(include_str!("../tests/fixtures/fwupd-devices.json")).unwrap();
        let updates: Value = serde_json::from_str(include_str!("../tests/fixtures/fwupd-updates.json")).unwrap();
        let parsed = parse_devices_json(&devices, &parse_update_ids(&updates));
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].vendor_id, Some(0x046d));
        assert_eq!(parsed[0].product_id, Some(0xc548));
        assert!(parsed[0].update_available);
    }

    #[test]
    fn parses_release_metadata_and_replug_requirement() {
        let value: Value = serde_json::from_str(include_str!("../tests/fixtures/fwupd-releases.json")).unwrap();
        let releases = parse_releases_json(&value);
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].version, "5.3.0");
        assert_eq!(releases[0].trusted, Some(true));
        assert!(releases[0].needs_replug);
    }

    #[test]
    fn matches_local_logitech_device_by_pid() {
        let firmware = FirmwareDevice {
            name: "Receiver".into(),
            vendor_id: Some(0x046d),
            product_id: Some(0xc548),
            ..Default::default()
        };
        let local = DeviceSummary {
            key: "local-key".into(), path: "/dev/hidraw0".into(), device_index: 0xff,
            vendor_id: 0x046d, product_id: 0xc548, product: "LIGHTSPEED Receiver".into(),
            serial: None, hardware_id: None, interface_number: 0, usage_page: 0, usage: 0,
            transport: reforge_core::TransportKind::Usb, hidpp: true, device_class: reforge_core::DeviceClass::Receiver,
            providers: vec![], controls: vec![], features: vec![], dpi: None, lighting: None, error: None,
        };
        assert_eq!(match_logitech_device(&firmware, &[local]), Some("local-key".into()));
    }
}
