use crate::{diagnostics as diagnostic_ui, firmware as firmware_ui, keyboard, services as service_ui};
use eframe::egui;
use reforge_core::{
    client, paths, ControlGroup, ControlKind, ControlValue,
    DevicePresence, DeviceRuntimeInfo, DeviceSummary, DeviceTelemetry, DiagnosticSnapshot,
    FirmwareDevice, FirmwareRelease, HidppSessionStatus, IntegrationStatus, LightingEffect,
    LightingState, Profile, RgbColor, RpcData, RpcRequest, RpcResponse, ServiceKind, ServiceState,
    ServiceStatus, UiPreferences,
};
use std::{
    collections::BTreeMap,
    fs,
    process::Command,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    DeviceControls,
    Lighting,
    Profiles,
    Integrations,
    Services,
    Firmware,
    Diagnostics,
    Settings,
}

impl Page {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dashboard => "dashboard",
            Self::DeviceControls => "device_controls",
            Self::Lighting => "lighting",
            Self::Profiles => "profiles",
            Self::Integrations => "integrations",
            Self::Services => "services",
            Self::Firmware => "firmware",
            Self::Diagnostics => "diagnostics",
            Self::Settings => "settings",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "device_controls" => Self::DeviceControls,
            "lighting" => Self::Lighting,
            "profiles" => Self::Profiles,
            "integrations" => Self::Integrations,
            "services" => Self::Services,
            "firmware" => Self::Firmware,
            "diagnostics" => Self::Diagnostics,
            "settings" => Self::Settings,
            _ => Self::Dashboard,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ThemeChoice {
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    fn preference(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Light => egui::ThemePreference::Light,
            Self::Dark => egui::ThemePreference::Dark,
        }
    }
}

pub struct ReForgeApp {
    devices: Vec<DeviceSummary>,
    runtime_devices: Vec<DeviceRuntimeInfo>,
    telemetry: Option<DeviceTelemetry>,
    profiles: Vec<Profile>,
    integrations: IntegrationStatus,
    services: Vec<ServiceStatus>,
    session_status: Option<HidppSessionStatus>,
    firmware_devices: Vec<FirmwareDevice>,
    firmware_releases: BTreeMap<String, Vec<FirmwareRelease>>,
    selected_firmware_id: Option<String>,
    pending_firmware_install: Option<String>,
    diagnostics: Option<DiagnosticSnapshot>,
    selected_device_key: Option<String>,
    selected_profile_name: Option<String>,
    page: Page,
    dpi_value: u16,
    lighting: LightingState,
    profile_name: String,
    profile_apps: String,
    profile_auto: bool,
    profile_action_name: String,
    confirm_delete_profile: Option<String>,
    brush: RgbColor,
    theme: ThemeChoice,
    live_preview: bool,
    include_device_identifiers: bool,
    last_preview: Instant,
    last_runtime_poll: Instant,
    last_service_poll: Instant,
    status: String,
}

