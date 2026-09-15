use crate::{device_page, home, ipc, nav, output_dsp, profiles, settings, theme};
use eframe::egui;
use forgehx_core::{
    AudioNode, BackendStatus, Capability, Command, DeviceId, DeviceInfo, EqConfig,
    FirmwareIdentity, FirmwarePackageInfo, KeyboardModelInfo, LightingConfig,
    LightingControllerMetadata, MicrophoneDspConfig, MicrophoneDspState, MouseDeviceState,
    MouseModelInfo, OutputDeviceClass, OutputDspProfile, Reply, IPC_PROTOCOL_VERSION,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Devices,
    OutputDsp,
    Profiles,
    Settings,
}

#[derive(Debug, Clone)]
struct PendingDeviceWrite {
    action: device_page::DeviceAction,
    due_at: Instant,
}

const LIVE_DEVICE_WRITE_DEBOUNCE: Duration = Duration::from_millis(90);

#[derive(Debug, Clone, Copy)]
enum DeviceReconcile {
    Selected,
    Audio,
}

pub struct ForgeHxApp {
    pub page: Page,
    pub hyperx_devices: Vec<DeviceInfo>,
    pub selected_device: Option<DeviceId>,
    pub selected_tab: device_page::DeviceTab,
    pub status: String,
    pub error: Option<String>,
    pub doctor_text: String,
    pub profiles: Vec<String>,
    pub audio_nodes: Vec<AudioNode>,
    pub backend_statuses: Vec<BackendStatus>,
    pub lighting: LightingConfig,
    pub lighting_metadata: Option<LightingControllerMetadata>,
    pub dpi_stages: Vec<u16>,
    pub dpi_active: usize,
    pub mouse_state: Option<MouseDeviceState>,
    pub mouse_model: Option<MouseModelInfo>,
    pub keyboard_model: Option<KeyboardModelInfo>,
    pub polling_rate: u16,
    pub mouse_profile: u8,
    pub button_number: u32,
    pub button_action: String,
    pub mouse_lift_off_distance: u8,
    pub eq_config: EqConfig,
    pub eq_profiles: Vec<String>,
    pub selected_eq_profile: String,
    pub mic_dsp_config: MicrophoneDspConfig,
    pub mic_dsp_state: Option<MicrophoneDspState>,
    pub mic_monitor_enabled: bool,
    pub mic_monitor_level_percent: f32,
    pub output_dsp_profile: OutputDspProfile,
    pub output_dsp_live: bool,
    pub output_dsp_generation: u64,
    pub firmware_identity: Option<FirmwareIdentity>,
    pub firmware_package: Option<FirmwarePackageInfo>,
    pub firmware_path: String,
    pub preferences: settings::UiPreferences,
    mic_dsp_device: Option<DeviceId>,
    last_sync: Instant,
    pending_refresh: bool,
    pending_device_write: Option<PendingDeviceWrite>,
}

impl ForgeHxApp {
    pub fn new(ctx: &egui::Context) -> Self {
        theme::apply(ctx);
        let mut app = Self {
            page: Page::Home,
            hyperx_devices: Vec::new(),
            selected_device: None,
            selected_tab: device_page::DeviceTab::Overview,
            status: "Connecting to ForgeHX daemon…".into(),
            error: None,
            doctor_text: String::new(),
            profiles: Vec::new(),
            audio_nodes: Vec::new(),
            backend_statuses: Vec::new(),
            lighting: LightingConfig::default(),
            lighting_metadata: None,
            dpi_stages: vec![400, 800, 1600, 3200],
            dpi_active: 2,
            mouse_state: None,
            mouse_model: None,
            keyboard_model: None,
            polling_rate: 1000,
            mouse_profile: 0,
            button_number: 0,
            button_action: "mouse:left".into(),
            mouse_lift_off_distance: 1,
            eq_config: EqConfig::default(),
            eq_profiles: Vec::new(),
            selected_eq_profile: String::new(),
            mic_dsp_config: MicrophoneDspConfig::default(),
            mic_dsp_state: None,
            mic_monitor_enabled: false,
            mic_monitor_level_percent: 50.0,
            output_dsp_profile: OutputDspProfile::factory(OutputDeviceClass::Custom),
            output_dsp_live: false,
            output_dsp_generation: 0,
            firmware_identity: None,
            firmware_package: None,
            firmware_path: String::new(),
            preferences: settings::UiPreferences::load(),
            mic_dsp_device: None,
            last_sync: Instant::now() - Duration::from_secs(5),
            pending_refresh: false,
            pending_device_write: None,
        };
        app.sync_inventory();
        app
    }

