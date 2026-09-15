mod diagnostics;
mod providers;
mod registry;
mod services;
mod udev_monitor;
mod fwupd;

use reforge_core::{
    effects::render_host_frame, paths, ControlGroup, DesiredDeviceState, DevicePresence, DeviceSummary,
    DeviceTelemetry, DiagnosticLevel, DiagnosticSnapshot, IntegrationStatus, LightingEffect, LightingState,
    Profile, ProfileStore, ProviderKind, RgbColor, RpcData, RpcRequest, RpcResponse,
    ServiceKind, ServiceState,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env, fs,
    io::{BufRead, BufReader, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
struct ActiveHostEffect {
    device: DeviceSummary,
    state: LightingState,
    started: Instant,
}

#[derive(Default)]
struct RuntimeState {
    registry: registry::DeviceRegistry,
    lighting_states: HashMap<String, LightingState>,
    desired_states: HashMap<String, DesiredDeviceState>,
    telemetry: HashMap<String, DeviceTelemetry>,
    host_effects: HashMap<String, ActiveHostEffect>,
    diagnostics: diagnostics::DiagnosticRing,
    active_auto_profile: Option<String>,
}

struct AppState {
    runtime: Mutex<RuntimeState>,
    sessions: reforge_hid::SessionManager,
    services: Mutex<services::ServiceRegistry>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            runtime: Mutex::new(RuntimeState::default()),
            sessions: reforge_hid::SessionManager::default(),
            services: Mutex::new(services::ServiceRegistry::default()),
        }
    }
}

type SharedState = Arc<AppState>;

static HID_IO_LOCK: Mutex<()> = Mutex::new(());

fn with_hid_io<T>(operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let _guard = HID_IO_LOCK
        .lock()
        .map_err(|_| "daemon HID I/O lock is poisoned".to_owned())?;
    operation()
}

fn ensure_hid_session(shared: &SharedState, device: &DeviceSummary) -> Result<(), String> {
    if !device.hidpp {
        return Ok(());
    }
    with_hid_io(|| shared.sessions.ensure(device).map(|_| ()))
}

fn with_hid_session<T>(
    shared: &SharedState,
    device: &DeviceSummary,
    operation: impl FnOnce(&hidapi::HidDevice) -> Result<T, String>,
) -> Result<T, String> {
    ensure_hid_session(shared, device)?;
    with_hid_io(|| shared.sessions.with_device(&device.key, operation))
}

fn set_device_control(
    shared: &SharedState,
    device: &DeviceSummary,
    control_id: &str,
    value: &reforge_core::ControlValue,
) -> Result<reforge_core::ControlValue, String> {
    let control = device
        .controls
        .iter()
        .find(|control| control.id == control_id)
        .ok_or_else(|| format!("control is not available on {}: {control_id}", device.product))?;
    providers::validate_control_value(control, value)?;
    if control.provider == ProviderKind::Hidpp {
        with_hid_session(shared, device, |handle| {
            reforge_hid::set_control_with_handle(handle, device, control, value)
        })
    } else {
        providers::set_control(device, control_id, value)
    }
}

fn get_device_control(
    shared: &SharedState,
    device: &DeviceSummary,
    control_id: &str,
) -> Result<reforge_core::ControlValue, String> {
    let control = device
        .controls
        .iter()
        .find(|control| control.id == control_id)
        .ok_or_else(|| format!("control is not available on {}: {control_id}", device.product))?;
    if control.provider == ProviderKind::Hidpp {
        with_hid_session(shared, device, |handle| {
            reforge_hid::read_control_with_handle(handle, device, control)
        })
    } else {
        providers::get_control(device, control_id)
    }
}

