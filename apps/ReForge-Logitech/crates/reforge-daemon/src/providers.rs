use reforge_core::{
    ControlChoice, ControlGroup, ControlKind, ControlValue, DeviceClass, DeviceControl, DevicePresence,
    DeviceSummary, DeviceTelemetry, ProviderKind, ReadbackKind, TransportKind,
};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const LOGITECH_VENDOR_ID: u16 = 0x046d;

fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_hex_u16(value: &str) -> Option<u16> {
    u16::from_str_radix(value.trim().trim_start_matches("0x"), 16).ok()
}

fn logitech_identity(vendor: Option<u16>, manufacturer: Option<&str>) -> bool {
    vendor == Some(LOGITECH_VENDOR_ID)
        || manufacturer
            .map(|value| {
                let value = value.to_ascii_lowercase();
                value.contains("logitech") || value == "logi" || value.starts_with("logi ")
            })
            .unwrap_or(false)
}

fn classify_product(product: &str) -> DeviceClass {
    let lower = product.to_ascii_lowercase();
    if lower.contains("camera")
        || lower.contains("webcam")
        || lower.contains("brio")
        || lower.contains("streamcam")
    {
        DeviceClass::Camera
    } else if lower.contains("headset")
        || lower.contains("speaker")
        || lower.contains("microphone")
        || lower.contains("audio")
        || lower.contains("blue yeti")
        || lower.contains("yeti")
    {
        DeviceClass::Audio
    } else if lower.contains("keyboard")
        || lower.contains("g515")
        || lower.contains("g915")
        || lower.contains("g815")
        || lower.contains("keys")
    {
        DeviceClass::Keyboard
    } else if lower.contains("mouse")
        || lower.contains("g502")
        || lower.contains("g703")
        || lower.contains("g903")
        || lower.contains("master")
    {
        DeviceClass::Mouse
    } else if lower.contains("receiver")
        || lower.contains("unifying")
        || lower.contains("bolt")
        || lower.contains("lightspeed")
    {
        DeviceClass::Receiver
    } else {
        DeviceClass::Other
    }
}

fn usb_summary(path: &Path) -> Option<DeviceSummary> {
    let vendor = read_trim(path.join("idVendor")).and_then(|value| parse_hex_u16(&value));
    let manufacturer = read_trim(path.join("manufacturer"));
    if !logitech_identity(vendor, manufacturer.as_deref()) {
        return None;
    }
    let product_id = read_trim(path.join("idProduct"))
        .and_then(|value| parse_hex_u16(&value))
        .unwrap_or(0);
    let product = read_trim(path.join("product"))
        .or_else(|| manufacturer.clone())
        .unwrap_or_else(|| format!("Logitech USB Device {:04X}", product_id));
    let serial = read_trim(path.join("serial"));
    let sysname = path.file_name()?.to_string_lossy().into_owned();
    Some(DeviceSummary {
        key: format!("usb:{sysname}"),
        path: path.to_string_lossy().into_owned(),
        device_index: 0xff,
        vendor_id: vendor.unwrap_or(LOGITECH_VENDOR_ID),
        product_id,
        product: product.clone(),
        serial,
        hardware_id: None,
        interface_number: -1,
        usage_page: 0,
        usage: 0,
        transport: TransportKind::Usb,
        hidpp: false,
        device_class: classify_product(&product),
        providers: vec![ProviderKind::Usb],
        controls: vec![],
        features: vec![],
        dpi: None,
        lighting: None,
        error: None,
    })
}

pub fn scan_usb_inventory() -> Vec<DeviceSummary> {
    let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") else {
        return vec![];
    };
    entries
        .flatten()
        .filter_map(|entry| usb_summary(&entry.path()))
        .collect()
}