    fn sync_inventory(&mut self) {
        self.error = None;
        match ipc::send(Command::DaemonStatus {
            protocol_version: IPC_PROTOCOL_VERSION,
        }) {
            Ok(Reply::Status {
                protocol_version,
                device_count,
                all_device_count,
                write_protected_count,
            }) => {
                let _ = all_device_count;
                self.status = format!("Daemon online • protocol {protocol_version} • {device_count} HyperX • {write_protected_count} protected");
            }
            Ok(Reply::Error { message, .. }) => self.error = Some(message),
            Ok(_) => self.status = "Daemon returned unexpected status data".into(),
            Err(error) => {
                self.status = "Daemon offline".into();
                self.error = Some(error);
                self.last_sync = Instant::now();
                return;
            }
        }
        if let Ok(Reply::Devices { devices }) = ipc::send(Command::ListHyperxDevices {
            protocol_version: IPC_PROTOCOL_VERSION,
        }) {
            self.hyperx_devices = devices;
        }
        if let Ok(Reply::Profiles { names }) = ipc::send(Command::ProfileList {
            protocol_version: IPC_PROTOCOL_VERSION,
        }) {
            self.profiles = names;
        }
        if let Ok(Reply::AudioNodes { nodes }) = ipc::send(Command::ListAudio {
            protocol_version: IPC_PROTOCOL_VERSION,
        }) {
            self.audio_nodes = nodes;
        }
        if let Ok(Reply::BackendStatuses { backends }) = ipc::send(Command::BackendStatus {
            protocol_version: IPC_PROTOCOL_VERSION,
        }) {
            self.backend_statuses = backends;
        }
        if let Ok(Reply::EqProfiles { names }) = ipc::send(Command::EqList {
            protocol_version: IPC_PROTOCOL_VERSION,
        }) {
            self.eq_profiles = names;
            if self.selected_eq_profile.is_empty() {
                self.selected_eq_profile = self.eq_profiles.first().cloned().unwrap_or_default();
            }
        }
        if let Ok(Reply::OutputDspState {
            profile,
            live,
            generation,
            ..
        }) = ipc::send(Command::OutputDspGet {
            protocol_version: IPC_PROTOCOL_VERSION,
        }) {
            self.output_dsp_profile = profile;
            self.output_dsp_live = live;
            self.output_dsp_generation = generation;
        }
        if let Some(selected) = &self.selected_device {
            if !self
                .hyperx_devices
                .iter()
                .any(|device| &device.id == selected)
            {
                self.selected_device = self.hyperx_devices.first().map(|device| device.id.clone());
            }
        }
        self.sync_selected_backend_state();
        self.last_sync = Instant::now();
    }

