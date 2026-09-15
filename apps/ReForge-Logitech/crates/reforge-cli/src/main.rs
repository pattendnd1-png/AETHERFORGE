use clap::{Parser, Subcommand};
use reforge_core::{
    client, diagnostics::redact_snapshot, paths, ControlKind, ControlValue, DevicePresence, DeviceSummary, LightingEffect, LightingState,
    Profile, RgbColor, RpcData, RpcRequest, RpcResponse, ServiceKind,
};
use std::fs;

#[derive(Debug, Parser)]
#[command(name = "reforge-logitechctl", version, about = "Control ReForge Logitech Linux")]
struct Cli {
    #[arg(long, global = true, help = "Print machine-readable JSON")]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Health,
    Devices,
    Runtime,
    Rescan,
    Telemetry { device: usize },
    Dpi {
        #[command(subcommand)]
        command: DpiCommand,
    },
    Lighting {
        #[command(subcommand)]
        command: LightingCommand,
    },
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    Controls { device: usize },
    Control {
        #[command(subcommand)]
        command: ControlCommand,
    },
    Services {
        #[command(subcommand)]
        command: Option<ServicesCommand>,
    },
    Session { device: usize },
    Firmware {
        #[command(subcommand)]
        command: FirmwareCommand,
    },
    Integrations,
    Diagnostics {
        #[command(subcommand)]
        command: DiagnosticsCommand,
    },
}


#[derive(Debug, Subcommand)]
enum ControlCommand {
    Get { device: usize, control: String },
    Set { device: usize, control: String, value: String },
}

#[derive(Debug, Subcommand)]
enum DpiCommand {
    Get { device: usize },
    Set { device: usize, dpi: u16 },
}

#[derive(Debug, Subcommand)]
enum LightingCommand {
    Get { device: usize },
    Set {
        device: usize,
        #[arg(long, default_value = "static")]
        effect: String,
        #[arg(long, default_value = "#009dff")]
        color: String,
        #[arg(long, default_value = "#8a2be2")]
        secondary: String,
        #[arg(long)]
        brightness: Option<u16>,
        #[arg(long, default_value_t = 3000)]
        period: u16,
        #[arg(long, default_value_t = 100)]
        intensity: u8,
        #[arg(long, default_value_t = 1)]
        direction: u8,
    },
    Key {
        device: usize,
        key: u8,
        color: String,
    },
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    List,
    Save {
        name: String,
        device: usize,
        #[arg(long)]
        dpi: Option<u16>,
        #[arg(long = "app")]
        applications: Vec<String>,
        #[arg(long)]
        auto_switch: bool,
    },
    Apply { name: String, device: usize },
    Rename { old_name: String, new_name: String },
    Clone { source_name: String, new_name: String },
    Delete { name: String },
    Import { path: String, #[arg(long)] replace: bool },
    Export { path: String, names: Vec<String> },
}

#[derive(Debug, Subcommand)]
enum ServicesCommand {
    Reconnect { service: String },
}

#[derive(Debug, Subcommand)]
enum FirmwareCommand {
    Devices,
    Refresh,
    Releases { device_id: String },
    Update { device_id: String },
}

#[derive(Debug, Subcommand)]
enum DiagnosticsCommand {
    Show,
    Clear,
    Export { path: String, #[arg(long)] include_device_identifiers: bool },
}

fn rpc(request: RpcRequest) -> Result<RpcData, String> {
    match client::call(&request).map_err(|error| {
        format!(
            "{error}\nHint: start the service with: systemctl --user start reforge-logitechd.service"
        )
    })? {
        RpcResponse::Ok { data } => Ok(data),
        RpcResponse::Err { message } => Err(message),
    }
}

fn devices() -> Result<Vec<DeviceSummary>, String> {
    match rpc(RpcRequest::ListDevices)? {
        RpcData::Devices(devices) => Ok(devices),
        _ => Err("daemon returned an unexpected response to list_devices".into()),
    }
}

fn indexed_device(index: usize) -> Result<DeviceSummary, String> {
    let devices = devices()?;
    if index == 0 || index > devices.len() {
        return Err(format!(
            "device index {index} is invalid; run 'reforge-logitechctl devices' to list {} device(s)",
            devices.len()
        ));
    }
    Ok(devices[index - 1].clone())
}

fn print_json(data: &RpcData) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(data)
            .map_err(|error| format!("failed to encode output JSON: {error}"))?
    );
    Ok(())
}