fn usb_ancestor(start: &Path) -> Option<PathBuf> {
    let mut current = fs::canonicalize(start).ok()?;
    loop {
        if current.join("idVendor").is_file() && current.join("idProduct").is_file() {
            let vendor = read_trim(current.join("idVendor")).and_then(|value| parse_hex_u16(&value));
            let manufacturer = read_trim(current.join("manufacturer"));
            if logitech_identity(vendor, manufacturer.as_deref()) {
                return Some(current);
            }
            return None;
        }
        if !current.pop() {
            return None;
        }
    }
}

fn title_label(id: &str) -> String {
    id.split('_')
        .filter(|piece| !piece.is_empty())
        .map(|piece| {
            let mut chars = piece.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn attr_i64(attrs: &str, key: &str) -> Option<i64> {
    attrs.split_whitespace().find_map(|field| {
        let (name, value) = field.split_once('=')?;
        (name == key)
            .then(|| value.trim_end_matches(',').parse().ok())
            .flatten()
    })
}

fn v4l2_flags(attrs: &str) -> String {
    attrs
        .split_once("flags=")
        .map(|(_, flags)| flags.to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn parse_v4l2_controls(endpoint: &str, text: &str) -> Vec<DeviceControl> {
    let mut controls = Vec::<DeviceControl>::new();
    let mut last_menu: Option<usize> = None;
    for raw in text.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if line.chars().next().is_some_and(char::is_whitespace) {
            if let Some(index) = last_menu
                && let Some((value, label)) = trimmed.split_once(':')
                && let Ok(value) = value.trim().parse::<i64>()
            {
                controls[index].choices.push(ControlChoice {
                    value,
                    label: label.trim().to_owned(),
                });
            }
            continue;
        }
        last_menu = None;
        let Some((lhs, attrs)) = trimmed.split_once(" : ") else {
            continue;
        };
        let Some(type_start) = lhs.rfind('(') else {
            continue;
        };
        let Some(type_end) = lhs.rfind(')') else {
            continue;
        };
        let kind_name = &lhs[type_start + 1..type_end];
        let id = lhs[..type_start].split_whitespace().next().unwrap_or("");
        if id.is_empty() {
            continue;
        }
        let current = attr_i64(attrs, "value").unwrap_or(0);
        let flags = v4l2_flags(attrs);
        let disabled = flags.contains("disabled") || flags.contains("inactive");
        let read_only = flags.contains("read-only") || flags.contains("readonly");
        let (kind, value) = match kind_name {
            "bool" => (ControlKind::Toggle, ControlValue::Bool(current != 0)),
            "menu" | "intmenu" => (ControlKind::Choice, ControlValue::Int(current)),
            "button" => (ControlKind::Action, ControlValue::None),
            "int" | "int64" | "integer" | "integer64" => {
                (ControlKind::Range, ControlValue::Int(current))
            }
            _ => continue,
        };
        let writable = !disabled && !read_only;
        let readback = if writable {
            ReadbackKind::Readable
        } else {
            ReadbackKind::Telemetry
        };
        let index = controls.len();
        controls.push(DeviceControl {
            id: format!("v4l2:{id}"),
            label: title_label(id),
            provider: ProviderKind::V4l2,
            endpoint: endpoint.to_owned(),
            backend_id: id.to_owned(),
            kind,
            value,
            min: attr_i64(attrs, "min"),
            max: attr_i64(attrs, "max"),
            step: attr_i64(attrs, "step"),
            choices: vec![],
            writable,
            profile_eligible: writable && kind != ControlKind::Action,
            group: ControlGroup::Camera,
            readback,
        });
        if matches!(controls[index].kind, ControlKind::Choice) {
            last_menu = Some(index);
        }
    }
    controls
}

fn v4l2_controls(endpoint: &str) -> Vec<DeviceControl> {
    let output = Command::new("v4l2-ctl")
        .args(["--list-ctrls-menus", "-d", endpoint])
        .output();
    let Ok(output) = output else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    parse_v4l2_controls(endpoint, &String::from_utf8_lossy(&output.stdout))
}

pub fn scan_v4l2() -> Vec<DeviceSummary> {
    let Ok(entries) = fs::read_dir("/sys/class/video4linux") else {
        return vec![];
    };
    let mut by_usb = BTreeMap::<String, DeviceSummary>::new();
    for entry in entries.flatten() {
        let node = entry.file_name().to_string_lossy().into_owned();
        let endpoint = format!("/dev/{node}");
        let Some(usb) = usb_ancestor(&entry.path().join("device")) else {
            continue;
        };
        let Some(mut summary) = usb_summary(&usb) else {
            continue;
        };
        summary.device_class = DeviceClass::Camera;
        summary.providers = vec![ProviderKind::Usb, ProviderKind::V4l2];
        summary.controls = v4l2_controls(&endpoint);
        if summary.product.to_ascii_lowercase().contains("usb device") {
            summary.product = read_trim(entry.path().join("name")).unwrap_or(summary.product);
        }
        by_usb
            .entry(summary.key.clone())
            .and_modify(|existing| {
                if !existing.providers.contains(&ProviderKind::V4l2) {
                    existing.providers.push(ProviderKind::V4l2);
                }
                for control in &summary.controls {
                    if !existing
                        .controls
                        .iter()
                        .any(|item| item.id == control.id && item.endpoint == control.endpoint)
                    {
                        existing.controls.push(control.clone());
                    }
                }
            })
            .or_insert(summary);
    }
    by_usb.into_values().collect()
}

fn property(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        if key.trim().trim_start_matches('*').trim() != name {
            return None;
        }
        Some(value.trim().trim_matches('"').to_owned())
    })
}

fn wpctl_ids(status: &str) -> Vec<(u32, String)> {
    let mut result = vec![];
    for line in status.lines() {
        let lower = line.to_ascii_lowercase();
        if !lower.contains("logitech") && !lower.contains("logi ") {
            continue;
        }
        let trimmed = line.trim_start_matches(|c: char| !c.is_ascii_digit());
        let Some((id, rest)) = trimmed.split_once('.') else {
            continue;
        };
        let Ok(id) = id.trim().parse::<u32>() else {
            continue;
        };
        let name = rest.trim().trim_end_matches('*').trim().to_owned();
        result.push((id, name));
    }
    result
}

fn pipewire_controls(id: u32, endpoint_name: &str) -> Vec<DeviceControl> {
    let endpoint = id.to_string();
    let volume_text = Command::new("wpctl")
        .args(["get-volume", &endpoint])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
        .unwrap_or_default();
    let volume = volume_text
        .split_whitespace()
        .find_map(|token| token.parse::<f64>().ok())
        .unwrap_or(1.0);
    let muted = volume_text.contains("MUTED");
    vec![
        DeviceControl {
            id: format!("pipewire:{endpoint_name}:volume"),
            label: format!("Volume — {endpoint_name}"),
            provider: ProviderKind::Pipewire,
            endpoint: endpoint.clone(),
            backend_id: "volume".into(),
            kind: ControlKind::Range,
            value: ControlValue::Int((volume * 100.0).round() as i64),
            min: Some(0),
            max: Some(150),
            step: Some(1),
            choices: vec![],
            writable: true,
            profile_eligible: true,
            group: ControlGroup::Audio,
            readback: ReadbackKind::Readable,
        },
        DeviceControl {
            id: format!("pipewire:{endpoint_name}:mute"),
            label: format!("Mute — {endpoint_name}"),
            provider: ProviderKind::Pipewire,
            endpoint,
            backend_id: "mute".into(),
            kind: ControlKind::Toggle,
            value: ControlValue::Bool(muted),
            min: Some(0),
            max: Some(1),
            step: Some(1),
            choices: vec![],
            writable: true,
            profile_eligible: true,
            group: ControlGroup::Audio,
            readback: ReadbackKind::Readable,
        },
    ]
}

pub fn scan_pipewire() -> Vec<DeviceSummary> {
    let Ok(status) = Command::new("wpctl").args(["status", "-n"]).output() else {
        return vec![];
    };
    if !status.status.success() {
        return vec![];
    }
    let status = String::from_utf8_lossy(&status.stdout);
    let mut devices = vec![];
    for (id, fallback_name) in wpctl_ids(&status) {
        let inspect = Command::new("wpctl")
            .args(["inspect", &id.to_string()])
            .output()
            .ok();
        let inspect_text = inspect
            .as_ref()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default();
        let lower = inspect_text.to_ascii_lowercase();
        if !lower.contains("logitech")
            && !lower.contains("0x046d")
            && !lower.contains("1133")
            && !fallback_name.to_ascii_lowercase().contains("logi")
        {
            continue;
        }
        let product = property(&inspect_text, "device.product.name")
            .or_else(|| property(&inspect_text, "node.description"))
            .unwrap_or(fallback_name.clone());
        let product_id = property(&inspect_text, "device.product.id")
            .and_then(|value| {
                let value = value.trim();
                if value.starts_with("0x") {
                    parse_hex_u16(value)
                } else {
                    value.parse::<u16>().ok().or_else(|| parse_hex_u16(value))
                }
            })
            .unwrap_or(0);
        let serial = property(&inspect_text, "device.serial");
        devices.push(DeviceSummary {
            key: format!("pipewire:{id}"),
            path: format!("pipewire:{id}"),
            device_index: 0xff,
            vendor_id: LOGITECH_VENDOR_ID,
            product_id,
            product: product.clone(),
            serial,
            hardware_id: None,
            interface_number: -1,
            usage_page: 0,
            usage: 0,
            transport: TransportKind::Usb,
            hidpp: false,
            device_class: DeviceClass::Audio,
            providers: vec![ProviderKind::Pipewire],
            controls: pipewire_controls(id, &fallback_name),
            features: vec![],
            dpi: None,
            lighting: None,
            error: None,
        });
    }
    devices
}

fn normalized_product(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    if let Some((base, _)) = lower.split_once(" [receiver slot ") {
        base.trim().to_owned()
    } else {
        lower.trim().to_owned()
    }
}

fn same_physical(a: &DeviceSummary, b: &DeviceSummary) -> bool {
    if a.key == b.key {
        return true;
    }
    if !a.path.is_empty() && a.path == b.path {
        return true;
    }
    if a.product_id != 0 && a.product_id == b.product_id {
        if let (Some(a_serial), Some(b_serial)) = (a.serial.as_deref(), b.serial.as_deref()) {
            return !a_serial.is_empty() && a_serial == b_serial;
        }
    }
    false
}

fn merge_into(existing: &mut DeviceSummary, incoming: DeviceSummary) {
    if incoming.hidpp && !existing.hidpp {
        let providers = existing.providers.clone();
        let controls = existing.controls.clone();
        *existing = incoming;
        for provider in providers {
            if !existing.providers.contains(&provider) {
                existing.providers.push(provider);
            }
        }
        for control in controls {
            if !existing
                .controls
                .iter()
                .any(|item| item.id == control.id && item.endpoint == control.endpoint)
            {
                existing.controls.push(control);
            }
        }
        return;
    }
    if matches!(incoming.device_class, DeviceClass::Camera | DeviceClass::Audio) {
        existing.device_class = incoming.device_class;
    }
    for provider in incoming.providers {
        if !existing.providers.contains(&provider) {
            existing.providers.push(provider);
        }
    }
    for control in incoming.controls {
        if !existing
            .controls
            .iter()
            .any(|item| item.id == control.id && item.endpoint == control.endpoint)
        {
            existing.controls.push(control);
        }
    }
    if existing.serial.is_none() {
        existing.serial = incoming.serial;
    }
    if existing.product_id == 0 {
        existing.product_id = incoming.product_id;
    }
}

pub fn merge_devices(groups: impl IntoIterator<Item = Vec<DeviceSummary>>) -> Vec<DeviceSummary> {
    let mut result = Vec::<DeviceSummary>::new();
    for group in groups {
        for device in group {
            if let Some(existing) = result
                .iter_mut()
                .find(|existing| same_physical(existing, &device))
            {
                merge_into(existing, device);
            } else {
                result.push(device);
            }
        }
    }
    let mut seen = HashSet::new();
    result.retain(|device| seen.insert(device.key.clone()));
    result.sort_by(|a, b| a.product.cmp(&b.product).then(a.key.cmp(&b.key)));
    result
}

pub fn scan_all(hidpp: Vec<DeviceSummary>, generic_hid: Vec<DeviceSummary>) -> Vec<DeviceSummary> {
    let v4l2 = scan_v4l2();
    let pipewire = scan_pipewire();
    let specialized: Vec<&DeviceSummary> = hidpp
        .iter()
        .chain(generic_hid.iter())
        .chain(v4l2.iter())
        .chain(pipewire.iter())
        .collect();
    let usb = scan_usb_inventory()
        .into_iter()
        .filter(|usb| {
            !specialized.iter().any(|device| {
                device.product_id != 0
                    && device.product_id == usb.product_id
                    && normalized_product(&device.product) == normalized_product(&usb.product)
            })
        })
        .collect::<Vec<_>>();
    merge_devices([hidpp, generic_hid, v4l2, pipewire, usb])
}

pub fn validate_control_value(control: &DeviceControl, value: &ControlValue) -> Result<(), String> {
    if !control.writable {
        return Err(format!("control is read-only: {}", control.label));
    }
    match control.kind {
        ControlKind::Toggle => {
            if value.as_bool().is_none() {
                return Err(format!("{} requires an on/off value", control.label));
            }
        }
        ControlKind::Range => {
            let scalar = value
                .as_i64()
                .ok_or_else(|| format!("{} requires an integer value", control.label))?;
            if control.min.is_some_and(|min| scalar < min)
                || control.max.is_some_and(|max| scalar > max)
            {
                return Err(format!(
                    "value {scalar} is outside the supported range for {}",
                    control.label
                ));
            }
            if let (Some(min), Some(step)) = (control.min, control.step.filter(|step| *step > 1))
                && (scalar - min) % step != 0
            {
                return Err(format!(
                    "value {scalar} does not match step {step} for {}",
                    control.label
                ));
            }
        }
        ControlKind::Choice => {
            let scalar = value
                .as_i64()
                .ok_or_else(|| format!("{} requires a choice value", control.label))?;
            if !control.choices.iter().any(|choice| choice.value == scalar) {
                return Err(format!(
                    "value {scalar} is not a supported choice for {}",
                    control.label
                ));
            }
        }
        ControlKind::Action => {}
        ControlKind::Vector => {
            let values = value
                .as_vector()
                .ok_or_else(|| format!("{} requires a vector value", control.label))?;
            if let Some(expected) = control.value.as_vector().map(|values| values.len())
                && values.len() != expected
            {
                return Err(format!(
                    "{} requires exactly {expected} values (got {})",
                    control.label,
                    values.len()
                ));
            }
            for item in values {
                if control.min.is_some_and(|min| *item < min)
                    || control.max.is_some_and(|max| *item > max)
                {
                    return Err(format!(
                        "vector value {item} is outside the supported range for {}",
                        control.label
                    ));
                }
            }
        }
        ControlKind::Status => {
            return Err(format!("status control is not writable: {}", control.label));
        }
    }
    Ok(())
}

fn set_standard_control(control: &DeviceControl, value: &ControlValue) -> Result<(), String> {
    let scalar = value.as_i64().unwrap_or(1);
    let status = match control.provider {
        ProviderKind::V4l2 => Command::new("v4l2-ctl")
            .args([
                "-d",
                &control.endpoint,
                "--set-ctrl",
                &format!("{}={scalar}", control.backend_id),
            ])
            .status(),
        ProviderKind::Pipewire if control.backend_id == "volume" => Command::new("wpctl")
            .args([
                "set-volume",
                &control.endpoint,
                &format!("{scalar}%"),
            ])
            .status(),
        ProviderKind::Pipewire if control.backend_id == "mute" => Command::new("wpctl")
            .args([
                "set-mute",
                &control.endpoint,
                if value.as_bool().unwrap_or(false) { "1" } else { "0" },
            ])
            .status(),
        provider => {
            return Err(format!(
                "provider {provider:?} does not expose a Linux-standard writable control for {}",
                control.label
            ));
        }
    }
    .map_err(|error| format!("failed to launch control backend for {}: {error}", control.label))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("control backend rejected {}", control.label))
    }
}