    fn sync_selected_backend_state(&mut self) {
        let Some(device) = self.selected() else {
            self.lighting_metadata = None;
            self.mouse_state = None;
            self.mouse_model = None;
            self.keyboard_model = None;
            self.mic_dsp_state = None;
            self.firmware_identity = None;
            self.firmware_package = None;
            self.mic_dsp_device = None;
            return;
        };
        if device.supports(Capability::Lighting) {
            if let Ok(Reply::LightingController { metadata }) =
                ipc::send(Command::LightingMetadata {
                    protocol_version: IPC_PROTOCOL_VERSION,
                    device_id: device.id.clone(),
                })
            {
                self.lighting_metadata = metadata;
            }
        } else {
            self.lighting_metadata = None;
        }
        if device.supports(Capability::Eq) {
            let ids = device
                .interfaces
                .iter()
                .filter_map(|interface| interface.audio_node_id)
                .collect::<Vec<_>>();
            if let Some(node) = self
                .audio_nodes
                .iter()
                .find(|node| node.kind == "sink" && ids.contains(&node.id))
            {
                self.eq_config.target_node_id = Some(node.id);
            }
        }
        if device.supports(Capability::MicDsp) {
            if let Ok(Reply::MicDspState { mut state }) = ipc::send(Command::MicDspGet {
                protocol_version: IPC_PROTOCOL_VERSION,
                device_id: device.id.clone(),
            }) {
                let same_device = self.mic_dsp_device.as_ref() == Some(&device.id);
                if !same_device {
                    self.mic_dsp_config = state.config.clone();
                } else if state.applied {
                    // The voiceprint is daemon/runtime-owned enrollment data. Refresh that one
                    // field without surrendering user ownership of any editable DSP setting.
                    self.mic_dsp_config.speaker_lock.voiceprint =
                        state.config.speaker_lock.voiceprint.clone();
                }
                // FORGEHX_USER_CONFIG_AUTHORITY: the editor is authoritative for editable
                // settings; daemon snapshots provide runtime status/telemetry only.
                state.config = self.mic_dsp_config.clone();
                self.mic_monitor_enabled = state.monitor.enabled;
                self.mic_monitor_level_percent = state.monitor.level_percent;
                self.mic_dsp_device = Some(device.id.clone());
                self.mic_dsp_state = Some(*state);
            }
        } else {
            self.mic_dsp_state = None;
            self.mic_dsp_device = None;
        }
        if device.supports(Capability::MicFirmwareInventory) {
            self.firmware_identity = match ipc::send(Command::MicFirmwareGet {
                protocol_version: IPC_PROTOCOL_VERSION,
                device_id: device.id.clone(),
            }) {
                Ok(Reply::FirmwareIdentity { identity }) => Some(identity),
                _ => None,
            };
        } else {
            self.firmware_identity = None;
            self.firmware_package = None;
        }
        if device.device_class == forgehx_core::DeviceClass::Mouse {
            self.mouse_model = match ipc::send(Command::MouseCapabilities {
                protocol_version: IPC_PROTOCOL_VERSION,
                device_id: device.id.clone(),
            }) {
                Ok(Reply::MouseCapabilities { model }) => model,
                _ => None,
            };
        } else {
            self.mouse_model = None;
        }
        if device.device_class == forgehx_core::DeviceClass::Keyboard {
            self.keyboard_model = match ipc::send(Command::KeyboardCapabilities {
                protocol_version: IPC_PROTOCOL_VERSION,
                device_id: device.id.clone(),
            }) {
                Ok(Reply::KeyboardCapabilities { model }) => model,
                _ => None,
            };
        } else {
            self.keyboard_model = None;
        }
        if device.supports(Capability::Dpi)
            || device.supports(Capability::PollingRate)
            || device.supports(Capability::Profiles)
        {
            if let Ok(Reply::MouseState { state }) = ipc::send(Command::MouseState {
                protocol_version: IPC_PROTOCOL_VERSION,
                device_id: device.id.clone(),
            }) {
                if !state.dpi_stages.is_empty() {
                    self.dpi_stages = state.dpi_stages.clone();
                    self.dpi_active = state
                        .active_dpi_stage
                        .unwrap_or(0)
                        .min(self.dpi_stages.len().saturating_sub(1));
                }
                if let Some(rate) = state.report_rate_hz {
                    self.polling_rate = rate;
                }
                if let Some(profile) = state.active_profile {
                    self.mouse_profile = profile;
                }
                if let Some(mm) = state.lift_off_distance_mm {
                    self.mouse_lift_off_distance = mm;
                }
                if let Some(readback) = state.lighting.as_ref() {
                    // The Haste 0x52 report confirms RGB and brightness. Effect mode/speed
                    // are not exposed by that report, so preserve those editor fields.
                    self.lighting.color = readback.color;
                    self.lighting.brightness = readback.brightness;
                }
                self.mouse_state = Some(state);
            }
        } else {
            self.mouse_state = None;
        }
    }