fn scan_all_devices() -> Result<Vec<DeviceSummary>, String> {
    let hidpp = with_hid_io(reforge_hid::scan_logitech_devices)?;
    let generic_hid = with_hid_io(reforge_hid::scan_logitech_nonusb_hid_inventory)?;
    Ok(providers::scan_all(hidpp, generic_hid))
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn record_event(
    shared: &SharedState,
    level: DiagnosticLevel,
    component: &str,
    message: impl Into<String>,
    device_key: Option<String>,
) {
    if let Ok(mut runtime) = shared.runtime.lock() {
        runtime
            .diagnostics
            .record(unix_ms(), level, component, message, device_key);
    }
}

fn merge_scan(shared: &SharedState, devices: Vec<DeviceSummary>) -> Vec<registry::RegistryTransition> {
    shared
        .runtime
        .lock()
        .map(|mut runtime| runtime.registry.merge_scan(unix_ms(), devices))
        .unwrap_or_default()
}

fn restore_group_rank(group: ControlGroup) -> u8 {
    match group {
        ControlGroup::Profiles => 0,
        ControlGroup::Performance | ControlGroup::Assignments => 1,
        ControlGroup::Lighting => 2,
        ControlGroup::Audio | ControlGroup::Camera => 3,
        ControlGroup::Power => 4,
        _ => 5,
    }
}

fn restore_device_state(shared: &SharedState, device: &DeviceSummary) -> Vec<String> {
    let desired = shared
        .runtime
        .lock()
        .ok()
        .and_then(|runtime| runtime.desired_states.get(&device.key).cloned());
    let Some(desired) = desired else {
        return Vec::new();
    };
    let mut failures = Vec::new();

    if let Some(dpi) = desired.dpi
        && device.dpi.is_some()
        && let Err(error) = with_hid_session(shared, device, |handle| reforge_hid::set_dpi_with_handle(handle, device, dpi))
    {
        failures.push(format!("DPI restore: {error}"));
    }

    let mut controls = desired.controls.iter().collect::<Vec<_>>();
    controls.sort_by_key(|(control_id, _)| {
        device
            .controls
            .iter()
            .find(|control| control.id == **control_id)
            .map(|control| restore_group_rank(control.group))
            .unwrap_or(9)
    });
    for (control_id, value) in controls {
        let Some(control) = device.controls.iter().find(|control| control.id == *control_id) else {
            continue;
        };
        if !control.writable || !control.profile_eligible {
            continue;
        }
        if let Err(error) = set_device_control(shared, device, control_id, value) {
            failures.push(format!("{} restore: {error}", control.label));
        }
    }

    if let Some(lighting) = desired.lighting
        && device.lighting.is_some()
        && let Err(error) = apply_lighting_state(shared, device, lighting)
    {
        failures.push(format!("lighting restore: {error}"));
    }
    failures
}

fn refresh_devices(shared: &SharedState) -> Result<Vec<DeviceSummary>, String> {
    let devices = scan_all_devices()?;
    let transitions = merge_scan(shared, devices.clone());
    let mut session_failures = Vec::new();
    for device in devices.iter().filter(|device| device.hidpp) {
        if let Err(error) = ensure_hid_session(shared, device) {
            session_failures.push(format!("{}: {error}", device.product));
        }
    }
    if let Ok(mut service_registry) = shared.services.lock() {
        if session_failures.is_empty() {
            service_registry.ready(
                ServiceKind::Hidpp,
                "hidapi/hidraw",
                Some("hidapi 2.6.6".into()),
                format!("{} persistent HID++ session(s) active", shared.sessions.status_all().len()),
                unix_ms(),
            );
        } else {
            service_registry.degraded(ServiceKind::Hidpp, "hidapi/hidraw", session_failures.join("; "));
        }
        if command_exists("v4l2-ctl") {
            service_registry.ready(ServiceKind::V4l2, "v4l2-ctl", None, "V4L2/UVC backend available", unix_ms());
        } else {
            service_registry.unavailable(ServiceKind::V4l2, "v4l2-ctl", "v4l2-ctl is not installed");
        }
        if command_exists("wpctl") {
            service_registry.ready(ServiceKind::Pipewire, "wpctl", None, "PipeWire control backend available", unix_ms());
        } else {
            service_registry.unavailable(ServiceKind::Pipewire, "wpctl", "wpctl is not installed");
        }
    }
    for transition in transitions {
        match transition.kind {
            registry::RegistryTransitionKind::Added => {
                record_event(shared, DiagnosticLevel::Info, "discovery", "device connected", Some(transition.key));
            }
            registry::RegistryTransitionKind::Offline => {
                shared.sessions.remove(&transition.key);
                if let Ok(mut runtime) = shared.runtime.lock() {
                    runtime.host_effects.remove(&transition.key);
                }
                record_event(shared, DiagnosticLevel::Warning, "discovery", "device went offline", Some(transition.key));
            }
            registry::RegistryTransitionKind::Reconnecting => {
                record_event(shared, DiagnosticLevel::Info, "discovery", "device reconnecting", Some(transition.key.clone()));
                let device = shared.runtime.lock().ok().and_then(|runtime| runtime.registry.get_summary(&transition.key));
                if let Some(device) = device {
                    let mut failures = Vec::new();
                    if let Err(error) = ensure_hid_session(shared, &device) {
                        failures.push(format!("session reconnect: {error}"));
                    }
                    failures.extend(restore_device_state(shared, &device));
                    if let Ok(mut runtime) = shared.runtime.lock() {
                        runtime.registry.mark_online(&transition.key);
                    }
                    if failures.is_empty() {
                        record_event(shared, DiagnosticLevel::Info, "restore", "reconnect restore complete", Some(transition.key));
                    } else {
                        record_event(shared, DiagnosticLevel::Warning, "restore", failures.join("; "), Some(transition.key));
                    }
                }
            }
            registry::RegistryTransitionKind::Removed => {
                shared.sessions.remove(&transition.key);
                record_event(shared, DiagnosticLevel::Info, "discovery", "expired offline device from registry", Some(transition.key));
            }
            registry::RegistryTransitionKind::Refreshed => {}
        }
    }
    Ok(devices)
}

fn find_device(shared: &SharedState, key: &str) -> Result<DeviceSummary, String> {
    if let Some(info) = shared
        .runtime
        .lock()
        .ok()
        .and_then(|runtime| runtime.registry.get_info(key))
    {
        if info.presence == DevicePresence::Online {
            return Ok(info.summary);
        }
    }
    refresh_devices(shared)?;
    let info = shared
        .runtime
        .lock()
        .ok()
        .and_then(|runtime| runtime.registry.get_info(key))
        .ok_or_else(|| format!("device is no longer available: {key}"))?;
    if info.presence != DevicePresence::Online {
        return Err(format!("device is offline: {key}"));
    }
    Ok(info.summary)
}

fn apply_lighting_state(
    shared: &SharedState,
    device: &DeviceSummary,
    state: LightingState,
) -> Result<(), String> {
    with_hid_session(shared, device, |handle| reforge_hid::apply_lighting_with_handle(handle, device, &state))?;
    let mut runtime = shared
        .runtime
        .lock()
        .map_err(|_| "daemon lighting state lock is poisoned".to_owned())?;
    runtime
        .lighting_states
        .insert(device.key.clone(), state.clone());
    runtime
        .desired_states
        .entry(device.key.clone())
        .or_default()
        .lighting = Some(state.clone());
    let host_fallback = device
        .lighting
        .as_ref()
        .map(|caps| {
            caps.per_key_v2
                && matches!(
                    state.effect,
                    LightingEffect::Breathing
                        | LightingEffect::ColorCycle
                        | LightingEffect::Wave
                        | LightingEffect::Ripple
                )
                && !caps.supports_effect(state.effect)
        })
        .unwrap_or(false);
    if state.enabled && (state.effect.is_host_driven() || host_fallback) {
        runtime.host_effects.insert(
            device.key.clone(),
            ActiveHostEffect {
                device: device.clone(),
                state,
                started: Instant::now(),
            },
        );
    } else {
        runtime.host_effects.remove(&device.key);
    }
    Ok(())
}

fn apply_profile_to_device(
    shared: &SharedState,
    profile: &Profile,
    device: &DeviceSummary,
) -> Result<RpcData, String> {
    if !profile.matches(device) {
        return Err(format!(
            "profile '{}' does not match device '{}'",
            profile.name, device.product
        ));
    }
    let mut dpi_result = None;
    if let Some(dpi) = profile.dpi {
        dpi_result = Some(with_hid_session(shared, device, |handle| reforge_hid::set_dpi_with_handle(handle, device, dpi))?);
        if let Ok(mut runtime) = shared.runtime.lock() {
            runtime.desired_states.entry(device.key.clone()).or_default().dpi = Some(dpi);
        }
    }
    if let Some(lighting) = profile.lighting.clone() {
        apply_lighting_state(shared, device, lighting)?;
    }
    let mut profile_controls: Vec<_> = profile.controls.iter().collect();
    profile_controls.sort_by_key(|(control_id, _)| {
        device.controls.iter().find(|control| control.id == **control_id).map(|control| match control.group {
            ControlGroup::Profiles => 0,
            ControlGroup::Performance | ControlGroup::Assignments => 1,
            ControlGroup::Lighting => 2,
            ControlGroup::Audio | ControlGroup::Camera => 3,
            ControlGroup::Power => 4,
            _ => 5,
        }).unwrap_or(9)
    });
    for (control_id, value) in profile_controls {
        if device.controls.iter().any(|control| control.id == *control_id && control.writable && control.profile_eligible) {
            let applied = set_device_control(shared, device, control_id, value)?;
            if let Ok(mut runtime) = shared.runtime.lock() {
                runtime.desired_states.entry(device.key.clone()).or_default().controls.insert(control_id.clone(), applied);
            }
        }
    }
    if let Ok(mut runtime) = shared.runtime.lock() {
        runtime.desired_states.entry(device.key.clone()).or_default().profile_name = Some(profile.name.clone());
    }
    Ok(dpi_result.map(RpcData::Dpi).unwrap_or(RpcData::Ack))
}

fn get_device_telemetry(shared: &SharedState, key: &str) -> Result<DeviceTelemetry, String> {
    let info = shared
        .runtime
        .lock()
        .ok()
        .and_then(|runtime| runtime.registry.get_info(key))
        .ok_or_else(|| format!("unknown device: {key}"))?;
    let telemetry = if info.presence == DevicePresence::Online {
        let mut telemetry = providers::read_telemetry(&info.summary, info.presence, unix_ms());
        if info.summary.hidpp
            && info.summary.controls.iter().any(|control| control.id == "hidpp:dpi")
            && let Ok(value) = get_device_control(shared, &info.summary, "hidpp:dpi")
        {
            telemetry.active_dpi = value.as_i64().and_then(|value| u16::try_from(value).ok());
        }
        telemetry
    } else {
        shared
            .runtime
            .lock()
            .ok()
            .and_then(|runtime| runtime.telemetry.get(key).cloned())
            .unwrap_or_else(|| providers::read_telemetry(&info.summary, info.presence, unix_ms()))
    };
    if let Ok(mut runtime) = shared.runtime.lock() {
        runtime.telemetry.insert(key.to_owned(), telemetry.clone());
    }
    Ok(telemetry)
}

fn diagnostics_snapshot(shared: &SharedState) -> DiagnosticSnapshot {
    let integrations = integration_status(shared);
    let store = ProfileStore::load().unwrap_or_default();
    let (devices, recent_events, provider_health) = shared
        .runtime
        .lock()
        .map(|runtime| {
            let devices = runtime.registry.runtime_list();
            let events = runtime.diagnostics.snapshot();
            let mut health = BTreeMap::<ProviderKind, bool>::new();
            for device in &devices {
                let online = device.presence == DevicePresence::Online;
                for provider in &device.summary.providers {
                    health
                        .entry(*provider)
                        .and_modify(|value| *value |= online)
                        .or_insert(online);
                }
            }
            (devices, events, health)
        })
        .unwrap_or_default();
    let runtime_socket_category = if env::var_os("XDG_RUNTIME_DIR").is_some() {
        "$XDG_RUNTIME_DIR/reforge-logitech/reforge.sock"
    } else {
        "/tmp/reforge-logitech-$UID/reforge.sock"
    };
    DiagnosticSnapshot {
        package_version: env!("CARGO_PKG_VERSION").to_owned(),
        daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
        runtime_socket_category: runtime_socket_category.to_owned(),
        integrations,
        devices,
        provider_health,
        recent_events,
        profile_store_version: store.version,
        profile_count: store.profiles.len(),
    }
}

fn handle_request(request: RpcRequest, shared: &SharedState) -> Result<RpcData, String> {
    match request {
        RpcRequest::Health => Ok(RpcData::Health {
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }),
        RpcRequest::ListDevices => {
            let devices = shared
                .runtime
                .lock()
                .map_err(|_| "daemon registry lock is poisoned".to_owned())?
                .registry
                .online_summaries();
            Ok(RpcData::Devices(devices))
        }
        RpcRequest::RescanDevices => {
            refresh_devices(shared)?;
            let items = shared
                .runtime
                .lock()
                .map_err(|_| "daemon registry lock is poisoned".to_owned())?
                .registry
                .runtime_list();
            Ok(RpcData::DeviceRuntime(items))
        }
        RpcRequest::ListDeviceRuntime => {
            let items = shared
                .runtime
                .lock()
                .map_err(|_| "daemon registry lock is poisoned".to_owned())?
                .registry
                .runtime_list();
            Ok(RpcData::DeviceRuntime(items))
        }
        RpcRequest::GetDeviceTelemetry { key } => Ok(RpcData::Telemetry(get_device_telemetry(shared, &key)?)),
        RpcRequest::SetDpi { key, dpi } => {
            let device = find_device(shared, &key)?;
            let state = with_hid_session(shared, &device, |handle| reforge_hid::set_dpi_with_handle(handle, &device, dpi))?;
            if let Ok(mut runtime) = shared.runtime.lock() {
                runtime.desired_states.entry(key.clone()).or_default().dpi = Some(state.current);
                let mut updated = runtime.registry.get_summary(&key).unwrap_or(device.clone());
                updated.dpi = Some(state.clone());
                runtime.registry.update_summary(updated);
            }
            Ok(RpcData::Dpi(state))
        }
        RpcRequest::GetLighting { key } => {
            let device = find_device(shared, &key)?;
            let state = shared
                .runtime
                .lock()
                .map_err(|_| "daemon lighting state lock is poisoned".to_owned())?
                .lighting_states
                .get(&key)
                .cloned()
                .unwrap_or_else(|| {
                    let mut state = LightingState::default();
                    state.brightness = device
                        .lighting
                        .as_ref()
                        .and_then(|caps| caps.brightness.as_ref().map(|value| value.current));
                    state
                });
            Ok(RpcData::Lighting(state))
        }
        RpcRequest::SetLighting { key, state } => {
            let device = find_device(shared, &key)?;
            apply_lighting_state(shared, &device, state.clone())?;
            Ok(RpcData::Lighting(state))
        }
        RpcRequest::SetKeyColor { key, key_id, color } => {
            let device = find_device(shared, &key)?;
            let mut state = shared
                .runtime
                .lock()
                .map_err(|_| "daemon lighting state lock is poisoned".to_owned())?
                .lighting_states
                .get(&key)
                .cloned()
                .unwrap_or_default();
            state.enabled = true;
            state.effect = LightingEffect::Static;
            state.per_key.insert(key_id, color);
            apply_lighting_state(shared, &device, state.clone())?;
            Ok(RpcData::Lighting(state))
        }
        RpcRequest::GetControl { key, control_id } => {
            let device = find_device(shared, &key)?;
            Ok(RpcData::Control(get_device_control(shared, &device, &control_id)?))
        }
        RpcRequest::SetControl { key, control_id, value } => {
            let device = find_device(shared, &key)?;
            let applied = set_device_control(shared, &device, &control_id, &value)?;
            if let Ok(mut runtime) = shared.runtime.lock() {
                runtime
                    .desired_states
                    .entry(key.clone())
                    .or_default()
                    .controls
                    .insert(control_id.clone(), applied.clone());
                let mut updated = runtime.registry.get_summary(&key).unwrap_or(device.clone());
                if let Some(control) = updated.controls.iter_mut().find(|control| control.id == control_id) {
                    control.value = applied.clone();
                }
                runtime.registry.update_summary(updated);
            }
            Ok(RpcData::Control(applied))
        }
        RpcRequest::ListProfiles => Ok(RpcData::Profiles(ProfileStore::load()?.profiles)),
        RpcRequest::SaveProfile { profile } => {
            let mut store = ProfileStore::load()?;
            store.upsert(profile);
            store.save()?;
            Ok(RpcData::Ack)
        }
        RpcRequest::RenameProfile { old_name, new_name } => {
            let mut store = ProfileStore::load()?;
            store.rename(&old_name, &new_name)?;
            store.save()?;
            Ok(RpcData::Ack)
        }
        RpcRequest::DeleteProfile { name } => {
            let mut store = ProfileStore::load()?;
            store.remove(&name)?;
            store.save()?;
            Ok(RpcData::Ack)
        }
        RpcRequest::CloneProfile { source_name, new_name } => {
            let mut store = ProfileStore::load()?;
            store.clone_as(&source_name, &new_name)?;
            store.save()?;
            Ok(RpcData::Ack)
        }
        RpcRequest::ImportProfiles { json, replace } => {
            let mut store = ProfileStore::load()?;
            store.import_json(&json, replace)?;
            store.save()?;
            Ok(RpcData::Profiles(store.profiles))
        }
        RpcRequest::ExportProfiles { names } => {
            let store = ProfileStore::load()?;
            Ok(RpcData::ExportedProfiles { json: store.export_json(&names)? })
        }
        RpcRequest::ApplyProfile { profile_name, key } => {
            let store = ProfileStore::load()?;
            let profile = store
                .profiles
                .into_iter()
                .find(|profile| profile.name == profile_name)
                .ok_or_else(|| format!("profile does not exist: {profile_name}"))?;
            let device = find_device(shared, &key)?;
            let result = apply_profile_to_device(shared, &profile, &device)?;
            record_event(shared, DiagnosticLevel::Info, "profiles", format!("applied profile '{}'", profile.name), Some(key));
            Ok(result)
        }
        RpcRequest::GetDiagnostics => Ok(RpcData::Diagnostics(diagnostics_snapshot(shared))),
        RpcRequest::ClearDiagnostics => {
            if let Ok(mut runtime) = shared.runtime.lock() {
                runtime.diagnostics.clear();
            }
            Ok(RpcData::Ack)
        }
        RpcRequest::ListServices => {
            let items = shared
                .services
                .lock()
                .map_err(|_| "service registry lock is poisoned".to_owned())?
                .list();
            Ok(RpcData::Services(items))
        }
        RpcRequest::ReconnectService { service } => {
            match service {
                ServiceKind::Hidpp => {
                    let devices = shared
                        .runtime
                        .lock()
                        .map_err(|_| "daemon registry lock is poisoned".to_owned())?
                        .registry
                        .online_summaries();
                    let mut failures = Vec::new();
                    for device in devices.iter().filter(|device| device.hidpp) {
                        if let Err(error) = with_hid_io(|| shared.sessions.reconnect(device).map(|_| ())) {
                            failures.push(format!("{}: {error}", device.product));
                        }
                    }
                    if !failures.is_empty() {
                        return Err(failures.join("; "));
                    }
                    refresh_devices(shared)?;
                }
                ServiceKind::V4l2 | ServiceKind::Pipewire | ServiceKind::Udev => {
                    refresh_devices(shared)?;
                }
                ServiceKind::Fwupd => {
                    fwupd::probe_and_update_status(shared)?;
                }
            }
            Ok(RpcData::Ack)
        }
        RpcRequest::GetSessionStatus { key } => shared
            .sessions
            .status(&key)
            .map(RpcData::SessionStatus)
            .ok_or_else(|| format!("no persistent HID++ session for {key}")),
        RpcRequest::ListFirmwareDevices => Ok(RpcData::FirmwareDevices(fwupd::list_devices(shared)?)),
        RpcRequest::RefreshFirmwareMetadata => {
            fwupd::refresh_metadata(shared)?;
            Ok(RpcData::Ack)
        }
        RpcRequest::GetFirmwareReleases { device_id } => {
            Ok(RpcData::FirmwareReleases(fwupd::releases(shared, &device_id)?))
        }
        RpcRequest::InstallFirmwareUpdate { device_id } => {
            Ok(RpcData::FirmwareInstall(fwupd::install_update(shared, &device_id)?))
        }
        RpcRequest::IntegrationStatus => Ok(RpcData::Integrations(integration_status(shared))),
    }
}

fn serve_connection(mut stream: UnixStream, shared: SharedState) -> Result<(), String> {
    let mut line = String::new();
    BufReader::new(
        stream
            .try_clone()
            .map_err(|error| format!("failed to clone daemon socket: {error}"))?,
    )
    .read_line(&mut line)
    .map_err(|error| format!("failed to read daemon request: {error}"))?;

    let response = if line.trim().is_empty() {
        RpcResponse::error("empty RPC request")
    } else {
        match serde_json::from_str::<RpcRequest>(line.trim_end()) {
            Ok(request) => match handle_request(request, &shared) {
                Ok(data) => RpcResponse::ok(data),
                Err(error) => {
                    record_event(&shared, DiagnosticLevel::Error, "rpc", error.clone(), None);
                    RpcResponse::error(error)
                },
            },
            Err(error) => RpcResponse::error(format!("invalid RPC request: {error}")),
        }
    };

    let payload = serde_json::to_vec(&response)
        .map_err(|error| format!("failed to serialize daemon response: {error}"))?;
    stream
        .write_all(&payload)
        .and_then(|_| stream.write_all(b"\n"))
        .and_then(|_| stream.flush())
        .map_err(|error| format!("failed to write daemon response: {error}"))
}

fn prepare_socket(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid runtime socket path: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("failed to secure {}: {error}", parent.display()))?;
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("failed to remove stale {}: {error}", path.display()))?;
    }
    Ok(())
}