fn parse_color(value: &str) -> Result<RgbColor, String> {
    let trimmed = value.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return Err(format!("color '{value}' must be #RRGGBB"));
    }
    let packed = u32::from_str_radix(trimmed, 16)
        .map_err(|_| format!("color '{value}' must be hexadecimal #RRGGBB"))?;
    Ok(RgbColor::from_packed(packed))
}

fn parse_effect(value: &str) -> Result<LightingEffect, String> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "off" => Ok(LightingEffect::Off),
        "static" => Ok(LightingEffect::Static),
        "breathing" | "breathe" => Ok(LightingEffect::Breathing),
        "cycle" | "color_cycle" => Ok(LightingEffect::ColorCycle),
        "wave" => Ok(LightingEffect::Wave),
        "ripple" => Ok(LightingEffect::Ripple),
        "gradient" => Ok(LightingEffect::Gradient),
        "screen" | "screen_reactive" => Ok(LightingEffect::ScreenReactive),
        "audio" | "audio_reactive" => Ok(LightingEffect::AudioReactive),
        _ => Err(format!(
            "unknown effect '{value}'; use off, static, breathing, cycle, wave, ripple, gradient, screen, or audio"
        )),
    }
}

fn format_control_value(value: &ControlValue) -> String {
    match value {
        ControlValue::Bool(value) => value.to_string(),
        ControlValue::Int(value) => value.to_string(),
        ControlValue::Vector(values) => values.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
        ControlValue::None => "n/a".into(),
    }
}

fn parse_control_value(kind: ControlKind, raw: &str) -> Result<ControlValue, String> {
    match kind {
        ControlKind::Toggle => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "on" | "yes" => Ok(ControlValue::Bool(true)),
            "0" | "false" | "off" | "no" => Ok(ControlValue::Bool(false)),
            _ => Err(format!("toggle value '{raw}' must be on/off, true/false, or 1/0")),
        },
        ControlKind::Vector => {
            let values = raw.split(',').map(|part| part.trim().parse::<i64>().map_err(|_| format!("invalid vector value '{part}'"))).collect::<Result<Vec<_>, _>>()?;
            Ok(ControlValue::Vector(values))
        }
        ControlKind::Action => Ok(ControlValue::None),
        ControlKind::Status => Err("status controls are read-only".into()),
        ControlKind::Range | ControlKind::Choice => raw.trim().parse::<i64>().map(ControlValue::Int).map_err(|_| format!("invalid integer value '{raw}'")),
    }
}

fn parse_service_kind(value: &str) -> Result<ServiceKind, String> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "hidpp" | "device" | "logitech" => Ok(ServiceKind::Hidpp),
        "udev" | "hotplug" => Ok(ServiceKind::Udev),
        "fwupd" | "firmware" => Ok(ServiceKind::Fwupd),
        "v4l2" | "camera" => Ok(ServiceKind::V4l2),
        "pipewire" | "audio" => Ok(ServiceKind::Pipewire),
        _ => Err(format!("unknown service '{value}'; use hidpp, udev, fwupd, v4l2, or pipewire")),
    }
}