    fn sync_selected_pane_state(&mut self, pane: device_page::DeviceTab) {
        if matches!(
            pane,
            device_page::DeviceTab::Audio
                | device_page::DeviceTab::Microphone
                | device_page::DeviceTab::Hardware
        ) {
            if let Ok(Reply::AudioNodes { nodes }) = ipc::send(Command::ListAudio {
                protocol_version: IPC_PROTOCOL_VERSION,
            }) {
                self.audio_nodes = nodes;
            }
        }
        self.sync_selected_backend_state();
    }

    fn refresh_after_update(&mut self) {
        if self.preferences.refresh_after_changes {
            self.pending_refresh = true;
        }
    }

    fn explicit_refresh(&mut self) {
        match ipc::send(Command::BackendRescan {
            protocol_version: IPC_PROTOCOL_VERSION,
        }) {
            Ok(Reply::Ok { message }) => self.status = message,
            Ok(Reply::Error { message, .. }) => self.error = Some(message),
            Ok(_) => {}
            Err(error) => self.error = Some(error),
        }
        self.sync_inventory();
    }

    fn ensure_backends(&mut self) {
        self.set_command(Command::BackendEnsure {
            protocol_version: IPC_PROTOCOL_VERSION,
        });
    }

    fn restart_backend(&mut self, backend: forgehx_core::BackendKind) {
        self.set_command(Command::BackendRestart {
            protocol_version: IPC_PROTOCOL_VERSION,
            backend,
        });
    }

    fn selected(&self) -> Option<DeviceInfo> {
        let id = self.selected_device.as_ref()?;
        self.hyperx_devices
            .iter()
            .find(|device| &device.id == id)
            .cloned()
    }