impl ReForgeApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let (preferences, preference_error) = UiPreferences::load_or_default();
        let theme = ThemeChoice::from_str(&preferences.theme);
        ctx.set_theme(theme.preference());
        let mut app = Self {
            devices: Vec::new(),
            runtime_devices: Vec::new(),
            telemetry: None,
            profiles: Vec::new(),
            integrations: IntegrationStatus::default(),
            services: Vec::new(),
            session_status: None,
            firmware_devices: Vec::new(),
            firmware_releases: BTreeMap::new(),
            selected_firmware_id: None,
            pending_firmware_install: None,
            diagnostics: None,
            selected_device_key: preferences.selected_device_key,
            selected_profile_name: preferences.selected_profile_name,
            page: Page::from_str(&preferences.page),
            dpi_value: 800,
            lighting: LightingState::default(),
            profile_name: "Gaming".to_owned(),
            profile_apps: String::new(),
            profile_auto: false,
            profile_action_name: String::new(),
            confirm_delete_profile: None,
            brush: RgbColor::new(0x00, 0x9d, 0xff),
            theme,
            live_preview: preferences.live_preview,
            include_device_identifiers: false,
            last_preview: Instant::now() - Duration::from_secs(1),
            last_runtime_poll: Instant::now() - Duration::from_secs(2),
            last_service_poll: Instant::now() - Duration::from_secs(5),
            status: preference_error.unwrap_or_else(|| "Starting ReForge Control Center…".to_owned()),
        };
        app.refresh();
        app
    }

    fn rpc(request: RpcRequest) -> Result<RpcData, String> {
        match client::call(&request).map_err(|error| {
            format!(
                "{error}. Start the daemon with: systemctl --user start reforge-logitechd.service"
            )
        })? {
            RpcResponse::Ok { data } => Ok(data),
            RpcResponse::Err { message } => Err(message),
        }
    }

    fn save_preferences(&mut self) {
        let prefs = UiPreferences {
            selected_device_key: self.selected_device_key.clone(),
            selected_profile_name: self.selected_profile_name.clone(),
            page: self.page.as_str().into(),
            theme: self.theme.as_str().into(),
            live_preview: self.live_preview,
            window_width: None,
            window_height: None,
        };
        if let Err(error) = prefs.save() {
            self.status = error;
        }
    }

    fn refresh_profiles(&mut self) {
        if let Ok(RpcData::Profiles(profiles)) = Self::rpc(RpcRequest::ListProfiles) {
            self.profiles = profiles;
            if self
                .selected_profile_name
                .as_ref()
                .is_none_or(|name| !self.profiles.iter().any(|profile| &profile.name == name))
            {
                self.selected_profile_name = self.profiles.first().map(|profile| profile.name.clone());
            }
        }
    }

    fn refresh_runtime(&mut self) {
        match Self::rpc(RpcRequest::ListDeviceRuntime) {
            Ok(RpcData::DeviceRuntime(runtime)) => {
                self.runtime_devices = runtime;
                self.devices = self
                    .runtime_devices
                    .iter()
                    .map(|runtime| runtime.summary.clone())
                    .collect();
                if self.devices.is_empty() {
                    self.selected_device_key = None;
                    self.telemetry = None;
                    self.status = "No Logitech devices detected.".to_owned();
                    return;
                }
                if self
                    .selected_device_key
                    .as_ref()
                    .is_none_or(|key| !self.devices.iter().any(|device| &device.key == key))
                {
                    self.selected_device_key = self
                        .runtime_devices
                        .iter()
                        .find(|item| item.presence == DevicePresence::Online)
                        .or_else(|| self.runtime_devices.first())
                        .map(|item| item.summary.key.clone());
                }
                if let Some(device) = self.selected_device()
                    && let Some(dpi) = device.dpi.as_ref()
                {
                    self.dpi_value = dpi.current;
                }
                self.refresh_selected_telemetry();
                let online = self
                    .runtime_devices
                    .iter()
                    .filter(|item| item.presence == DevicePresence::Online)
                    .count();
                let offline = self.runtime_devices.len().saturating_sub(online);
                self.status = format!("{online} online • {offline} offline/reconnecting Logitech device(s).");
            }
            Ok(_) => self.status = "Unexpected runtime-device response from daemon.".to_owned(),
            Err(error) => self.status = error,
        }
    }

    fn refresh_services(&mut self) {
        match Self::rpc(RpcRequest::ListServices) {
            Ok(RpcData::Services(items)) => self.services = items,
            Ok(_) => self.status = "Unexpected service-list response from daemon.".into(),
            Err(error) => self.status = error,
        }
        self.refresh_selected_session();
        self.last_service_poll = Instant::now();
    }

    fn refresh_selected_session(&mut self) {
        let Some(device) = self.selected_device() else {
            self.session_status = None;
            return;
        };
        if !device.hidpp || !self.selected_online() {
            self.session_status = None;
            return;
        }
        self.session_status = match Self::rpc(RpcRequest::GetSessionStatus { key: device.key }) {
            Ok(RpcData::SessionStatus(status)) => Some(status),
            _ => None,
        };
    }

    fn refresh_firmware_devices(&mut self) {
        match Self::rpc(RpcRequest::ListFirmwareDevices) {
            Ok(RpcData::FirmwareDevices(devices)) => {
                self.firmware_devices = devices;
                if self
                    .selected_firmware_id
                    .as_ref()
                    .is_none_or(|id| !self.firmware_devices.iter().any(|device| &device.id == id))
                {
                    self.selected_firmware_id = self.firmware_devices.first().map(|device| device.id.clone());
                }
            }
            Ok(_) => self.status = "Unexpected firmware-device response from daemon.".into(),
            Err(error) => self.status = error,
        }
        self.refresh_services();
    }

    fn refresh(&mut self) {
        let _ = Self::rpc(RpcRequest::RescanDevices);
        self.refresh_runtime();
        self.refresh_profiles();
        if let Ok(RpcData::Integrations(status)) = Self::rpc(RpcRequest::IntegrationStatus) {
            self.integrations = status;
        }
        self.load_selected_lighting();
        self.refresh_services();
        self.last_runtime_poll = Instant::now();
    }

    fn poll_runtime(&mut self) {
        if self.last_runtime_poll.elapsed() >= Duration::from_secs(1) {
            self.last_runtime_poll = Instant::now();
            self.refresh_runtime();
        }
        if self.last_service_poll.elapsed() >= Duration::from_secs(3) {
            self.refresh_services();
        }
    }

    fn selected_device(&self) -> Option<DeviceSummary> {
        let key = self.selected_device_key.as_deref()?;
        self.devices.iter().find(|device| device.key == key).cloned()
    }

    fn selected_presence(&self) -> Option<DevicePresence> {
        let key = self.selected_device_key.as_deref()?;
        self.runtime_devices
            .iter()
            .find(|runtime| runtime.summary.key == key)
            .map(|runtime| runtime.presence)
    }

    fn selected_online(&self) -> bool {
        self.selected_presence() == Some(DevicePresence::Online)
    }

    fn select_device(&mut self, index: usize) {
        self.selected_device_key = self.devices.get(index).map(|device| device.key.clone());
        if let Some(dpi) = self.devices.get(index).and_then(|device| device.dpi.as_ref()) {
            self.dpi_value = dpi.current;
        }
        self.refresh_selected_telemetry();
        self.refresh_selected_session();
        self.load_selected_lighting();
        self.save_preferences();
    }

    fn refresh_selected_telemetry(&mut self) {
        let Some(key) = self.selected_device_key.clone() else {
            self.telemetry = None;
            return;
        };
        if let Ok(RpcData::Telemetry(value)) = Self::rpc(RpcRequest::GetDeviceTelemetry { key }) {
            if let Some(dpi) = value.active_dpi {
                self.dpi_value = dpi;
            }
            self.telemetry = Some(value);
        }
    }

    fn load_selected_lighting(&mut self) {
        let Some(device) = self.selected_device() else {
            return;
        };
        if !self.selected_online() {
            return;
        }
        if let Ok(RpcData::Lighting(state)) = Self::rpc(RpcRequest::GetLighting { key: device.key }) {
            self.brush = state.primary;
            self.lighting = state;
        }
    }

    fn apply_dpi(&mut self) {
        let Some(device) = self.selected_device() else {
            self.status = "Select a device first.".to_owned();
            return;
        };
        if !self.selected_online() {
            self.status = "Device is offline; DPI changes are not queued.".to_owned();
            return;
        }
        let requested = device
            .dpi
            .as_ref()
            .and_then(|info| {
                info.supported
                    .iter()
                    .min_by_key(|&&value| value.abs_diff(self.dpi_value))
                    .copied()
            })
            .unwrap_or(self.dpi_value);
        match Self::rpc(RpcRequest::SetDpi {
            key: device.key.clone(),
            dpi: requested,
        }) {
            Ok(RpcData::Dpi(state)) => {
                self.dpi_value = state.current;
                if let Some(cached) = self.devices.iter_mut().find(|item| item.key == device.key) {
                    cached.dpi = Some(state);
                }
                self.status = format!("DPI applied and verified at {}.", self.dpi_value);
            }
            Ok(_) => self.status = "Unexpected DPI response from daemon.".to_owned(),
            Err(error) => self.status = error,
        }
    }

    fn apply_lighting(&mut self) {
        let Some(device) = self.selected_device() else {
            self.status = "Select a device first.".to_owned();
            return;
        };
        if !self.selected_online() {
            self.status = "Device is offline; lighting changes are not queued.".to_owned();
            return;
        }
        match Self::rpc(RpcRequest::SetLighting {
            key: device.key,
            state: self.lighting.clone(),
        }) {
            Ok(RpcData::Lighting(state)) => {
                self.lighting = state;
                self.status = "Lighting applied. Host effects continue in the daemon.".to_owned();
            }
            Ok(_) => self.status = "Unexpected lighting response from daemon.".to_owned(),
            Err(error) => self.status = error,
        }
    }

    fn save_profile(&mut self) {
        let Some(device) = self.selected_device() else {
            self.status = "Select a device before saving a profile.".to_owned();
            return;
        };
        let name = self.profile_name.trim().to_owned();
        if name.is_empty() {
            self.status = "Profile name cannot be empty.".to_owned();
            return;
        }
        let applications = self
            .profile_apps
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        let profile = Profile {
            name: name.clone(),
            product_id: (device.product_id != 0).then_some(device.product_id),
            product: Some(device.product.clone()),
            serial: device.serial.clone(),
            dpi: device.dpi.as_ref().map(|_| self.dpi_value),
            lighting: device.lighting.as_ref().map(|_| self.lighting.clone()),
            applications,
            auto_switch: self.profile_auto,
            controls: device.controls.iter().filter(|control| control.writable && control.profile_eligible).map(|control| (control.id.clone(), control.value.clone())).collect::<BTreeMap<_, _>>(),
        };
        match Self::rpc(RpcRequest::SaveProfile { profile }) {
            Ok(RpcData::Ack) => {
                self.status = format!("Saved profile '{name}'.");
                if let Ok(RpcData::Profiles(profiles)) = Self::rpc(RpcRequest::ListProfiles) {
                    self.profiles = profiles;
                }
            }
            Ok(_) => self.status = "Unexpected profile-save response from daemon.".to_owned(),
            Err(error) => self.status = error,
        }
    }

    fn apply_profile(&mut self, profile: Profile) {
        let Some(device) = self.selected_device() else {
            self.status = "Select a device first.".to_owned();
            return;
        };
        if !self.selected_online() {
            self.status = "Device is offline; profile application is not queued.".to_owned();
            return;
        }
        match Self::rpc(RpcRequest::ApplyProfile {
            profile_name: profile.name.clone(),
            key: device.key,
        }) {
            Ok(_) => {
                self.status = format!("Applied '{}'.", profile.name);
                self.refresh();
            }
            Err(error) => self.status = error,
        }
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        ui.heading("ReForge");
        ui.small("LOGITECH CONTROL CENTER");
        ui.add_space(12.0);
        for (page, label) in [
            (Page::Dashboard, "Dashboard"),
            (Page::DeviceControls, "Device Controls"),
            (Page::Lighting, "Lighting"),
            (Page::Profiles, "Profiles"),
            (Page::Integrations, "Integrations"),
            (Page::Services, "Services"),
            (Page::Firmware, "Firmware"),
            (Page::Diagnostics, "Diagnostics"),
            (Page::Settings, "Settings"),
        ] {
            if ui.selectable_label(self.page == page, label).clicked() {
                self.page = page;
                self.save_preferences();
            }
        }
        ui.add_space(18.0);
        ui.separator();
        ui.small("DEVICES");
        let devices: Vec<_> = self
            .runtime_devices
            .iter()
            .enumerate()
            .map(|(index, runtime)| {
                let badge = match runtime.presence {
                    DevicePresence::Online => "●",
                    DevicePresence::Offline => "○",
                    DevicePresence::Reconnecting => "↻",
                };
                (index, runtime.summary.key.clone(), format!("{badge} {}", runtime.summary.product))
            })
            .collect();
        for (index, key, name) in devices {
            if ui
                .selectable_label(self.selected_device_key.as_deref() == Some(key.as_str()), name)
                .clicked()
            {
                self.select_device(index);
            }
        }
        ui.add_space(12.0);
        if ui.button("Rescan devices").clicked() {
            self.refresh();
        }
    }

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Device Dashboard");
        ui.label("Stable device identity, live telemetry, and automatic reconnect restoration.");
        ui.add_space(12.0);
        if self.runtime_devices.is_empty() {
            ui.label("No Logitech devices detected.");
            return;
        }
        let cards = self.runtime_devices.clone();
        for (index, runtime) in cards.into_iter().enumerate() {
            let device = runtime.summary;
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.strong(&device.product);
                        ui.small(format!(
                            "{:?} • PID {:04X} • {:?}",
                            runtime.presence, device.product_id, device.transport
                        ));
                        if !device.providers.is_empty() {
                            ui.small(format!(
                                "Providers: {}",
                                device.providers.iter().map(|p| format!("{p:?}")).collect::<Vec<_>>().join(" + ")
                            ));
                        }
                        if runtime.presence != DevicePresence::Online {
                            ui.small(format!("Last seen: {} • reconnects: {}", runtime.last_seen_unix_ms, runtime.reconnect_count));
                        }
                    });
                    if ui.button("Configure").clicked() {
                        self.select_device(index);
                        self.page = if !device.controls.is_empty() {
                            Page::DeviceControls
                        } else if device.lighting.is_some() {
                            Page::Lighting
                        } else {
                            Page::Profiles
                        };
                        self.save_preferences();
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    if self.selected_device_key.as_deref() == Some(device.key.as_str()) {
                        if let Some(telemetry) = &self.telemetry {
                            if let Some(battery) = telemetry.battery_percent {
                                ui.label(format!("Battery: {battery}%"));
                            }
                            if let Some(dpi) = telemetry.active_dpi {
                                ui.label(format!("Pointer: {dpi} DPI"));
                            }
                        }
                    } else if let Some(dpi) = &device.dpi {
                        ui.label(format!("Pointer: {} DPI", dpi.current));
                    }
                    if let Some(caps) = &device.lighting {
                        if caps.per_key_v2 {
                            ui.label(format!("Per-key RGB: {} zones", caps.supported_keys.len()));
                        } else if !caps.zones.is_empty() {
                            ui.label(format!("Lighting: {} zone(s)", caps.zones.len()));
                        }
                        if caps.backlight_v2 && !caps.color_led_effects && !caps.rgb_effects {
                            ui.label("Keyboard backlight");
                        }
                    }
                });
            });
            ui.add_space(8.0);
        }
    }

    fn set_device_control(&mut self, control_id: String, value: ControlValue) {
        let Some(device) = self.selected_device() else {
            self.status = "Select a device first.".to_owned();
            return;
        };
        if !self.selected_online() {
            self.status = "Device is offline; control changes are not queued.".to_owned();
            return;
        }
        match Self::rpc(RpcRequest::SetControl {
            key: device.key.clone(),
            control_id: control_id.clone(),
            value: value.clone(),
        }) {
            Ok(RpcData::Control(applied)) => {
                if let Some(control) = self
                    .devices
                    .iter_mut()
                    .find(|item| item.key == device.key)
                    .and_then(|item| item.controls.iter_mut().find(|control| control.id == control_id))
                {
                    control.value = applied;
                    self.status = format!("Applied {}.", control.label);
                } else {
                    self.status = "Device control applied.".to_owned();
                }
            }
            Ok(RpcData::Ack) => {
                self.status = "Device control applied.".to_owned();
            }
            Ok(_) => self.status = "Unexpected device-control response from daemon.".to_owned(),
            Err(error) => self.status = error,
        }
    }

    fn control_group_name(group: ControlGroup) -> &'static str {
        match group {
            ControlGroup::Overview => "Overview",
            ControlGroup::Assignments => "Assignments",
            ControlGroup::Performance => "Performance",
            ControlGroup::Lighting => "Lighting",
            ControlGroup::Audio => "Audio",
            ControlGroup::Camera => "Camera",
            ControlGroup::Power => "Power",
            ControlGroup::Profiles => "Profiles",
            ControlGroup::Receiver => "Receiver",
            ControlGroup::Other => "Other",
        }
    }

    fn render_control(&mut self, ui: &mut egui::Ui, control: &reforge_core::DeviceControl) {
        let writable = control.writable && self.selected_online();
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.strong(&control.label);
                    ui.small(format!("{:?} • {}", control.provider, control.endpoint));
                });
                if !control.writable {
                    ui.label("Read only");
                } else if !writable {
                    ui.label("Offline");
                }
            });
            let mut requested: Option<ControlValue> = None;
            match control.kind {
                ControlKind::Toggle => {
                    let mut enabled = control.value.as_bool().unwrap_or(false);
                    let label = if enabled { "On" } else { "Off" };
                    if ui
                        .add_enabled(writable, egui::Checkbox::new(&mut enabled, label))
                        .changed()
                    {
                        requested = Some(ControlValue::Bool(enabled));
                    }
                }
                ControlKind::Range => {
                    let mut value = control.value.as_i64().unwrap_or(control.min.unwrap_or(0));
                    let min = control.min.unwrap_or(0);
                    let max = control.max.unwrap_or(100);
                    let mut slider = egui::Slider::new(&mut value, min..=max).text("");
                    if let Some(step) = control.step.filter(|step| *step > 0) {
                        slider = slider.step_by(step as f64);
                    }
                    if ui.add_enabled(writable, slider).changed() {
                        requested = Some(ControlValue::Int(value));
                    }
                }
                ControlKind::Choice => {
                    let mut value = control.value.as_i64().unwrap_or(0);
                    let selected = control
                        .choices
                        .iter()
                        .find(|choice| choice.value == value)
                        .map(|choice| choice.label.as_str())
                        .unwrap_or("Unknown");
                    ui.add_enabled_ui(writable, |ui| {
                        egui::ComboBox::from_label(format!("{} value", control.label))
                            .selected_text(selected)
                            .show_ui(ui, |ui| {
                                for choice in &control.choices {
                                    ui.selectable_value(&mut value, choice.value, &choice.label);
                                }
                            });
                    });
                    if value != control.value.as_i64().unwrap_or(0) && writable {
                        requested = Some(ControlValue::Int(value));
                    }
                }
                ControlKind::Action => {
                    if ui
                        .add_enabled(writable, egui::Button::new("Run"))
                        .clicked()
                    {
                        requested = Some(ControlValue::None);
                    }
                }
                ControlKind::Vector => {
                    let mut values = control
                        .value
                        .as_vector()
                        .map(ToOwned::to_owned)
                        .unwrap_or_default();
                    if values.is_empty() {
                        ui.label("No vector data available.");
                    } else {
                        let min = control.min.unwrap_or(-12);
                        let max = control.max.unwrap_or(12);
                        let step = control.step.unwrap_or(1).max(1) as f64;
                        let mut changed = false;
                        for (index, value) in values.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("Band {}", index + 1));
                                changed |= ui
                                    .add_enabled(
                                        writable,
                                        egui::Slider::new(value, min..=max).step_by(step),
                                    )
                                    .changed();
                            });
                        }
                        if changed {
                            requested = Some(ControlValue::Vector(values));
                        }
                    }
                }
                ControlKind::Status => match &control.value {
                    ControlValue::Bool(value) => { ui.label(if *value { "On" } else { "Off" }); }
                    ControlValue::Int(value) => { ui.label(value.to_string()); }
                    ControlValue::Vector(values) => { ui.label(format!("{:?}", values)); }
                    ControlValue::None => { ui.label("Unavailable"); }
                },
            }
            if let Some(value) = requested {
                self.set_device_control(control.id.clone(), value);
            }
        });
        ui.add_space(6.0);
    }

    fn device_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Device Controls");
        let Some(device) = self.selected_device() else {
            ui.label("Select a device.");
            return;
        };
        ui.label(format!("{} • {:?}", device.product, device.device_class));
        ui.small(format!(
            "Active providers: {}",
            device
                .providers
                .iter()
                .map(|p| format!("{p:?}"))
                .collect::<Vec<_>>()
                .join(" + ")
        ));
        ui.add_space(10.0);
        if let Some(dpi) = device.dpi.as_ref() {
            ui.group(|ui| {
                ui.strong("Pointer DPI");
                let mut value = self.dpi_value;
                if !dpi.supported.is_empty() {
                    egui::ComboBox::from_label("DPI")
                        .selected_text(format!("{} DPI", value))
                        .show_ui(ui, |ui| {
                            for supported in &dpi.supported {
                                ui.selectable_value(
                                    &mut value,
                                    *supported,
                                    format!("{} DPI", supported),
                                );
                            }
                        });
                } else {
                    ui.add(
                        egui::Slider::new(&mut value, dpi.min..=dpi.max)
                            .step_by(dpi.step.max(1) as f64)
                            .text("DPI"),
                    );
                }
                if value != self.dpi_value {
                    self.dpi_value = value;
                }
                if ui.add_enabled(self.selected_online(), egui::Button::new("Apply DPI")).clicked() {
                    self.apply_dpi();
                }
            });
            ui.add_space(6.0);
        }
        if device.controls.is_empty() && device.dpi.is_none() {
            ui.label("This device is detected, but ReForge does not have a verified writable control adapter for it yet.");
            return;
        }
        let controls = device.controls.clone();
        let mut last_group = None;
        for control in controls {
            if last_group != Some(control.group) {
                if last_group.is_some() {
                    ui.add_space(8.0);
                }
                ui.heading(Self::control_group_name(control.group));
                last_group = Some(control.group);
            }
            self.render_control(ui, &control);
        }
    }

    fn effect_name(effect: LightingEffect) -> &'static str {
        match effect {
            LightingEffect::Off => "Off",
            LightingEffect::Static => "Static",
            LightingEffect::Breathing => "Breathing",
            LightingEffect::ColorCycle => "Color Cycle",
            LightingEffect::Wave => "Wave",
            LightingEffect::Ripple => "Ripple",
            LightingEffect::Gradient => "Gradient",
            LightingEffect::ScreenReactive => "Screen Reactive",
            LightingEffect::AudioReactive => "Audio Reactive",
        }
    }

    fn lighting(&mut self, ui: &mut egui::Ui) {
        ui.heading("Lighting Studio");
        let Some(device) = self.selected_device() else {
            ui.label("Select a device.");
            return;
        };
        let Some(caps) = device.lighting.clone() else {
            ui.label("This device does not expose a supported HID++ lighting feature.");
            return;
        };
        ui.label(&device.product);
        ui.add_space(8.0);

        let mut apply_live = false;
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Effect")
                .selected_text(Self::effect_name(self.lighting.effect))
                .show_ui(ui, |ui| {
                    for effect in [
                        LightingEffect::Off,
                        LightingEffect::Static,
                        LightingEffect::Breathing,
                        LightingEffect::ColorCycle,
                        LightingEffect::Wave,
                        LightingEffect::Ripple,
                        LightingEffect::Gradient,
                        LightingEffect::ScreenReactive,
                        LightingEffect::AudioReactive,
                    ] {
                        let native_or_host = caps.supports_effect(effect)
                            || (caps.per_key_v2
                                && matches!(
                                    effect,
                                    LightingEffect::Breathing
                                        | LightingEffect::ColorCycle
                                        | LightingEffect::Wave
                                        | LightingEffect::Ripple
                                ));
                        let host_available = caps.per_key_v2 || !caps.zones.is_empty();
                        let enabled = (matches!(effect, LightingEffect::Off | LightingEffect::Static)
                            && caps.supports_effect(effect))
                            || (effect.is_host_driven() && host_available)
                            || native_or_host;
                        let response = if enabled {
                            ui.selectable_label(
                                self.lighting.effect == effect,
                                Self::effect_name(effect),
                            )
                        } else {
                            ui.add_enabled(false, egui::Button::new(Self::effect_name(effect)))
                        };
                        if response.clicked() {
                            self.lighting.effect = effect;
                            self.lighting.enabled = effect != LightingEffect::Off;
                            apply_live = true;
                        }
                    }
                });
            ui.checkbox(&mut self.live_preview, "Live preview");
        });

        let has_color = caps.color_led_effects || caps.rgb_effects || caps.per_key_v2;
        if has_color {
            ui.horizontal(|ui| {
                let mut primary = [self.lighting.primary.r, self.lighting.primary.g, self.lighting.primary.b];
                ui.label("Primary");
                if ui.color_edit_button_srgb(&mut primary).changed() {
                    self.lighting.primary = RgbColor::new(primary[0], primary[1], primary[2]);
                    apply_live = true;
                }
                let mut secondary = [self.lighting.secondary.r, self.lighting.secondary.g, self.lighting.secondary.b];
                ui.label("Secondary");
                if ui.color_edit_button_srgb(&mut secondary).changed() {
                    self.lighting.secondary = RgbColor::new(secondary[0], secondary[1], secondary[2]);
                    apply_live = true;
                }
            });
        } else if caps.backlight_v2 {
            ui.small("Monochrome keyboard backlight — brightness and on/off are hardware controlled.");
        }

        if let Some(brightness) = caps.brightness.as_ref() {
            let value = self.lighting.brightness.get_or_insert(brightness.current);
            let lower = if brightness.can_switch_off { 0 } else { brightness.min };
            if ui
                .add(egui::Slider::new(value, lower..=brightness.max).text("Brightness"))
                .changed()
            {
                if *value != 0 && *value < brightness.min {
                    *value = brightness.min;
                }
                apply_live = true;
            }
        }
        if ui
            .add(egui::Slider::new(&mut self.lighting.period_ms, 250..=20_000).text("Effect period (ms)"))
            .changed()
        {
            apply_live = true;
        }
        if ui
            .add(egui::Slider::new(&mut self.lighting.intensity, 0..=100).text("Intensity"))
            .changed()
        {
            apply_live = true;
        }

        ui.add_space(10.0);
        if caps.per_key_v2 && !caps.supported_keys.is_empty() {
            ui.separator();
            ui.heading("Per-Key Paint");
            ui.horizontal(|ui| {
                let mut brush = [self.brush.r, self.brush.g, self.brush.b];
                ui.label("Brush");
                if ui.color_edit_button_srgb(&mut brush).changed() {
                    self.brush = RgbColor::new(brush[0], brush[1], brush[2]);
                }
                if ui.button("Paint all").clicked() {
                    for key in &caps.supported_keys {
                        self.lighting.per_key.insert(*key, self.brush);
                    }
                    self.lighting.effect = LightingEffect::Static;
                    apply_live = true;
                }
                if ui.button("Clear paint").clicked() {
                    self.lighting.per_key.clear();
                    apply_live = true;
                }
            });
            if keyboard::show(
                ui,
                &caps.supported_keys,
                &mut self.lighting.per_key,
                self.lighting.primary,
                self.brush,
            ) {
                self.lighting.effect = LightingEffect::Static;
                apply_live = true;
            }
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(self.selected_online(), egui::Button::new("Apply lighting")).clicked() {
                self.apply_lighting();
            }
            if apply_live
                && self.live_preview
                && self.last_preview.elapsed() >= Duration::from_millis(80)
            {
                self.last_preview = Instant::now();
                self.apply_lighting();
            }
        });
    }

    fn profiles(&mut self, ui: &mut egui::Ui) {
        ui.heading("Profiles & Game Automation");
        ui.label("Profiles survive device reconnects and can include HID++, lighting, camera, and audio controls.");
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label("Profile name");
            ui.text_edit_singleline(&mut self.profile_name);
        });
        ui.horizontal(|ui| {
            ui.label("Apps / games");
            ui.text_edit_singleline(&mut self.profile_apps);
        });
        ui.small("Comma-separated process names, for example: game.exe, steam_app_123");
        ui.checkbox(&mut self.profile_auto, "Automatically activate when a matching process runs");
        if ui.button("Save current device profile").clicked() {
            self.save_profile();
            self.refresh_profiles();
        }

        ui.separator();
        ui.strong("Profile lifecycle");
        ui.horizontal(|ui| {
            ui.label("New name");
            ui.text_edit_singleline(&mut self.profile_action_name);
        });
        let selected = self.selected_profile_name.clone();
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(selected.is_some(), egui::Button::new("Rename")).clicked()
                && let Some(old_name) = selected.clone()
            {
                match Self::rpc(RpcRequest::RenameProfile { old_name, new_name: self.profile_action_name.clone() }) {
                    Ok(RpcData::Ack) => {
                        self.selected_profile_name = Some(self.profile_action_name.trim().to_owned());
                        self.status = "Profile renamed.".into();
                        self.refresh_profiles();
                        self.save_preferences();
                    }
                    Ok(_) => self.status = "Unexpected profile rename response.".into(),
                    Err(error) => self.status = error,
                }
            }
            if ui.add_enabled(selected.is_some(), egui::Button::new("Clone")).clicked()
                && let Some(source_name) = selected.clone()
            {
                match Self::rpc(RpcRequest::CloneProfile { source_name, new_name: self.profile_action_name.clone() }) {
                    Ok(RpcData::Ack) => {
                        self.selected_profile_name = Some(self.profile_action_name.trim().to_owned());
                        self.status = "Profile cloned.".into();
                        self.refresh_profiles();
                        self.save_preferences();
                    }
                    Ok(_) => self.status = "Unexpected profile clone response.".into(),
                    Err(error) => self.status = error,
                }
            }
            if ui.add_enabled(selected.is_some(), egui::Button::new("Delete")).clicked() {
                self.confirm_delete_profile = selected.clone();
            }
        });
        if let Some(name) = self.confirm_delete_profile.clone() {
            ui.horizontal(|ui| {
                ui.label(format!("Delete '{name}'?"));
                if ui.button("Confirm delete").clicked() {
                    match Self::rpc(RpcRequest::DeleteProfile { name: name.clone() }) {
                        Ok(RpcData::Ack) => {
                            self.status = format!("Deleted '{name}'.");
                            self.confirm_delete_profile = None;
                            self.selected_profile_name = None;
                            self.refresh_profiles();
                            self.save_preferences();
                        }
                        Ok(_) => self.status = "Unexpected profile delete response.".into(),
                        Err(error) => self.status = error,
                    }
                }
                if ui.button("Cancel").clicked() {
                    self.confirm_delete_profile = None;
                }
            });
        }

        let import_path = paths::config_dir().join("profiles-import.json");
        let export_path = paths::config_dir().join("profiles-export.json");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Import profiles").clicked() {
                match fs::read_to_string(&import_path) {
                    Ok(json) => match Self::rpc(RpcRequest::ImportProfiles { json, replace: false }) {
                        Ok(RpcData::Profiles(profiles)) => {
                            self.profiles = profiles;
                            self.status = format!("Imported profiles from {}.", import_path.display());
                        }
                        Ok(_) => self.status = "Unexpected profile import response.".into(),
                        Err(error) => self.status = error,
                    },
                    Err(error) => self.status = format!("Could not read {}: {error}", import_path.display()),
                }
            }
            if ui.button("Export selected").clicked() {
                let names = self.selected_profile_name.clone().into_iter().collect::<Vec<_>>();
                match Self::rpc(RpcRequest::ExportProfiles { names }) {
                    Ok(RpcData::ExportedProfiles { json }) => {
                        if let Err(error) = fs::create_dir_all(paths::config_dir()).and_then(|_| fs::write(&export_path, json)) {
                            self.status = format!("Could not write {}: {error}", export_path.display());
                        } else {
                            self.status = format!("Exported profiles to {}.", export_path.display());
                        }
                    }
                    Ok(_) => self.status = "Unexpected profile export response.".into(),
                    Err(error) => self.status = error,
                }
            }
        });
        ui.small(format!("Import path: {}", import_path.display()));
        ui.small(format!("Export path: {}", export_path.display()));

        ui.separator();
        if self.profiles.is_empty() {
            ui.label("No saved profiles yet.");
            return;
        }
        let profiles = self.profiles.clone();
        for profile in profiles {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(self.selected_profile_name.as_deref() == Some(profile.name.as_str()), &profile.name)
                        .clicked()
                    {
                        self.selected_profile_name = Some(profile.name.clone());
                        self.save_preferences();
                    }
                    if ui.add_enabled(self.selected_online(), egui::Button::new("Apply")).clicked() {
                        self.apply_profile(profile.clone());
                    }
                });
                if let Some(dpi) = profile.dpi {
                    ui.small(format!("DPI: {dpi}"));
                }
                if let Some(lighting) = &profile.lighting {
                    ui.small(format!("Lighting: {}", Self::effect_name(lighting.effect)));
                }
                if !profile.controls.is_empty() {
                    ui.small(format!("Device controls: {} saved", profile.controls.len()));
                }
                if !profile.applications.is_empty() {
                    ui.small(format!("Apps: {}", profile.applications.join(", ")));
                }
                if profile.auto_switch {
                    ui.small("Auto-switch enabled");
                }
            });
            ui.add_space(6.0);
        }
    }

    fn integrations(&mut self, ui: &mut egui::Ui) {
        ui.heading("Integrations");
        ui.label("Reactive effects and app-aware automation run in the ReForge daemon.");
        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("Screen Reactive");
            ui.label(
                self.integrations
                    .screen_capture
                    .as_deref()
                    .unwrap_or("Unavailable — install grim on wlroots Wayland or ImageMagick on X11"),
            );
        });
        ui.add_space(6.0);
        ui.group(|ui| {
            ui.strong("Audio Reactive");
            ui.label(
                self.integrations
                    .audio_capture
                    .as_deref()
                    .unwrap_or("Unavailable — install PipeWire tools (pw-cat)"),
            );
        });
        ui.add_space(6.0);
        ui.group(|ui| {
            ui.strong("Game / App Profiles");
            ui.label(if self.integrations.process_watcher {
                "Linux /proc process watcher ready"
            } else {
                "Process watcher unavailable"
            });
            ui.label(format!(
                "Active auto profile: {}",
                self.integrations
                    .active_auto_profile
                    .as_deref()
                    .unwrap_or("None")
            ));
        });
        if ui.button("Refresh integration status").clicked()
            && let Ok(RpcData::Integrations(status)) = Self::rpc(RpcRequest::IntegrationStatus)
        {
            self.integrations = status;
        }
    }

    fn services_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Logitech Services");
        ui.label("Persistent device sessions and supported Linux service backends used by ReForge.");
        ui.horizontal(|ui| {
            if ui.button("Refresh services").clicked() {
                self.refresh_services();
            }
            if ui.button("Rescan hardware").clicked() {
                match Self::rpc(RpcRequest::RescanDevices) {
                    Ok(_) => self.refresh(),
                    Err(error) => self.status = error,
                }
            }
        });
        ui.add_space(10.0);
        for service in self.services.clone() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.strong(service_ui::kind_label(service.kind));
                        ui.small(format!("{} • {}", service_ui::state_label(service.state), service.backend));
                        if let Some(version) = &service.version {
                            ui.small(format!("Version: {version}"));
                        }
                        if let Some(message) = &service.message {
                            ui.label(message);
                        }
                    });
                    let reconnectable = matches!(service.kind, ServiceKind::Hidpp | ServiceKind::Fwupd | ServiceKind::V4l2 | ServiceKind::Pipewire | ServiceKind::Udev);
                    if reconnectable && ui.button("Reconnect").clicked() {
                        match Self::rpc(RpcRequest::ReconnectService { service: service.kind }) {
                            Ok(RpcData::Ack) => {
                                self.status = format!("Reconnect requested for {}.", service_ui::kind_label(service.kind));
                                self.refresh_services();
                            }
                            Ok(_) => self.status = "Unexpected reconnect response.".into(),
                            Err(error) => self.status = error,
                        }
                    }
                });
            });
            ui.add_space(6.0);
        }
        ui.add_space(10.0);
        ui.heading("Selected device session");
        match self.session_status.clone() {
            Some(session) => {
                ui.group(|ui| {
                    ui.strong(format!("{:?}", session.state));
                    ui.label(format!("Device index: {:#04x}", session.device_index));
                    ui.label(format!("HID path: {}", session.path));
                    ui.label(format!("Session reconnects: {}", session.reconnect_count));
                    if let Some(error) = session.last_error {
                        ui.label(format!("Last error: {error}"));
                    }
                });
            }
            None => {
                ui.label("The selected device has no active HID++ session.");
            }
        }
    }

    fn firmware_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Logitech Firmware");
        ui.label("Firmware discovery and installation are delegated to fwupd/LVFS and system polkit authorization.");
        ui.horizontal(|ui| {
            if ui.button("Check firmware devices").clicked() || self.firmware_devices.is_empty() {
                self.refresh_firmware_devices();
            }
            if ui.button("Refresh LVFS metadata").clicked() {
                match Self::rpc(RpcRequest::RefreshFirmwareMetadata) {
                    Ok(RpcData::Ack) => {
                        self.status = "LVFS metadata refresh completed through fwupd.".into();
                        self.refresh_firmware_devices();
                    }
                    Ok(_) => self.status = "Unexpected firmware-refresh response.".into(),
                    Err(error) => self.status = error,
                }
            }
        });
        ui.add_space(10.0);
        if self.firmware_devices.is_empty() {
            let fwupd_state = self.services.iter().find(|service| service.kind == ServiceKind::Fwupd).map(|service| service.state);
            ui.label(match fwupd_state {
                Some(ServiceState::Unavailable) => "fwupd is unavailable. Install/enable fwupd to use Logitech firmware services.",
                Some(ServiceState::Degraded | ServiceState::Error) => "fwupd is present but not healthy. Open Services for details.",
                _ => "No Logitech firmware-capable devices are currently exposed by fwupd.",
            });
            return;
        }
        for device in self.firmware_devices.clone() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.strong(&device.name);
                        ui.small(format!("Current firmware: {}", firmware_ui::device_version(&device)));
                        ui.small(format!("fwupd ID: {}", device.id));
                        if let Some(key) = &device.matched_device_key {
                            ui.small(format!("Matched ReForge device: {key}"));
                        }
                        ui.label(if device.update_available { "Update available" } else { "Firmware current / no advertised update" });
                    });
                    if ui.button("Releases").clicked() {
                        match Self::rpc(RpcRequest::GetFirmwareReleases { device_id: device.id.clone() }) {
                            Ok(RpcData::FirmwareReleases(releases)) => {
                                self.firmware_releases.insert(device.id.clone(), releases);
                                self.selected_firmware_id = Some(device.id.clone());
                            }
                            Ok(_) => self.status = "Unexpected firmware-release response.".into(),
                            Err(error) => self.status = error,
                        }
                    }
                    if ui.add_enabled(device.update_available, egui::Button::new("Install update")).clicked() {
                        self.pending_firmware_install = Some(device.id.clone());
                    }
                });
                if self.pending_firmware_install.as_deref() == Some(device.id.as_str()) {
                    ui.separator();
                    ui.label("Firmware installation will be performed by fwupd using its normal trust, safety and authorization checks.");
                    ui.horizontal(|ui| {
                        if ui.button("Confirm firmware update").clicked() {
                            match Self::rpc(RpcRequest::InstallFirmwareUpdate { device_id: device.id.clone() }) {
                                Ok(RpcData::FirmwareInstall(result)) => {
                                    self.status = format!("{}{}{}", result.message,
                                        if result.needs_replug { " • Replug required." } else { "" },
                                        if result.needs_reboot { " • Reboot required." } else { "" });
                                    self.pending_firmware_install = None;
                                    self.refresh_firmware_devices();
                                }
                                Ok(_) => self.status = "Unexpected firmware-install response.".into(),
                                Err(error) => self.status = error,
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.pending_firmware_install = None;
                        }
                    });
                }
                if let Some(releases) = self.firmware_releases.get(&device.id) {
                    for release in releases {
                        ui.separator();
                        ui.strong(firmware_ui::release_title(release));
                        if let Some(summary) = &release.summary { ui.label(summary); }
                        if let Some(remote) = &release.remote_id { ui.small(format!("Remote: {remote}")); }
                        if release.trusted == Some(true) { ui.small("Trusted payload metadata reported by fwupd"); }
                        if release.needs_replug { ui.small("Requires device replug after installation"); }
                        if release.needs_reboot { ui.small("Requires system reboot after installation"); }
                    }
                }
            });
            ui.add_space(8.0);
        }
    }

    fn diagnostics(&mut self, ui: &mut egui::Ui) {
        ui.heading("Diagnostics");
        ui.label("Runtime state and recent errors without raw HID report payloads.");
        ui.horizontal(|ui| {
            if ui.button("Refresh diagnostics").clicked() || self.diagnostics.is_none() {
                match Self::rpc(RpcRequest::GetDiagnostics) {
                    Ok(RpcData::Diagnostics(snapshot)) => self.diagnostics = Some(snapshot),
                    Ok(_) => self.status = "Unexpected diagnostics response.".into(),
                    Err(error) => self.status = error,
                }
            }
            if ui.button("Clear recent events").clicked() {
                match Self::rpc(RpcRequest::ClearDiagnostics) {
                    Ok(RpcData::Ack) => {
                        self.diagnostics = None;
                        self.status = "Diagnostic event history cleared.".into();
                    }
                    Ok(_) => self.status = "Unexpected diagnostics-clear response.".into(),
                    Err(error) => self.status = error,
                }
            }
        });
        ui.checkbox(&mut self.include_device_identifiers, "Include device identifiers in exported bundle");
        if ui.button("Export diagnostic bundle").clicked() {
            if let Some(snapshot) = self.diagnostics.clone() {
                match diagnostic_ui::export(snapshot, self.include_device_identifiers) {
                    Ok(path) => self.status = format!("Diagnostic bundle exported to {}.", path.display()),
                    Err(error) => self.status = error,
                }
            } else {
                self.status = "Refresh diagnostics before exporting.".into();
            }
        }
        ui.add_space(10.0);
        let Some(snapshot) = self.diagnostics.clone() else {
            ui.label("No diagnostic snapshot loaded.");
            return;
        };
        ui.group(|ui| {
            ui.strong(format!("ReForge {}", snapshot.daemon_version));
            ui.label(format!("Socket: {}", snapshot.runtime_socket_category));
            ui.label(format!("Profiles: {} (store v{})", snapshot.profile_count, snapshot.profile_store_version));
            ui.label(format!("Tracked devices: {}", snapshot.devices.len()));
        });
        ui.add_space(8.0);
        ui.heading("Devices");
        for runtime in snapshot.devices {
            ui.group(|ui| {
                ui.strong(format!("{} • {:?}", runtime.summary.product, runtime.presence));
                ui.small(format!("Key: {}", runtime.summary.key));
                ui.small(format!("PID {:04X} • reconnects {}", runtime.summary.product_id, runtime.reconnect_count));
                if !runtime.summary.providers.is_empty() {
                    ui.small(format!("Providers: {:?}", runtime.summary.providers));
                }
                if !runtime.summary.features.is_empty() {
                    ui.small(format!("HID++ features: {}", runtime.summary.features.iter().map(|f| format!("{:04X}", f.feature_id)).collect::<Vec<_>>().join(", ")));
                }
            });
        }
        ui.add_space(8.0);
        ui.heading("Recent events");
        for event in snapshot.recent_events.iter().rev().take(80) {
            ui.label(format!("[{:?}] {}: {}", event.level, event.component, event.message));
        }
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Control Center Settings");
        ui.label("ReForge follows your desktop color scheme by default.");
        let old_theme = self.theme;
        egui::ComboBox::from_label("Appearance")
            .selected_text(match self.theme {
                ThemeChoice::System => "System",
                ThemeChoice::Light => "Light",
                ThemeChoice::Dark => "Dark",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.theme, ThemeChoice::System, "System");
                ui.selectable_value(&mut self.theme, ThemeChoice::Light, "Light");
                ui.selectable_value(&mut self.theme, ThemeChoice::Dark, "Dark");
            });
        ui.ctx().set_theme(self.theme.preference());
        if old_theme != self.theme {
            self.save_preferences();
        }
        let old_preview = self.live_preview;
        ui.checkbox(&mut self.live_preview, "Live lighting preview");
        if old_preview != self.live_preview {
            self.save_preferences();
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Rescan devices now").clicked() {
                match Self::rpc(RpcRequest::RescanDevices) {
                    Ok(_) => self.refresh_runtime(),
                    Err(error) => self.status = error,
                }
            }
            if ui.button("Restart ReForge daemon").clicked() {
                match Command::new("systemctl")
                    .args(["--user", "restart", "reforge-logitechd.service"])
                    .status()
                {
                    Ok(status) if status.success() => {
                        self.status = "Daemon restart requested; reconnecting automatically.".into();
                        self.last_runtime_poll = Instant::now() - Duration::from_secs(2);
                    }
                    Ok(status) => self.status = format!("systemctl returned {status}."),
                    Err(error) => self.status = format!("failed to run systemctl: {error}"),
                }
            }
        });
        ui.separator();
        ui.strong("Safety boundaries");
        ui.label("Arbitrary firmware flashing/DFU and raw HID report injection remain disabled; supported firmware updates are delegated to fwupd/LVFS.");
        ui.label("Reconnect restoration revalidates every control against the freshly discovered capability set.");
    }

}

impl eframe::App for ReForgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_runtime();
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(170.0);
                    self.nav(ui);
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_min_width(720.0);
                    egui::ScrollArea::vertical().show(ui, |ui| match self.page {
                        Page::Dashboard => self.dashboard(ui),
                        Page::DeviceControls => self.device_controls(ui),
                        Page::Lighting => self.lighting(ui),
                        Page::Profiles => self.profiles(ui),
                        Page::Integrations => self.integrations(ui),
                        Page::Services => self.services_page(ui),
                        Page::Firmware => self.firmware_page(ui),
                        Page::Diagnostics => self.diagnostics(ui),
                        Page::Settings => self.settings(ui),
                    });
                });
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                ui.strong("Status:");
                ui.label(&self.status);
            });
        });
    }
}


#[cfg(test)]
mod page_tests {
    use super::Page;

    #[test]
    fn service_and_firmware_pages_round_trip_through_preferences() {
        for page in [Page::Services, Page::Firmware] {
            assert_eq!(Page::from_str(page.as_str()), page);
        }
    }
}