fn command_exists(name: &str) -> bool {
    env::var_os("PATH")
        .map(|paths| {
            env::split_paths(&paths).any(|path| {
                let candidate = path.join(name);
                candidate.is_file()
            })
        })
        .unwrap_or(false)
}

fn integration_status(shared: &SharedState) -> IntegrationStatus {
    let screen_capture = if command_exists("grim") {
        Some("grim (Wayland/wlroots)".to_owned())
    } else if command_exists("import") {
        Some("ImageMagick import (X11)".to_owned())
    } else {
        None
    };
    let audio_capture = command_exists("pw-cat").then(|| "PipeWire pw-cat input".to_owned());
    let active_auto_profile = shared
        .runtime
        .lock()
        .ok()
        .and_then(|state| state.active_auto_profile.clone());
    IntegrationStatus {
        screen_capture,
        audio_capture,
        process_watcher: Path::new("/proc").is_dir(),
        active_auto_profile,
    }
}

fn parse_ppm_average(data: &[u8]) -> Option<RgbColor> {
    let mut index = 0_usize;
    let mut tokens = Vec::new();
    while tokens.len() < 4 && index < data.len() {
        while index < data.len() && data[index].is_ascii_whitespace() {
            index += 1;
        }
        if index < data.len() && data[index] == b'#' {
            while index < data.len() && data[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        let start = index;
        while index < data.len() && !data[index].is_ascii_whitespace() {
            index += 1;
        }
        if start < index {
            tokens.push(std::str::from_utf8(&data[start..index]).ok()?.to_owned());
        }
    }
    if tokens.first().map(String::as_str) != Some("P6") {
        return None;
    }
    let width: usize = tokens.get(1)?.parse().ok()?;
    let height: usize = tokens.get(2)?.parse().ok()?;
    let max: usize = tokens.get(3)?.parse().ok()?;
    if max == 0 || max > 255 {
        return None;
    }
    if index < data.len() && data[index].is_ascii_whitespace() {
        index += 1;
    }
    let pixel_count = width.checked_mul(height)?;
    let needed = pixel_count.checked_mul(3)?;
    let pixels = data.get(index..index.checked_add(needed)?)?;
    if pixels.is_empty() {
        return None;
    }
    let stride = (pixel_count / 20_000).max(1);
    let mut r = 0_u64;
    let mut g = 0_u64;
    let mut b = 0_u64;
    let mut count = 0_u64;
    for pixel in pixels.chunks_exact(3).step_by(stride) {
        r += u64::from(pixel[0]);
        g += u64::from(pixel[1]);
        b += u64::from(pixel[2]);
        count += 1;
    }
    (count > 0).then(|| RgbColor::new((r / count) as u8, (g / count) as u8, (b / count) as u8))
}

fn sample_screen_color() -> Option<RgbColor> {
    if command_exists("grim")
        && let Ok(output) = Command::new("grim").args(["-t", "ppm", "-"]).output()
        && output.status.success()
        && let Some(color) = parse_ppm_average(&output.stdout)
    {
        return Some(color);
    }
    if command_exists("import")
        && let Ok(output) = Command::new("import")
            .args(["-window", "root", "ppm:-"])
            .output()
        && output.status.success()
    {
        return parse_ppm_average(&output.stdout);
    }
    None
}

fn sample_audio_level() -> Option<u8> {
    if !command_exists("pw-cat") {
        return None;
    }
    let output = Command::new("pw-cat")
        .args([
            "--record",
            "--raw",
            "--format=s16",
            "--rate=8000",
            "--channels=1",
            "--sample-count=400",
            "--properties={\"stream.capture.sink\":true}",
            "-",
        ])
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.len() < 2 {
        return None;
    }
    let mut peak = 0_i32;
    for sample in output.stdout.chunks_exact(2) {
        let value = i16::from_ne_bytes([sample[0], sample[1]]) as i32;
        peak = peak.max(value.abs());
    }
    Some(((peak * 255) / i32::from(i16::MAX)).clamp(0, 255) as u8)
}

fn effect_loop(shared: SharedState) {
    let mut sessions: HashMap<String, reforge_hid::HostFrameSession> = HashMap::new();
    loop {
        let active: Vec<ActiveHostEffect> = shared
            .runtime
            .lock()
            .map(|runtime| runtime.host_effects.values().cloned().collect())
            .unwrap_or_default();
        let active_keys: HashSet<String> = active
            .iter()
            .map(|effect| effect.device.key.clone())
            .collect();
        sessions.retain(|key, _| active_keys.contains(key));

        let need_screen = active
            .iter()
            .any(|effect| effect.state.effect == LightingEffect::ScreenReactive);
        let need_audio = active
            .iter()
            .any(|effect| effect.state.effect == LightingEffect::AudioReactive);
        let sampled_screen = need_screen.then(sample_screen_color).flatten();
        let sampled_audio = need_audio.then(sample_audio_level).flatten();

        for active_effect in active {
            let keys = active_effect
                .device
                .lighting
                .as_ref()
                .map(|caps| caps.supported_keys.clone())
                .filter(|keys| !keys.is_empty())
                .unwrap_or_else(|| vec![0]);
            let frame = render_host_frame(
                &active_effect.state,
                &keys,
                active_effect.started.elapsed().as_millis() as u64,
                sampled_screen,
                sampled_audio,
            );
            let key = active_effect.device.key.clone();
            if !sessions.contains_key(&key) {
                match with_hid_io(|| reforge_hid::HostFrameSession::open(&active_effect.device)) {
                    Ok(session) => {
                        sessions.insert(key.clone(), session);
                    }
                    Err(error) => {
                        eprintln!(
                            "ReForge host effect session error on {}: {error}",
                            active_effect.device.product
                        );
                        continue;
                    }
                }
            }
            let result = with_hid_io(|| {
                sessions
                    .get_mut(&key)
                    .ok_or_else(|| "host effect session disappeared".to_owned())
                    .and_then(|session| session.write_frame(&frame))
            });
            if let Err(error) = result {
                sessions.remove(&key);
                eprintln!(
                    "ReForge host effect error on {}: {error}",
                    active_effect.device.product
                );
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn hidpp_notification_loop(shared: SharedState) {
    loop {
        let notifications = with_hid_io(|| Ok(shared.sessions.poll_notifications())).unwrap_or_default();
        for notification in notifications {
            let device = shared
                .runtime
                .lock()
                .ok()
                .and_then(|runtime| runtime.registry.get_summary(&notification.key));
            let Some(device) = device else {
                continue;
            };
            let feature_id = device
                .features
                .iter()
                .find(|feature| feature.index == notification.feature_index)
                .map(|feature| feature.feature_id);
            match feature_id {
                Some(0x1000 | 0x1004) => {
                    if let Some(level) = notification.params.first().copied() {
                        if let Ok(mut runtime) = shared.runtime.lock() {
                            let telemetry = runtime.telemetry.entry(notification.key.clone()).or_default();
                            telemetry.key = notification.key.clone();
                            telemetry.presence = DevicePresence::Online;
                            telemetry.battery_percent = Some(level.min(100));
                            telemetry.updated_unix_ms = unix_ms();
                        }
                    }
                }
                Some(0x0601) => {
                    if let Some(muted) = notification.params.first().copied() {
                        if let Ok(mut runtime) = shared.runtime.lock()
                            && let Some(mut summary) = runtime.registry.get_summary(&notification.key)
                        {
                            if let Some(control) = summary.controls.iter_mut().find(|control| control.backend_id.starts_with("headset_mic_mute:")) {
                                control.value = reforge_core::ControlValue::Bool(muted != 0);
                            }
                            runtime.registry.update_summary(summary);
                        }
                    }
                }
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn running_processes() -> HashSet<String> {
    let mut names = HashSet::new();
    let Ok(entries) = fs::read_dir("/proc") else {
        return names;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|name| name.parse::<u32>().ok()) else {
            continue;
        };
        let base = Path::new("/proc").join(pid.to_string());
        if let Ok(comm) = fs::read_to_string(base.join("comm")) {
            let name = comm.trim().to_ascii_lowercase();
            if !name.is_empty() {
                names.insert(name);
            }
        }
        if let Ok(exe) = fs::read_link(base.join("exe"))
            && let Some(name) = exe.file_name().and_then(|name| name.to_str())
        {
            names.insert(name.to_ascii_lowercase());
        }
    }
    names
}

fn auto_profile_loop(shared: SharedState) {
    loop {
        let processes = running_processes();
        if let Ok(store) = ProfileStore::load() {
            let matching = store
                .profiles
                .iter()
                .find(|profile| profile.matches_processes(processes.iter().map(String::as_str)))
                .cloned();
            let current = shared
                .runtime
                .lock()
                .ok()
                .and_then(|runtime| runtime.active_auto_profile.clone());
            if matching.as_ref().map(|profile| &profile.name) != current.as_ref() {
                if let Some(profile) = matching {
                    let devices = shared
                        .runtime
                        .lock()
                        .map(|runtime| runtime.registry.online_summaries())
                        .unwrap_or_default();
                    for device in devices.iter().filter(|device| profile.matches(device)) {
                        if let Err(error) = apply_profile_to_device(&shared, &profile, device) {
                            record_event(&shared, DiagnosticLevel::Error, "profiles", format!("auto profile '{}' failed: {error}", profile.name), Some(device.key.clone()));
                        }
                    }
                    record_event(&shared, DiagnosticLevel::Info, "profiles", format!("auto profile '{}' activated", profile.name), None);
                    if let Ok(mut runtime) = shared.runtime.lock() {
                        runtime.active_auto_profile = Some(profile.name);
                    }
                } else if let Ok(mut runtime) = shared.runtime.lock() {
                    runtime.active_auto_profile = None;
                }
            }
        }
        thread::sleep(Duration::from_secs(2));
    }
}

fn discovery_loop(shared: SharedState) {
    loop {
        if let Err(error) = refresh_devices(&shared) {
            record_event(&shared, DiagnosticLevel::Error, "discovery", error, None);
        }
        thread::sleep(Duration::from_secs(10));
    }
}

fn run() -> Result<(), String> {
    let socket = paths::runtime_socket();
    prepare_socket(&socket)?;
    let listener = UnixListener::bind(&socket)
        .map_err(|error| format!("failed to bind {}: {error}", socket.display()))?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("failed to secure {}: {error}", socket.display()))?;

    let shared = Arc::new(AppState::default());
    refresh_devices(&shared)?;
    let _ = fwupd::probe_and_update_status(&shared);
    {
        let state = Arc::clone(&shared);
        thread::spawn(move || discovery_loop(state));
    }
    {
        let state = Arc::clone(&shared);
        thread::spawn(move || udev_monitor::monitor_loop(state));
    }
    {
        let state = Arc::clone(&shared);
        thread::spawn(move || hidpp_notification_loop(state));
    }
    {
        let state = Arc::clone(&shared);
        thread::spawn(move || effect_loop(state));
    }
    {
        let state = Arc::clone(&shared);
        thread::spawn(move || auto_profile_loop(state));
    }

    eprintln!("ReForge Logitech daemon listening on {}", socket.display());
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let state = Arc::clone(&shared);
                thread::spawn(move || {
                    if let Err(error) = serve_connection(stream, state) {
                        eprintln!("ReForge request error: {error}");
                    }
                });
            }
            Err(error) => eprintln!("ReForge accept error: {error}"),
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("reforge-logitechd: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_restore_group_order_is_stable() {
        let mut groups = vec![
            ControlGroup::Power,
            ControlGroup::Camera,
            ControlGroup::Lighting,
            ControlGroup::Assignments,
            ControlGroup::Profiles,
        ];
        groups.sort_by_key(|group| restore_group_rank(*group));
        assert_eq!(
            groups,
            vec![
                ControlGroup::Profiles,
                ControlGroup::Assignments,
                ControlGroup::Lighting,
                ControlGroup::Camera,
                ControlGroup::Power,
            ]
        );
    }

    #[test]
    fn ppm_average_reads_binary_p6_pixels() {
        let mut ppm = b"P6\n2 1\n255\n".to_vec();
        ppm.extend_from_slice(&[255, 0, 0, 0, 0, 255]);
        assert_eq!(parse_ppm_average(&ppm), Some(RgbColor::new(127, 0, 127)));
    }

    #[test]
    fn absent_capture_commands_do_not_disable_process_watcher_model() {
        let shared = Arc::new(AppState::default());
        let status = integration_status(&shared);
        assert_eq!(status.process_watcher, Path::new("/proc").is_dir());
    }
}