    fn commit_device_action(&mut self, action: device_page::DeviceAction) {
        use device_page::DeviceAction;
        let p = IPC_PROTOCOL_VERSION;
        match action {
            DeviceAction::RunDoctor(device_id) => match ipc::send(Command::Doctor {
                protocol_version: p,
                device_id: Some(device_id),
            }) {
                Ok(Reply::Doctor { reports }) => {
                    self.doctor_text =
                        serde_json::to_string_pretty(&reports).unwrap_or_else(|e| e.to_string());
                    self.error = None;
                }
                Ok(Reply::Error { message, .. }) => self.error = Some(message),
                Ok(_) => self.error = Some("Unexpected Device Doctor response".into()),
                Err(error) => self.error = Some(error),
            },
            DeviceAction::SetLighting(device_id, config) => self.send_device_setting_command(
                Command::SetLighting {
                    protocol_version: p,
                    device_id,
                    config,
                },
                DeviceReconcile::Selected,
            ),
            DeviceAction::SetDpi(device_id, config) => self.send_device_setting_command(
                Command::SetDpi {
                    protocol_version: p,
                    device_id,
                    config,
                },
                DeviceReconcile::Selected,
            ),
            DeviceAction::SetPollingRate(device_id, hz) => self.send_device_setting_command(
                Command::SetPollingRate {
                    protocol_version: p,
                    device_id,
                    hz,
                },
                DeviceReconcile::Selected,
            ),
            DeviceAction::SetMouseProfile(device_id, profile) => self.send_device_setting_command(
                Command::SetMouseProfile {
                    protocol_version: p,
                    device_id,
                    profile,
                },
                DeviceReconcile::Selected,
            ),
            DeviceAction::SetButtonAssignment(device_id, button, assignment) => self
                .send_device_setting_command(
                    Command::SetButtonAssignment {
                        protocol_version: p,
                        device_id,
                        button,
                        action: assignment,
                    },
                    DeviceReconcile::Selected,
                ),
            DeviceAction::SetLiftOffDistance(device_id, mm) => self.send_device_setting_command(
                Command::SetLiftOffDistance {
                    protocol_version: p,
                    device_id,
                    mm,
                },
                DeviceReconcile::Selected,
            ),
            DeviceAction::SetAudioVolume(node_id, volume) => self.send_device_setting_command(
                Command::AudioSetVolume {
                    protocol_version: p,
                    node_id,
                    volume,
                },
                DeviceReconcile::Audio,
            ),
            DeviceAction::SetAudioMute(node_id, muted) => self.send_device_setting_command(
                Command::AudioSetMute {
                    protocol_version: p,
                    node_id,
                    muted,
                },
                DeviceReconcile::Audio,
            ),
            DeviceAction::EqLoad(name) => match ipc::send(Command::EqGet {
                protocol_version: p,
                name: name.clone(),
            }) {
                Ok(Reply::EqProfile { config }) => {
                    self.eq_config = config;
                    self.selected_eq_profile = name;
                    self.error = None;
                }
                Ok(Reply::Error { message, .. }) => self.error = Some(message),
                Ok(_) => self.error = Some("Unexpected EQ response".into()),
                Err(error) => self.error = Some(error),
            },
            DeviceAction::EqSave(config) => {
                self.selected_eq_profile = config.name.clone();
                self.eq_config = config.clone();
                self.set_command(Command::EqSave {
                    protocol_version: p,
                    config,
                });
            }
            DeviceAction::EqApply(name) => self.set_command(Command::EqApply {
                protocol_version: p,
                name,
            }),
            DeviceAction::EqDelete(name) => {
                self.set_command(Command::EqDelete {
                    protocol_version: p,
                    name,
                });
                self.selected_eq_profile.clear();
            }
            DeviceAction::EqBypass => self.set_command(Command::EqBypass {
                protocol_version: p,
            }),
            DeviceAction::MicDspSave(device_id, config) => {
                self.mic_dsp_config = config.clone();
                self.set_command(Command::MicDspSave {
                    protocol_version: p,
                    device_id,
                    config,
                });
            }
            DeviceAction::MicDspLiveUpdate(device_id, config) => {
                self.mic_dsp_config = config.clone();
                self.handle_mic_dsp_reply(Command::MicDspLiveUpdate {
                    protocol_version: p,
                    device_id,
                    config,
                });
            }
            DeviceAction::MicDspApply(device_id, config) => {
                let name = config.name.clone();
                match ipc::send(Command::MicDspSave {
                    protocol_version: p,
                    device_id: device_id.clone(),
                    config: config.clone(),
                }) {
                    Ok(Reply::Ok { .. }) => {
                        self.mic_dsp_config = config;
                        self.handle_mic_dsp_reply(Command::MicDspApply {
                            protocol_version: p,
                            device_id,
                            name,
                        });
                    }
                    Ok(Reply::Error { message, .. }) => self.error = Some(message),
                    Ok(_) => self.error = Some("Unexpected microphone DSP save response".into()),
                    Err(error) => self.error = Some(error),
                }
            }
            DeviceAction::MicVoiceForget(device_id) => {
                self.handle_mic_dsp_reply(Command::MicVoiceForget {
                    protocol_version: p,
                    device_id,
                })
            }
            DeviceAction::MicMonitor(device_id, enabled, level_percent) => {
                self.mic_monitor_enabled = enabled;
                self.mic_monitor_level_percent = level_percent;
                self.handle_mic_dsp_reply(Command::MicMonitorSet {
                    protocol_version: p,
                    device_id,
                    enabled,
                    level_percent,
                });
            }
            DeviceAction::MicFirmwareRefresh(device_id) => {
                self.handle_firmware_reply(ipc::send(Command::MicFirmwareGet {
                    protocol_version: p,
                    device_id,
                }))
            }
            DeviceAction::MicFirmwareStage(device_id, path) => {
                self.handle_firmware_reply(ipc::send(Command::MicFirmwareStage {
                    protocol_version: p,
                    device_id,
                    path,
                }))
            }
            DeviceAction::MicFirmwareValidate(device_id, staged_id) => {
                self.handle_firmware_reply(ipc::send(Command::MicFirmwareValidate {
                    protocol_version: p,
                    device_id,
                    staged_id,
                }))
            }
            DeviceAction::MicFirmwareBegin(device_id, staged_id) => {
                self.handle_firmware_reply(ipc::send(Command::MicFirmwareBegin {
                    protocol_version: p,
                    device_id,
                    staged_id,
                }))
            }
            DeviceAction::MicFirmwareForget(staged_id) => {
                self.handle_firmware_reply(ipc::send(Command::MicFirmwareForget {
                    protocol_version: p,
                    staged_id,
                }));
                self.firmware_package = None;
            }
        }
    }