pub fn set_control(
    device: &DeviceSummary,
    control_id: &str,
    value: &ControlValue,
) -> Result<ControlValue, String> {
    let control = device
        .controls
        .iter()
        .find(|control| control.id == control_id)
        .ok_or_else(|| format!("control is not available on {}: {control_id}", device.product))?;
    validate_control_value(control, value)?;
    match control.provider {
        ProviderKind::Hidpp => reforge_hid::set_control(device, control, value),
        ProviderKind::V4l2 | ProviderKind::Pipewire => {
            set_standard_control(control, value)?;
            Ok(value.clone())
        }
        provider => Err(format!(
            "provider {provider:?} is inventory-only for {}",
            control.label
        )),
    }
}

pub fn get_control(device: &DeviceSummary, control_id: &str) -> Result<ControlValue, String> {
    let control = device
        .controls
        .iter()
        .find(|control| control.id == control_id)
        .ok_or_else(|| format!("control is not available on {}: {control_id}", device.product))?;
    match control.provider {
        ProviderKind::Hidpp => reforge_hid::read_control(device, control),
        ProviderKind::V4l2 if control.readback == ReadbackKind::Readable => {
            let output = Command::new("v4l2-ctl")
                .args(["-d", &control.endpoint, "--get-ctrl", &control.backend_id])
                .output()
                .map_err(|error| format!("failed to read {}: {error}", control.label))?;
            if !output.status.success() {
                return Err(format!("V4L2 rejected read for {}", control.label));
            }
            let text = String::from_utf8_lossy(&output.stdout);
            let scalar = text.split(':').nth(1).and_then(|value| value.trim().parse::<i64>().ok())
                .ok_or_else(|| format!("could not parse V4L2 value for {}", control.label))?;
            Ok(if control.kind == ControlKind::Toggle { ControlValue::Bool(scalar != 0) } else { ControlValue::Int(scalar) })
        }
        ProviderKind::Pipewire => {
            let output = Command::new("wpctl")
                .args(["get-volume", &control.endpoint])
                .output()
                .map_err(|error| format!("failed to read {}: {error}", control.label))?;
            if !output.status.success() {
                return Err(format!("PipeWire rejected read for {}", control.label));
            }
            let text = String::from_utf8_lossy(&output.stdout);
            if control.backend_id == "mute" {
                Ok(ControlValue::Bool(text.contains("MUTED")))
            } else {
                let volume = text.split_whitespace().find_map(|token| token.parse::<f64>().ok())
                    .ok_or_else(|| format!("could not parse PipeWire volume for {}", control.label))?;
                Ok(ControlValue::Int((volume * 100.0).round() as i64))
            }
        }
        _ => Ok(control.value.clone()),
    }
}