fn print_devices(items: &[DeviceSummary]) {
    for (index, device) in items.iter().enumerate() {
        let dpi = device
            .dpi
            .as_ref()
            .map(|value| format!("{} DPI", value.current))
            .unwrap_or_else(|| "no DPI control".into());
        let lighting = device
            .lighting
            .as_ref()
            .map(|caps| {
                let mut labels = Vec::new();
                if caps.per_key_v2 {
                    labels.push(format!("per-key RGB ({} zones)", caps.supported_keys.len()));
                } else if caps.rgb_effects || caps.color_led_effects {
                    labels.push(format!("{} LED zone(s)", caps.zones.len()));
                }
                if caps.backlight_v2 {
                    labels.push("keyboard backlight".into());
                }
                if caps.brightness.is_some() {
                    labels.push("brightness".into());
                }
                if labels.is_empty() {
                    "lighting detected".into()
                } else {
                    labels.join(", ")
                }
            })
            .unwrap_or_else(|| "no lighting control".into());
        let providers = if device.providers.is_empty() { String::new() } else {
            format!(" — {:?} via {}", device.device_class, device.providers.iter().map(|p| format!("{p:?}")).collect::<Vec<_>>().join("+"))
        };
        let controls = if device.controls.is_empty() { String::new() } else { format!(" — {} standard control(s)", device.controls.len()) };
        println!("{}. {} — {dpi} — {lighting}{providers}{controls}", index + 1, device.product);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Health => {
            let data = rpc(RpcRequest::Health)?;
            if cli.json {
                print_json(&data)
            } else if let RpcData::Health { version } = data {
                println!("ReForge Logitech daemon {version} is healthy.");
                Ok(())
            } else {
                Err("daemon returned an unexpected health response".into())
            }
        }
        Command::Devices => {
            let data = rpc(RpcRequest::ListDevices)?;
            if cli.json {
                print_json(&data)
            } else if let RpcData::Devices(devices) = data {
                print_devices(&devices);
                Ok(())
            } else {
                Err("daemon returned an unexpected device list response".into())
            }
        }
        Command::Runtime => {
            let data = rpc(RpcRequest::ListDeviceRuntime)?;
            if cli.json {
                print_json(&data)
            } else if let RpcData::DeviceRuntime(items) = data {
                for (index, item) in items.iter().enumerate() {
                    println!(
                        "{}. {} — {:?} — reconnects {} — key {}",
                        index + 1,
                        item.summary.product,
                        item.presence,
                        item.reconnect_count,
                        item.summary.key
                    );
                }
                Ok(())
            } else {
                Err("daemon returned an unexpected runtime response".into())
            }
        }
        Command::Rescan => {
            let data = rpc(RpcRequest::RescanDevices)?;
            if cli.json {
                print_json(&data)
            } else if let RpcData::DeviceRuntime(items) = data {
                let online = items.iter().filter(|item| item.presence == DevicePresence::Online).count();
                println!("Rescan complete: {online} online, {} tracked.", items.len());
                Ok(())
            } else {
                Err("daemon returned an unexpected rescan response".into())
            }
        }
        Command::Telemetry { device } => {
            let selected = indexed_device(device)?;
            let data = rpc(RpcRequest::GetDeviceTelemetry { key: selected.key })?;
            if cli.json {
                print_json(&data)
            } else if let RpcData::Telemetry(value) = data {
                println!("Presence: {:?}", value.presence);
                println!("Battery: {}", value.battery_percent.map(|v| format!("{v}%")).unwrap_or_else(|| "n/a".into()));
                println!("DPI: {}", value.active_dpi.map(|v| v.to_string()).unwrap_or_else(|| "n/a".into()));
                println!("Providers: {:?}", value.provider_health);
                Ok(())
            } else {
                Err("daemon returned an unexpected telemetry response".into())
            }
        }
        Command::Dpi { command } => match command {
            DpiCommand::Get { device } => {
                let selected = indexed_device(device)?;
                let state = selected
                    .dpi
                    .ok_or_else(|| format!("{} does not expose adjustable DPI", selected.product))?;
                if cli.json {
                    print_json(&RpcData::Dpi(state))
                } else {
                    println!(
                        "{}: {} DPI (range {}–{})",
                        selected.product, state.current, state.min, state.max
                    );
                    Ok(())
                }
            }
            DpiCommand::Set { device, dpi } => {
                let selected = indexed_device(device)?;
                let data = rpc(RpcRequest::SetDpi {
                    key: selected.key,
                    dpi,
                })?;
                if cli.json {
                    print_json(&data)
                } else if let RpcData::Dpi(state) = data {
                    println!("{} DPI verified at {}.", selected.product, state.current);
                    Ok(())
                } else {
                    Err("daemon returned an unexpected DPI response".into())
                }
            }
        },
        Command::Lighting { command } => match command {
            LightingCommand::Get { device } => {
                let selected = indexed_device(device)?;
                let data = rpc(RpcRequest::GetLighting {
                    key: selected.key,
                })?;
                if cli.json {
                    print_json(&data)
                } else if let RpcData::Lighting(state) = data {
                    println!(
                        "{}: {:?}, primary #{:06X}, brightness {:?}",
                        selected.product,
                        state.effect,
                        state.primary.packed(),
                        state.brightness
                    );
                    Ok(())
                } else {
                    Err("daemon returned an unexpected lighting response".into())
                }
            }
            LightingCommand::Set {
                device,
                effect,
                color,
                secondary,
                brightness,
                period,
                intensity,
                direction,
            } => {
                let selected = indexed_device(device)?;
                let state = LightingState {
                    enabled: effect.trim().to_ascii_lowercase() != "off",
                    effect: parse_effect(&effect)?,
                    primary: parse_color(&color)?,
                    secondary: parse_color(&secondary)?,
                    brightness,
                    period_ms: period,
                    intensity,
                    direction,
                    ..Default::default()
                };
                let data = rpc(RpcRequest::SetLighting {
                    key: selected.key,
                    state,
                })?;
                if cli.json {
                    print_json(&data)
                } else {
                    println!("Applied lighting to {}.", selected.product);
                    Ok(())
                }
            }
            LightingCommand::Key { device, key, color } => {
                let selected = indexed_device(device)?;
                let data = rpc(RpcRequest::SetKeyColor {
                    key: selected.key,
                    key_id: key,
                    color: parse_color(&color)?,
                })?;
                if cli.json {
                    print_json(&data)
                } else {
                    println!("Set key zone {key:#04x} on {}.", selected.product);
                    Ok(())
                }
            }
        },
        Command::Profile { command } => match command {
            ProfileCommand::List => {
                let data = rpc(RpcRequest::ListProfiles)?;
                if cli.json {
                    print_json(&data)
                } else if let RpcData::Profiles(profiles) = data {
                    for profile in profiles {
                        let apps = if profile.applications.is_empty() {
                            String::new()
                        } else {
                            format!(" — apps: {}", profile.applications.join(", "))
                        };
                        println!("{}{}", profile.name, apps);
                    }
                    Ok(())
                } else {
                    Err("daemon returned an unexpected profile list response".into())
                }
            }
            ProfileCommand::Save {
                name,
                device,
                dpi,
                applications,
                auto_switch,
            } => {
                let selected = indexed_device(device)?;
                let dpi = dpi.or(selected.dpi.as_ref().map(|state| state.current));
                let lighting = if selected.lighting.is_some() {
                    match rpc(RpcRequest::GetLighting {
                        key: selected.key.clone(),
                    })? {
                        RpcData::Lighting(state) => Some(state),
                        _ => None,
                    }
                } else {
                    None
                };
                let profile = Profile {
                    name: name.clone(),
                    product_id: (selected.product_id != 0).then_some(selected.product_id),
                    product: Some(selected.product.clone()),
                    serial: selected.serial.clone(),
                    dpi,
                    lighting,
                    applications,
                    auto_switch,
                    controls: selected.controls.iter().filter(|control| control.writable && control.profile_eligible).map(|control| (control.id.clone(), control.value.clone())).collect(),
                };
                let data = rpc(RpcRequest::SaveProfile { profile })?;
                if cli.json {
                    print_json(&data)
                } else {
                    println!("Saved profile '{name}' for {}.", selected.product);
                    Ok(())
                }
            }
            ProfileCommand::Apply { name, device } => {
                let selected = indexed_device(device)?;
                let data = rpc(RpcRequest::ApplyProfile {
                    profile_name: name.clone(),
                    key: selected.key.clone(),
                })?;
                if cli.json {
                    print_json(&data)
                } else {
                    println!("Applied profile '{name}' to {}.", selected.product);
                    Ok(())
                }
            }

            ProfileCommand::Rename { old_name, new_name } => {
                let data = rpc(RpcRequest::RenameProfile { old_name: old_name.clone(), new_name: new_name.clone() })?;
                if cli.json { print_json(&data) } else { println!("Renamed '{old_name}' to '{new_name}'."); Ok(()) }
            }
            ProfileCommand::Clone { source_name, new_name } => {
                let data = rpc(RpcRequest::CloneProfile { source_name: source_name.clone(), new_name: new_name.clone() })?;
                if cli.json { print_json(&data) } else { println!("Cloned '{source_name}' as '{new_name}'."); Ok(()) }
            }
            ProfileCommand::Delete { name } => {
                let data = rpc(RpcRequest::DeleteProfile { name: name.clone() })?;
                if cli.json { print_json(&data) } else { println!("Deleted '{name}'."); Ok(()) }
            }
            ProfileCommand::Import { path, replace } => {
                let json = fs::read_to_string(&path).map_err(|error| format!("failed to read {path}: {error}"))?;
                let data = rpc(RpcRequest::ImportProfiles { json, replace })?;
                if cli.json { print_json(&data) } else { println!("Imported profiles from {path}."); Ok(()) }
            }
            ProfileCommand::Export { path, names } => {
                let data = rpc(RpcRequest::ExportProfiles { names })?;
                if let RpcData::ExportedProfiles { json } = &data {
                    fs::write(&path, json).map_err(|error| format!("failed to write {path}: {error}"))?;
                }
                if cli.json { print_json(&data) } else { println!("Exported profiles to {path}."); Ok(()) }
            }
        },
        Command::Controls { device } => {
            let selected = indexed_device(device)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&selected.controls).map_err(|error| error.to_string())?);
                Ok(())
            } else {
                if selected.controls.is_empty() { println!("{} has no Linux-standard controls.", selected.product); }
                for control in selected.controls {
                    println!("{} = {} — {:?} via {:?}", control.id, format_control_value(&control.value), control.kind, control.provider);
                }
                Ok(())
            }
        }
        Command::Control { command } => match command {
            ControlCommand::Get { device, control } => {
                let selected = indexed_device(device)?;
                let data = rpc(RpcRequest::GetControl { key: selected.key.clone(), control_id: control.clone() })?;
                if cli.json { print_json(&data) } else if let RpcData::Control(value) = data {
                    println!("{}: {} = {}", selected.product, control, format_control_value(&value));
                    Ok(())
                } else { Err("daemon returned an unexpected control response".into()) }
            }
            ControlCommand::Set { device, control, value } => {
                let selected = indexed_device(device)?;
                let advertised = selected.controls.iter().find(|item| item.id == control)
                    .ok_or_else(|| format!("{} does not advertise control {control}", selected.product))?;
                let typed = parse_control_value(advertised.kind, &value)?;
                let data = rpc(RpcRequest::SetControl { key: selected.key.clone(), control_id: control.clone(), value: typed.clone() })?;
                if cli.json { print_json(&data) } else {
                    println!("Set {control} to {} on {}.", format_control_value(&typed), selected.product);
                    Ok(())
                }
            }
        },
        Command::Services { command } => match command {
            None => {
                let data = rpc(RpcRequest::ListServices)?;
                if cli.json {
                    print_json(&data)
                } else if let RpcData::Services(items) = data {
                    for item in items {
                        println!(
                            "{:?}: {:?} — {}{}{}",
                            item.kind,
                            item.state,
                            item.backend,
                            item.version.as_ref().map(|value| format!(" — {value}")).unwrap_or_default(),
                            item.message.as_ref().map(|value| format!(" — {value}")).unwrap_or_default(),
                        );
                    }
                    Ok(())
                } else {
                    Err("daemon returned an unexpected service-list response".into())
                }
            }
            Some(ServicesCommand::Reconnect { service }) => {
                let kind = parse_service_kind(&service)?;
                let data = rpc(RpcRequest::ReconnectService { service: kind })?;
                if cli.json { print_json(&data) } else { println!("Reconnect requested for {kind:?}."); Ok(()) }
            }
        },
        Command::Session { device } => {
            let selected = indexed_device(device)?;
            let data = rpc(RpcRequest::GetSessionStatus { key: selected.key.clone() })?;
            if cli.json {
                print_json(&data)
            } else if let RpcData::SessionStatus(status) = data {
                println!("{}: {:?}", selected.product, status.state);
                println!("Path: {}", status.path);
                println!("Device index: {:#04x}", status.device_index);
                println!("Reconnects: {}", status.reconnect_count);
                if let Some(error) = status.last_error { println!("Last error: {error}"); }
                Ok(())
            } else {
                Err("daemon returned an unexpected HID++ session response".into())
            }
        }
        Command::Firmware { command } => match command {
            FirmwareCommand::Devices => {
                let data = rpc(RpcRequest::ListFirmwareDevices)?;
                if cli.json {
                    print_json(&data)
                } else if let RpcData::FirmwareDevices(devices) = data {
                    if devices.is_empty() {
                        println!("No Logitech firmware devices are currently exposed by fwupd.");
                    }
                    for device in devices {
                        println!(
                            "{} — {} — {} — fwupd {}{}",
                            device.name,
                            device.version.as_deref().unwrap_or("unknown firmware"),
                            if device.update_available { "UPDATE AVAILABLE" } else { "current/no advertised update" },
                            device.id,
                            device.matched_device_key.as_ref().map(|key| format!(" — ReForge {key}")).unwrap_or_default(),
                        );
                    }
                    Ok(())
                } else {
                    Err("daemon returned an unexpected firmware-device response".into())
                }
            }
            FirmwareCommand::Refresh => {
                let data = rpc(RpcRequest::RefreshFirmwareMetadata)?;
                if cli.json { print_json(&data) } else { println!("LVFS metadata refreshed through fwupd."); Ok(()) }
            }
            FirmwareCommand::Releases { device_id } => {
                let data = rpc(RpcRequest::GetFirmwareReleases { device_id })?;
                if cli.json {
                    print_json(&data)
                } else if let RpcData::FirmwareReleases(releases) = data {
                    for release in releases {
                        println!(
                            "{} — {}{}{}",
                            release.version,
                            release.summary.as_deref().unwrap_or("firmware release"),
                            if release.needs_replug { " — replug required" } else { "" },
                            if release.needs_reboot { " — reboot required" } else { "" },
                        );
                    }
                    Ok(())
                } else {
                    Err("daemon returned an unexpected firmware-release response".into())
                }
            }
            FirmwareCommand::Update { device_id } => {
                let data = rpc(RpcRequest::InstallFirmwareUpdate { device_id })?;
                if cli.json {
                    print_json(&data)
                } else if let RpcData::FirmwareInstall(result) = data {
                    println!("{}", result.message);
                    if result.needs_replug { println!("Device replug is required."); }
                    if result.needs_reboot { println!("System reboot is required."); }
                    Ok(())
                } else {
                    Err("daemon returned an unexpected firmware-install response".into())
                }
            }
        },
        Command::Integrations => {
            let data = rpc(RpcRequest::IntegrationStatus)?;
            if cli.json {
                print_json(&data)
            } else if let RpcData::Integrations(status) = data {
                println!(
                    "Screen: {}",
                    status.screen_capture.as_deref().unwrap_or("unavailable")
                );
                println!(
                    "Audio: {}",
                    status.audio_capture.as_deref().unwrap_or("unavailable")
                );
                println!("Process watcher: {}", if status.process_watcher { "ready" } else { "unavailable" });
                println!(
                    "Active auto profile: {}",
                    status.active_auto_profile.as_deref().unwrap_or("none")
                );
                Ok(())
            } else {
                Err("daemon returned an unexpected integration response".into())
            }
        }
        Command::Diagnostics { command } => match command {
            DiagnosticsCommand::Show => {
                let data = rpc(RpcRequest::GetDiagnostics)?;
                if cli.json { print_json(&data) } else if let RpcData::Diagnostics(snapshot) = data {
                    println!("ReForge {} — {} tracked device(s) — {} recent event(s)", snapshot.daemon_version, snapshot.devices.len(), snapshot.recent_events.len());
                    for event in snapshot.recent_events.iter().rev().take(50) {
                        println!("[{:?}] {}: {}", event.level, event.component, event.message);
                    }
                    Ok(())
                } else { Err("daemon returned an unexpected diagnostics response".into()) }
            }
            DiagnosticsCommand::Clear => {
                let data = rpc(RpcRequest::ClearDiagnostics)?;
                if cli.json { print_json(&data) } else { println!("Diagnostics cleared."); Ok(()) }
            }
            DiagnosticsCommand::Export { path, include_device_identifiers } => {
                let data = rpc(RpcRequest::GetDiagnostics)?;
                let RpcData::Diagnostics(snapshot) = data else { return Err("daemon returned an unexpected diagnostics response".into()); };
                let bundle = redact_snapshot(snapshot, include_device_identifiers);
                let json = serde_json::to_string_pretty(&bundle).map_err(|error| format!("failed to encode diagnostics: {error}"))?;
                let output = if path.trim().is_empty() { paths::diagnostic_export_dir().join("reforge-diagnostics.json") } else { std::path::PathBuf::from(path) };
                if let Some(parent) = output.parent() { fs::create_dir_all(parent).map_err(|error| format!("failed to create {}: {error}", parent.display()))?; }
                fs::write(&output, json).map_err(|error| format!("failed to write {}: {error}", output.display()))?;
                println!("Exported diagnostics to {}.", output.display());
                Ok(())
            }
        }
    }
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("reforge-logitechctl: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reliability_commands() {
        let cli = Cli::try_parse_from(["reforge-logitechctl", "runtime"]).unwrap();
        assert!(matches!(cli.command, Command::Runtime));

        let cli = Cli::try_parse_from(["reforge-logitechctl", "rescan"]).unwrap();
        assert!(matches!(cli.command, Command::Rescan));

        let cli = Cli::try_parse_from(["reforge-logitechctl", "telemetry", "1"]).unwrap();
        assert!(matches!(cli.command, Command::Telemetry { device: 1 }));

        let cli = Cli::try_parse_from(["reforge-logitechctl", "profile", "rename", "Old", "New"]).unwrap();
        assert!(matches!(cli.command, Command::Profile { command: ProfileCommand::Rename { .. } }));

        let cli = Cli::try_parse_from(["reforge-logitechctl", "diagnostics", "export", "diag.json"]).unwrap();
        assert!(matches!(cli.command, Command::Diagnostics { command: DiagnosticsCommand::Export { .. } }));

        let cli = Cli::try_parse_from(["reforge-logitechctl", "services"]).unwrap();
        assert!(matches!(cli.command, Command::Services { command: None }));

        let cli = Cli::try_parse_from(["reforge-logitechctl", "services", "reconnect", "hidpp"]).unwrap();
        assert!(matches!(cli.command, Command::Services { command: Some(ServicesCommand::Reconnect { .. }) }));

        let cli = Cli::try_parse_from(["reforge-logitechctl", "session", "1"]).unwrap();
        assert!(matches!(cli.command, Command::Session { device: 1 }));

        let cli = Cli::try_parse_from(["reforge-logitechctl", "firmware", "devices"]).unwrap();
        assert!(matches!(cli.command, Command::Firmware { command: FirmwareCommand::Devices }));

        let cli = Cli::try_parse_from(["reforge-logitechctl", "firmware", "update", "device-id"]).unwrap();
        assert!(matches!(cli.command, Command::Firmware { command: FirmwareCommand::Update { .. } }));
    }

    #[test]
    fn parses_hex_rgb() {
        assert_eq!(parse_color("#123456").unwrap(), RgbColor::new(0x12, 0x34, 0x56));
    }

    #[test]
    fn parses_effect_aliases() {
        assert_eq!(parse_effect("screen").unwrap(), LightingEffect::ScreenReactive);
        assert_eq!(parse_effect("color-cycle").unwrap(), LightingEffect::ColorCycle);
    }
}