    fn queue_device_action(&mut self, action: device_page::DeviceAction) {
        use device_page::DeviceAction;
        let debounced = matches!(
            &action,
            DeviceAction::SetLighting(_, _)
                | DeviceAction::SetDpi(_, _)
                | DeviceAction::SetAudioVolume(_, _)
                | DeviceAction::MicDspLiveUpdate(_, _)
                | DeviceAction::MicMonitor(_, _, _)
        );
        if debounced {
            let replaces_same_control = self.pending_device_write.as_ref().is_none_or(|pending| {
                std::mem::discriminant(&pending.action) == std::mem::discriminant(&action)
            });
            if !replaces_same_control {
                self.flush_pending_device_write();
            }
            self.pending_device_write = Some(PendingDeviceWrite {
                action,
                due_at: Instant::now() + LIVE_DEVICE_WRITE_DEBOUNCE,
            });
        } else {
            self.flush_pending_device_write();
            self.commit_device_action(action);
        }
    }

    fn flush_pending_device_write(&mut self) {
        if let Some(pending) = self.pending_device_write.take() {
            self.commit_device_action(pending.action);
        }
    }

    fn send_device_setting_command(&mut self, command: Command, reconcile: DeviceReconcile) {
        match ipc::send(command) {
            Ok(Reply::Ok { message }) => {
                self.status = message;
                self.error = None;
            }
            Ok(Reply::Error { message, .. }) => self.error = Some(message),
            Ok(_) => self.error = Some("Unexpected ForgeHX device-setting response".into()),
            Err(error) => self.error = Some(error),
        }
        // Read back after success OR failure. On success this confirms what hardware
        // accepted; on failure it restores the editor to the last device-reported state.
        self.reconcile_device_after_write(reconcile);
    }

    fn reconcile_device_after_write(&mut self, reconcile: DeviceReconcile) {
        match reconcile {
            DeviceReconcile::Audio => {
                if let Ok(Reply::AudioNodes { nodes }) = ipc::send(Command::ListAudio {
                    protocol_version: IPC_PROTOCOL_VERSION,
                }) {
                    self.audio_nodes = nodes;
                }
            }
            DeviceReconcile::Selected => self.sync_selected_backend_state(),
        }
    }

    fn handle_firmware_reply(&mut self, reply: Result<Reply, String>) {
        let mut changed = false;
        match reply {
            Ok(Reply::FirmwareIdentity { identity }) => {
                self.status = "Firmware inventory refreshed".into();
                self.firmware_identity = Some(identity);
                self.error = None;
                changed = true;
            }
            Ok(Reply::FirmwarePackage { package }) => {
                self.status = format!("Firmware package {:?}", package.validation);
                self.firmware_package = Some(package);
                self.error = None;
                changed = true;
            }
            Ok(Reply::FirmwareTransaction { status }) => {
                self.status = format!("Firmware transaction {:?}", status.state);
                self.error = None;
                changed = true;
            }
            Ok(Reply::Ok { message }) => {
                self.status = message;
                self.error = None;
                changed = true;
            }
            Ok(Reply::Error { message, .. }) => self.error = Some(message),
            Ok(_) => self.error = Some("Unexpected firmware response".into()),
            Err(error) => self.error = Some(error),
        }
        if changed {
            self.refresh_after_update();
        }
    }