pub fn read_telemetry(
    device: &DeviceSummary,
    presence: DevicePresence,
    updated_unix_ms: u64,
) -> DeviceTelemetry {
    let online = presence == DevicePresence::Online;
    let provider_health = device
        .providers
        .iter()
        .copied()
        .map(|provider| (provider, online))
        .collect();

    let battery_percent = device
        .controls
        .iter()
        .find(|control| control.id == "hidpp:battery_percent")
        .and_then(|control| control.value.as_i64())
        .map(|value| value.clamp(0, 100) as u8);

    let active_dpi = device
        .controls
        .iter()
        .find(|control| control.id == "hidpp:dpi")
        .and_then(|control| control.value.as_i64())
        .and_then(|value| u16::try_from(value).ok())
        .or_else(|| device.dpi.as_ref().map(|dpi| dpi.current));

    DeviceTelemetry {
        key: device.key.clone(),
        presence,
        battery_percent,
        charging: None,
        active_dpi,
        provider_health,
        updated_unix_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v4l2_range_bool_and_menu_controls() {
        let text = r#"brightness 0x00980900 (int)    : min=0 max=255 step=1 default=128 value=140
focus_automatic_continuous 0x009a090c (bool)   : default=1 value=1
exposure_auto 0x009a0901 (menu)   : min=0 max=3 default=3 value=1
    0: Auto Mode
    1: Manual Mode
"#;
        let controls = parse_v4l2_controls("/dev/video0", text);
        assert_eq!(controls.len(), 3);
        assert_eq!(controls[0].id, "v4l2:brightness");
        assert_eq!(controls[0].value, ControlValue::Int(140));
        assert_eq!(controls[1].value, ControlValue::Bool(true));
        assert_eq!(controls[2].choices.len(), 2);
    }

    #[test]
    fn v4l2_read_only_and_disabled_controls_are_not_writable() {
        let text = r#"privacy 0x009a0910 (bool) : default=0 value=0 flags=read-only
focus_absolute 0x009a090a (int) : min=0 max=255 step=5 default=0 value=0 flags=disabled
"#;
        let controls = parse_v4l2_controls("/dev/video0", text);
        assert_eq!(controls.len(), 2);
        assert!(controls.iter().all(|control| !control.writable));
        assert!(controls.iter().all(|control| control.readback == ReadbackKind::Telemetry));
    }

    #[test]
    fn validates_range_step_and_choices() {
        let range = DeviceControl {
            id: "v4l2:zoom".into(),
            label: "Zoom".into(),
            provider: ProviderKind::V4l2,
            endpoint: "/dev/video0".into(),
            backend_id: "zoom".into(),
            kind: ControlKind::Range,
            value: ControlValue::Int(100),
            min: Some(100), max: Some(500), step: Some(5),
            choices: vec![], writable: true, profile_eligible: true,
            group: ControlGroup::Camera, readback: ReadbackKind::Readable,
        };
        assert!(validate_control_value(&range, &ControlValue::Int(105)).is_ok());
        assert!(validate_control_value(&range, &ControlValue::Int(106)).is_err());
    }

    #[test]
    fn merges_matching_hardware_and_keeps_provider_controls() {
        let hid = DeviceSummary {
            key: "hid:one".into(), path: "/dev/hidraw0".into(), device_index: 0xff,
            vendor_id: LOGITECH_VENDOR_ID, product_id: 0x1234, product: "Logitech Test".into(), serial: Some("ABC".into()), hardware_id: None,
            interface_number: 2, usage_page: 0xff00, usage: 1, transport: TransportKind::Usb, hidpp: true,
            device_class: DeviceClass::Keyboard, providers: vec![ProviderKind::Hidpp], controls: vec![], features: vec![], dpi: None, lighting: None, error: None,
        };
        let mut camera = hid.clone();
        camera.key = "usb:1-1".into();
        camera.hidpp = false;
        camera.device_class = DeviceClass::Camera;
        camera.providers = vec![ProviderKind::V4l2];
        camera.controls = vec![DeviceControl {
            id: "v4l2:zoom_absolute".into(), label: "Zoom".into(), provider: ProviderKind::V4l2,
            endpoint: "/dev/video0".into(), backend_id: "zoom_absolute".into(), kind: ControlKind::Range,
            value: ControlValue::Int(100), min: Some(100), max: Some(500), step: Some(1), choices: vec![],
            writable: true, profile_eligible: true, group: ControlGroup::Camera, readback: ReadbackKind::Readable,
        }];
        let merged = merge_devices([vec![hid.clone()], vec![camera]]);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].providers.contains(&ProviderKind::Hidpp));
        assert!(merged[0].providers.contains(&ProviderKind::V4l2));
        assert_eq!(merged[0].controls.len(), 1);
    }
}