    fn handle_mic_dsp_reply(&mut self, command: Command) {
        let forget_voice = matches!(&command, Command::MicVoiceForget { .. });
        let monitor_change = matches!(&command, Command::MicMonitorSet { .. });
        let mut changed = false;
        match ipc::send(command) {
            Ok(Reply::MicDspState { mut state }) => {
                self.status = if monitor_change {
                    if state.monitor.enabled {
                        "Live headphone mic monitor enabled".into()
                    } else {
                        "Live headphone mic monitor disabled".into()
                    }
                } else if state.applied {
                    "ForgeHX Mic active".into()
                } else {
                    "ForgeHX Mic bypassed".into()
                };
                if forget_voice {
                    self.mic_dsp_config.speaker_lock.voiceprint.clear();
                } else if state.applied {
                    self.mic_dsp_config.speaker_lock.voiceprint =
                        state.config.speaker_lock.voiceprint.clone();
                }
                // FORGEHX_USER_CONFIG_AUTHORITY: never replace sliders/toggles with a
                // daemon snapshot after a user action. Keep the accepted editor values and
                // merge only daemon-owned runtime/enrollment state.
                state.config = self.mic_dsp_config.clone();
                self.mic_monitor_enabled = state.monitor.enabled;
                self.mic_monitor_level_percent = state.monitor.level_percent;
                self.mic_dsp_state = Some(*state);
                self.error = None;
                changed = true;
            }
            Ok(Reply::Error { message, .. }) => self.error = Some(message),
            Ok(_) => self.error = Some("Unexpected microphone DSP response".into()),
            Err(error) => self.error = Some(error),
        }
        if changed {
            self.refresh_after_update();
        }
    }

    fn set_command(&mut self, command: Command) {
        match ipc::send(command) {
            Ok(Reply::Ok { message }) => {
                self.status = message;
                self.error = None;
                self.refresh_after_update();
            }
            Ok(Reply::Error { message, .. }) => self.error = Some(message),
            Ok(_) => self.error = Some("Unexpected ForgeHX response".into()),
            Err(error) => self.error = Some(error),
        }
    }
}

impl eframe::App for ForgeHxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self
            .pending_device_write
            .as_ref()
            .is_some_and(|pending| Instant::now() >= pending.due_at)
        {
            self.flush_pending_device_write();
        }
        if self.pending_refresh {
            self.pending_refresh = false;
            self.sync_inventory();
        } else if self.preferences.automatic_inventory_refresh
            && self.last_sync.elapsed()
                >= Duration::from_secs(self.preferences.inventory_refresh_seconds.clamp(1, 60))
        {
            self.sync_inventory();
        }
        ctx.request_repaint_after(Duration::from_millis(250));
        let selected_device_before = self.selected_device.clone();
        nav::show(
            ctx,
            &mut self.page,
            &self.hyperx_devices,
            &mut self.selected_device,
        );
        if self.selected_device != selected_device_before {
            self.flush_pending_device_write();
            self.selected_tab = device_page::DeviceTab::Overview;
            self.sync_selected_backend_state();
        }
        egui::TopBottomPanel::bottom("forgehx_status")
            .frame(
                egui::Frame::new()
                    .fill(theme::bg_nav())
                    .inner_margin(egui::Margin::symmetric(14, 7)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&self.status);
                    if let Some(error) = &self.error {
                        ui.separator();
                        ui.colored_label(theme::status_blocked(), error);
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::bg_app())
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| {
                // FORGEHX_ALL_PAGE_SCROLL_HOST
                let scroll_id = match self.page {
                    Page::Home => "forgehx_scroll_home",
                    Page::Devices => "forgehx_scroll_devices",
                    Page::OutputDsp => "forgehx_scroll_output_dsp",
                    Page::Profiles => "forgehx_scroll_profiles",
                    Page::Settings => "forgehx_scroll_settings",
                };
                egui::ScrollArea::vertical()
                    .id_salt(scroll_id)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.page {
                        Page::Home => {
                            if let Some(id) = home::show(ui, &self.hyperx_devices) {
                                self.selected_device = Some(id);
                                self.selected_tab = device_page::DeviceTab::Overview;
                                self.page = Page::Devices;
                                self.sync_selected_backend_state();
                            }
                        }
                        Page::Devices => {
                            if let Some(device) = self.selected() {
                                // User controls are always live. There is no mode in which ForgeHX
                                // renders editable DSP controls but refuses to accept the user's edit.
                                let live_mic_dsp_edits = true;
                                let selected_tab_before = self.selected_tab;
                                let mut pane_changed = false;
                                let action = device_page::show(
                                    ui,
                                    &device,
                                    &mut self.selected_tab,
                                    &mut pane_changed,
                                    &mut self.doctor_text,
                                    &mut self.lighting,
                                    self.lighting_metadata.as_ref(),
                                    &mut self.dpi_stages,
                                    &mut self.dpi_active,
                                    self.mouse_state.as_ref(),
                                    self.mouse_model.as_ref(),
                                    self.keyboard_model.as_ref(),
                                    &mut self.polling_rate,
                                    &mut self.mouse_profile,
                                    &mut self.button_number,
                                    &mut self.button_action,
                                    &mut self.mouse_lift_off_distance,
                                    &self.audio_nodes,
                                    &mut self.mic_dsp_config,
                                    self.mic_dsp_state.as_ref(),
                                    live_mic_dsp_edits,
                                    &mut self.mic_monitor_enabled,
                                    &mut self.mic_monitor_level_percent,
                                    self.firmware_identity.as_ref(),
                                    self.firmware_package.as_ref(),
                                    &mut self.firmware_path,
                                    &self.profiles,
                                    &mut self.eq_config,
                                    &self.eq_profiles,
                                    &mut self.selected_eq_profile,
                                );
                                if let Some(action) = action {
                                    self.queue_device_action(action);
                                }
                                if pane_changed || self.selected_tab != selected_tab_before {
                                    self.flush_pending_device_write();
                                    self.sync_selected_pane_state(self.selected_tab);
                                }
                            } else {
                                crate::widgets::page_heading(
                                    ui,
                                    "Devices",
                                    "Select a HyperX device from the navigation rail.",
                                );
                                ui.label("No HyperX device selected.");
                            }
                        }
                        Page::OutputDsp => {
                            if let Some(action) = output_dsp::show(
                                ui,
                                &mut self.output_dsp_profile,
                                self.output_dsp_live,
                                self.output_dsp_generation,
                            ) {
                                let command = match action {
                                    output_dsp::OutputDspAction::LiveUpdate(profile) => {
                                        Command::OutputDspLiveUpdate {
                                            protocol_version: IPC_PROTOCOL_VERSION,
                                            profile,
                                        }
                                    }
                                    output_dsp::OutputDspAction::ResetFactory(device_class) => {
                                        Command::OutputDspResetFactory {
                                            protocol_version: IPC_PROTOCOL_VERSION,
                                            device_class,
                                        }
                                    }
                                };
                                match ipc::send(command) {
                                    Ok(Reply::OutputDspState {
                                        profile,
                                        live,
                                        generation,
                                        ..
                                    }) => {
                                        self.output_dsp_profile = profile;
                                        self.output_dsp_live = live;
                                        self.output_dsp_generation = generation;
                                        self.status =
                                            "Output DSP updated live through AetherStream".into();
                                        self.error = None;
                                    }
                                    Ok(Reply::Error { message, .. }) => self.error = Some(message),
                                    Ok(_) => {
                                        self.error = Some("Unexpected output DSP response".into())
                                    }
                                    Err(error) => self.error = Some(error),
                                }
                            }
                        }
                        Page::Profiles => profiles::show(ui, &self.profiles),
                        Page::Settings => {
                            let response = settings::show(
                                ui,
                                &self.status,
                                self.error.as_deref(),
                                self.hyperx_devices.len(),
                                &self.backend_statuses,
                                &mut self.preferences,
                            );
                            if response.changed {
                                match self.preferences.save() {
                                    Ok(()) => {
                                        self.status = "ForgeHX settings saved".into();
                                        self.error = None;
                                    }
                                    Err(error) => {
                                        self.error = Some(format!(
                                            "Could not save ForgeHX settings: {error}"
                                        ))
                                    }
                                }
                            }
                            if let Some(action) = response.action {
                                match action {
                                    settings::SettingsAction::Rescan => self.explicit_refresh(),
                                    settings::SettingsAction::EnsureBackends => {
                                        self.ensure_backends()
                                    }
                                    settings::SettingsAction::RestartBackend(backend) => {
                                        self.restart_backend(backend)
                                    }
                                }
                            }
                        }
                    });
            });
    }
}
