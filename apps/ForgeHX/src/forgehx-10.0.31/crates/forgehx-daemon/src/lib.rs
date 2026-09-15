use forgehx_aetherstream::{OutputDspClient, DEFAULT_OUTPUT_DSP_TARGET};
use forgehx_audio::{AudioBackend, EqManager, WpctlBackend};
use forgehx_backends::{
    match_external_device, select_owner_with_health, BackendCandidate, BackendError,
    BackendServiceManager, ExternalIdentity, MatchError, OpenRgbClient, RatbagClient,
};
use forgehx_core::{
    AudioNode, BackendHealth, BackendKind, BackendStatus, Capability, CapabilityOwner, Command,
    ConnectionKind, DeviceClass, DeviceId, DeviceInfo, DeviceInterface, FirmwareIdentity,
    FirmwarePackageInfo, FirmwareValidation, ForgeHxError, InterfaceSource,
    KeyboardLightingTopology as CoreKeyboardLightingTopology, KeyboardLimits as CoreKeyboardLimits,
    KeyboardModelInfo, KeyboardProtocolFamily as CoreKeyboardProtocolFamily, OutputDeviceClass,
    OutputDspProfile, Profile, Reply, SupportLevel, VendorFamily, IPC_MIN_PROTOCOL_VERSION,
    IPC_PROTOCOL_VERSION,
};
use forgehx_device::{
    classify, discover_devices, doctor_reports, DiscoveredDevice, DiscoveryBackend, RawInterface,
    SystemDiscoveryBackend,
};
use forgehx_dsp::MicDspManager;
use forgehx_firmware::{
    validate_package, FirmwareJournal, FirmwareRegistry, FirmwareStager, StagedFirmware,
};
use forgehx_keyboard::{
    model_for as keyboard_model_for, KeyboardLightingTopology as RegistryKeyboardLightingTopology,
    KeyboardProtocolFamily as RegistryKeyboardProtocolFamily,
};
use forgehx_mic::complete_microphone_support;
use forgehx_mouse::{
    model_for, MouseError, PulsefireHasteWireless, SagaPro,
    HASTE_V1_DRIVER_ID as PULSEFIRE_HASTE_DRIVER_ID, SAGA_PRO_DRIVER_ID,
};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tracing::{info, warn};

pub struct ProfileStore {
    root: PathBuf,
}

impl ProfileStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn default_path() -> PathBuf {
        if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(path).join("forgehx/profiles")
        } else if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join(".config/forgehx/profiles")
        } else {
            PathBuf::from(".forgehx/profiles")
        }
    }
    pub fn list(&self) -> Result<Vec<String>, ForgeHxError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_owned());
                }
            }
        }
        names.sort();
        Ok(names)
    }
    pub fn load(&self, name: &str) -> Result<Profile, ForgeHxError> {
        validate_profile_filename(name)?;
        let path = self.root.join(format!("{name}.json"));
        let bytes = fs::read(&path).map_err(io_error)?;
        let profile: Profile = serde_json::from_slice(&bytes)
            .map_err(|e| ForgeHxError::InvalidProfile(format!("{}: {e}", path.display())))?;
        let profile = profile.migrate()?;
        profile.validate()?;
        Ok(profile)
    }
    pub fn save(&self, profile: &Profile) -> Result<(), ForgeHxError> {
        let profile = profile.clone().migrate()?;
        profile.validate()?;
        validate_profile_filename(&profile.name)?;
        fs::create_dir_all(&self.root).map_err(io_error)?;
        let final_path = self.root.join(format!("{}.json", profile.name));
        let temp_path = self.root.join(format!(".{}.json.tmp", profile.name));
        let bytes = serde_json::to_vec_pretty(&profile)
            .map_err(|e| ForgeHxError::InvalidProfile(e.to_string()))?;
        fs::write(&temp_path, bytes).map_err(io_error)?;
        fs::rename(&temp_path, &final_path).map_err(io_error)?;
        Ok(())
    }
}

fn validate_profile_filename(name: &str) -> Result<(), ForgeHxError> {
    if name.trim().is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
    {
        Err(ForgeHxError::InvalidProfile("invalid profile name".into()))
    } else {
        Ok(())
    }
}
fn io_error(error: io::Error) -> ForgeHxError {
    ForgeHxError::Io(error.to_string())
}

fn output_dsp_state_path() -> PathBuf {
    if let Some(path) = std::env::var_os("XDG_STATE_HOME") {
        PathBuf::from(path).join("forgehx/output-dsp.json")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".local/state/forgehx/output-dsp.json")
    } else {
        PathBuf::from(".forgehx/output-dsp.json")
    }
}

fn load_output_dsp_profile() -> OutputDspProfile {
    fs::read(output_dsp_state_path())
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| OutputDspProfile::factory(OutputDeviceClass::Custom))
}

fn save_output_dsp_profile(profile: &OutputDspProfile) -> Result<(), ForgeHxError> {
    let path = output_dsp_state_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(profile)
        .map_err(|e| ForgeHxError::InvalidProfile(e.to_string()))?;
    fs::write(&tmp, bytes).map_err(io_error)?;
    fs::rename(tmp, path).map_err(io_error)
}

fn firmware_data_root() -> PathBuf {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(path).join("forgehx/firmware")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".local/share/forgehx/firmware")
    } else {
        PathBuf::from(".forgehx/firmware")
    }
}

pub struct DaemonState {
    pub hyperx_devices: Vec<DeviceInfo>,
    pub all_devices: Vec<DeviceInfo>,
    pub backend_statuses: Vec<BackendStatus>,
    discovered: Vec<DiscoveredDevice>,
    profiles: ProfileStore,
    discovery: Box<dyn DiscoveryBackend>,
    audio: Box<dyn AudioBackend>,
    eq: EqManager,
    mic_dsp: MicDspManager,
    output_dsp_profile: OutputDspProfile,
    output_dsp_generation: u64,
    firmware_stager: FirmwareStager,
    firmware_journal: FirmwareJournal,
    staged_firmware: BTreeMap<String, StagedFirmware>,
    validated_firmware: BTreeMap<String, FirmwarePackageInfo>,
    openrgb: OpenRgbClient,
    ratbag: RatbagClient,
    backend_services: BackendServiceManager,
    external_backends: bool,
}

impl DaemonState {
    pub fn system() -> Self {
        Self::new_internal(
            ProfileStore::new(ProfileStore::default_path()),
            Box::new(SystemDiscoveryBackend::default()),
            Box::new(WpctlBackend),
            EqManager::default(),
            true,
        )
    }
    pub fn new(
        profiles: ProfileStore,
        discovery: Box<dyn DiscoveryBackend>,
        audio: Box<dyn AudioBackend>,
    ) -> Self {
        Self::new_internal(profiles, discovery, audio, EqManager::default(), false)
    }
    fn new_internal(
        profiles: ProfileStore,
        discovery: Box<dyn DiscoveryBackend>,
        audio: Box<dyn AudioBackend>,
        eq: EqManager,
        external_backends: bool,
    ) -> Self {
        let firmware_root = firmware_data_root();
        Self {
            hyperx_devices: vec![],
            all_devices: vec![],
            backend_statuses: vec![],
            discovered: vec![],
            profiles,
            discovery,
            audio,
            eq,
            mic_dsp: MicDspManager::default(),
            output_dsp_profile: load_output_dsp_profile(),
            output_dsp_generation: 0,
            firmware_stager: FirmwareStager::new(firmware_root.join("staging")),
            firmware_journal: FirmwareJournal::new(firmware_root.join("transactions")),
            staged_firmware: BTreeMap::new(),
            validated_firmware: BTreeMap::new(),
            openrgb: OpenRgbClient::default(),
            ratbag: RatbagClient,
            backend_services: BackendServiceManager,
            external_backends,
        }
    }

    pub fn refresh_devices(&mut self) -> Result<(), ForgeHxError> {
        let mut discovered = discover_devices(self.discovery.as_ref())
            .map_err(|e| ForgeHxError::Io(e.to_string()))?;
        let audio_result = self.audio.discover();
        let audio_nodes = audio_result.as_ref().cloned().unwrap_or_default();
        match &audio_result {
            Ok(nodes) => attach_audio_nodes(&mut discovered, nodes.clone()),
            Err(error) => warn!(%error, "audio discovery unavailable; keeping USB/HID inventory"),
        }

        let (openrgb_controllers, openrgb_health, openrgb_detail) = if self.external_backends {
            match self.openrgb.controllers_with_protocol() {
                Ok((values, protocol)) => (
                    values,
                    BackendHealth::Ready,
                    format!(
                        "OpenRGB SDK 127.0.0.1:6742 • server v{} • negotiated v{}",
                        protocol.server_version, protocol.negotiated_version
                    ),
                ),
                Err(error) => (
                    Vec::new(),
                    health_from_backend_error(&error),
                    format!("OpenRGB SDK 127.0.0.1:6742 • {error}"),
                ),
            }
        } else {
            (
                Vec::new(),
                BackendHealth::Unavailable,
                "OpenRGB compatibility backend disabled".into(),
            )
        };
        let (ratbag_devices, ratbag_health) = if self.external_backends {
            match self.ratbag.devices() {
                Ok(values) => (values, BackendHealth::Ready),
                Err(error) => (Vec::new(), health_from_backend_error(&error)),
            }
        } else {
            (Vec::new(), BackendHealth::Unavailable)
        };
        let source_node_ids = audio_nodes
            .iter()
            .filter(|node| node.kind == "source")
            .map(|node| node.id)
            .collect::<Vec<_>>();
        let linux_health = match &audio_result {
            Ok(_) => BackendHealth::Ready,
            Err(error) => BackendHealth::BackendError(error.to_string()),
        };
        self.backend_statuses = vec![
            BackendStatus {
                backend: BackendKind::ForgeHxNative,
                health: BackendHealth::Ready,
                detail: Some("verified device registry".into()),
            },
            BackendStatus {
                backend: BackendKind::ForgeHxDsp,
                health: linux_health.clone(),
                detail: Some("input-only PipeWire microphone processing".into()),
            },
            BackendStatus {
                backend: BackendKind::LinuxStandard,
                health: linux_health.clone(),
                detail: Some("PipeWire/WirePlumber + sysfs".into()),
            },
            BackendStatus {
                backend: BackendKind::OpenRgb,
                health: openrgb_health.clone(),
                detail: Some(openrgb_detail),
            },
            BackendStatus {
                backend: BackendKind::Ratbag,
                health: ratbag_health.clone(),
                detail: Some("ratbagctl / ratbagd D-Bus".into()),
            },
            BackendStatus {
                backend: BackendKind::Diagnostic,
                health: BackendHealth::Ready,
                detail: Some("read-only diagnostics".into()),
            },
        ];

        let openrgb_identities = openrgb_controllers
            .iter()
            .map(|controller| ExternalIdentity {
                id: controller.metadata.backend_device_id.clone(),
                name: controller.metadata.name.clone(),
                vendor: Some(controller.metadata.vendor.clone()).filter(|v| !v.is_empty()),
                serial: Some(controller.metadata.serial.clone()).filter(|v| !v.is_empty()),
                location: Some(controller.metadata.location.clone()).filter(|v| !v.is_empty()),
                vendor_id: None,
                product_id: None,
            })
            .collect::<Vec<_>>();
        let ratbag_identities = ratbag_devices
            .iter()
            .map(|device| {
                let (vid, pid) = device
                    .model
                    .as_deref()
                    .and_then(parse_ratbag_usb_model)
                    .unwrap_or((0, 0));
                ExternalIdentity {
                    id: device.id.clone(),
                    name: device.name.clone(),
                    vendor: vendor_name_from_audio(&device.name),
                    serial: None,
                    location: device.model.clone(),
                    vendor_id: (vid != 0).then_some(vid),
                    product_id: (pid != 0).then_some(pid),
                }
            })
            .collect::<Vec<_>>();

        for device in &mut discovered {
            assign_capability_owners(
                &mut device.info,
                &source_node_ids,
                &linux_health,
                &openrgb_health,
                &openrgb_identities,
                &ratbag_health,
                &ratbag_identities,
                &self.ratbag,
            );
            refresh_native_mouse_status(&mut device.info);
            normalize_device_support(&mut device.info);
        }

        let mut all_devices = discovered
            .iter()
            .map(|device| device.info.clone())
            .collect::<Vec<_>>();
        all_devices.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.0.cmp(&b.id.0)));
        self.restore_always_on_dsp(&all_devices, &audio_nodes);
        let hyperx_devices = all_devices
            .iter()
            .filter(|device| device.is_hyperx())
            .cloned()
            .collect();
        self.discovered = discovered;
        self.all_devices = all_devices;
        self.hyperx_devices = hyperx_devices;
        Ok(())
    }

    fn restore_always_on_dsp(&self, devices: &[DeviceInfo], audio_nodes: &[AudioNode]) {
        for device in devices
            .iter()
            .filter(|device| should_keep_forgehx_mic_source(device))
        {
            // Reuse the stable PipeWire node.name already discovered by ForgeHX. Do not
            // force a second wpctl-only lookup here because registry-only physical nodes
            // are valid direct-capture targets and are already part of coherent discovery.
            let source = device
                .selected_owner(Capability::MicDsp)
                .and_then(|owner| owner.backend_device_id.as_deref())
                .and_then(|value| value.parse::<u32>().ok())
                .and_then(|node_id| {
                    audio_nodes
                        .iter()
                        .find(|node| node.id == node_id && node.kind == "source")
                });

            let runtime_ready = match source {
                Some(node) => self
                    .mic_dsp
                    .ensure_always_on_with_source_name(&device.id, &node.name)
                    .map(|_| ()),
                None => self.mic_dsp.ensure_direct_source(&device.id),
            };
            if let Err(error) = runtime_ready {
                warn!(device=%device.id.0, %error, "ForgeHX direct microphone runtime is waiting for its physical HyperX capture target");
                continue;
            }
        }
    }

    pub fn handle(&mut self, command: Command) -> Reply {
        let client_version = command.protocol_version();
        let required_version = command
            .minimum_protocol_version()
            .max(IPC_MIN_PROTOCOL_VERSION);
        if !matches!(&command, Command::Ping { .. })
            && (client_version < required_version || client_version > IPC_PROTOCOL_VERSION)
        {
            return Reply::error(
                "protocol_version",
                format!(
                    "protocol version mismatch: daemon={}, client={}, supported={}..={}, command_requires={}",
                    IPC_PROTOCOL_VERSION, client_version, IPC_MIN_PROTOCOL_VERSION, IPC_PROTOCOL_VERSION, required_version
                ),
            );
        }
        match command {
            Command::Ping { .. } => Reply::Pong {
                protocol_version: IPC_PROTOCOL_VERSION,
                min_protocol_version: IPC_MIN_PROTOCOL_VERSION,
            },
            Command::DaemonStatus { .. } => Reply::Status {
                protocol_version: IPC_PROTOCOL_VERSION,
                device_count: self.hyperx_devices.len(),
                all_device_count: self.all_devices.len(),
                write_protected_count: self
                    .all_devices
                    .iter()
                    .filter(|d| !d.capability_owners.iter().any(|o| o.writable))
                    .count(),
            },
            Command::ListDevices { .. } | Command::ListHyperxDevices { .. } => Reply::Devices {
                devices: project_devices_for_protocol(&self.hyperx_devices, client_version),
            },
            Command::ListAllDevices { .. } => Reply::Devices {
                devices: project_devices_for_protocol(&self.all_devices, client_version),
            },
            Command::RefreshDevices { .. } => self.refresh_reply("device inventory refreshed"),
            Command::Doctor { device_id, .. } => {
                let mut reports = doctor_reports(&self.discovered);
                if let Some(id) = device_id {
                    reports.retain(|r| r.device_id == id);
                }
                Reply::Doctor { reports }
            }
            Command::ListAudio { .. } => match self.audio.discover() {
                Ok(nodes) => Reply::AudioNodes { nodes },
                Err(e) => error_reply(e),
            },
            Command::BackendStatus { .. } => Reply::BackendStatuses {
                backends: project_backends_for_protocol(&self.backend_statuses, client_version),
            },
            Command::BackendRescan { .. } => self.backend_rescan(),
            Command::BackendEnsure { .. } => self.backend_ensure(),
            Command::BackendRestart { backend, .. } => self.backend_restart(backend),
            Command::ProfileList { .. } => match self.profiles.list() {
                Ok(names) => Reply::Profiles { names },
                Err(e) => error_reply(e),
            },
            Command::ProfileGet { name, .. } => match self.profiles.load(&name) {
                Ok(profile) => Reply::Profile { profile },
                Err(e) => error_reply(e),
            },
            Command::ProfileSave { profile, .. } => match self.profiles.save(&profile) {
                Ok(()) => Reply::Ok {
                    message: format!("saved profile {}", profile.name),
                },
                Err(e) => error_reply(e),
            },
            Command::ProfileApply {
                name, device_id, ..
            } => self.apply_profile(&name, &device_id),
            Command::LightingMetadata { device_id, .. } => self.lighting_metadata(&device_id),
            Command::SetLighting {
                device_id, config, ..
            } => self.set_lighting(&device_id, &config),
            Command::MouseState { device_id, .. } => self.mouse_state(&device_id),
            Command::MouseCapabilities { device_id, .. } => self.mouse_capabilities(&device_id),
            Command::KeyboardCapabilities { device_id, .. } => {
                self.keyboard_capabilities(&device_id)
            }
            Command::SetDpi {
                device_id, config, ..
            } => self.set_dpi(&device_id, &config),
            Command::SetPollingRate { device_id, hz, .. } => self.set_polling_rate(&device_id, hz),
            Command::SetMouseProfile {
                device_id, profile, ..
            } => self.set_mouse_profile(&device_id, profile),
            Command::SetButtonAssignment {
                device_id,
                button,
                action,
                ..
            } => self.set_button_assignment(&device_id, button, &action),
            Command::SetLiftOffDistance { device_id, mm, .. } => {
                self.set_lift_off_distance(&device_id, mm)
            }
            Command::AudioSetVolume {
                node_id, volume, ..
            } => match self.audio.set_volume(node_id, volume) {
                Ok(()) => Reply::Ok {
                    message: format!(
                        "set PipeWire node {node_id} volume to {:.0}%",
                        volume * 100.0
                    ),
                },
                Err(e) => error_reply(e),
            },
            Command::AudioSetMute { node_id, muted, .. } => {
                match self.audio.set_mute(node_id, muted) {
                    Ok(()) => Reply::Ok {
                        message: format!("set PipeWire node {node_id} mute={muted}"),
                    },
                    Err(e) => error_reply(e),
                }
            }
            Command::EqList { .. } => match self.eq.list() {
                Ok(names) => Reply::EqProfiles { names },
                Err(e) => error_reply(e),
            },
            Command::EqGet { name, .. } => match self.eq.get(&name) {
                Ok(config) => Reply::EqProfile { config },
                Err(e) => error_reply(e),
            },
            Command::EqSave { config, .. } => match self.eq.save(&config) {
                Ok(()) => Reply::Ok {
                    message: format!("saved EQ {}", config.name),
                },
                Err(e) => error_reply(e),
            },
            Command::EqDelete { name, .. } => match self.eq.delete(&name) {
                Ok(()) => Reply::Ok {
                    message: format!("deleted EQ {name}"),
                },
                Err(e) => error_reply(e),
            },
            Command::EqApply { name, .. } => match self.eq.apply(&name) {
                Ok(()) => Reply::Ok {
                    message: format!("applied EQ {name}"),
                },
                Err(e) => error_reply(e),
            },
            Command::EqBypass { .. } => match self.eq.bypass() {
                Ok(()) => Reply::Ok {
                    message: "ForgeHX EQ bypassed".into(),
                },
                Err(e) => error_reply(e),
            },
            Command::MicDspGet { device_id, .. } => self.mic_dsp_get(&device_id),
            Command::MicDspSave {
                device_id, config, ..
            } => self.mic_dsp_save(&device_id, &config),
            Command::MicDspLiveUpdate {
                device_id, config, ..
            } => self.mic_dsp_live_update(&device_id, &config),
            Command::MicDspApply {
                device_id, name, ..
            } => self.mic_dsp_apply(&device_id, &name),
            Command::MicDspBypass { device_id, .. } => self.mic_dsp_bypass(&device_id),
            Command::MicVoiceEnrollStart { device_id, .. } => {
                self.mic_voice_enroll_start(&device_id)
            }
            Command::MicVoiceEnrollCancel { device_id, .. } => {
                self.mic_voice_enroll_cancel(&device_id)
            }
            Command::MicVoiceForget { device_id, .. } => self.mic_voice_forget(&device_id),
            Command::MicMonitorSet {
                device_id,
                enabled,
                level_percent,
                ..
            } => self.mic_monitor_set(&device_id, enabled, level_percent),
            Command::MicFirmwareGet { device_id, .. } => self.mic_firmware_get(&device_id),
            Command::MicFirmwareStage {
                device_id, path, ..
            } => self.mic_firmware_stage(&device_id, &path),
            Command::MicFirmwareValidate {
                device_id,
                staged_id,
                ..
            } => self.mic_firmware_validate(&device_id, &staged_id),
            Command::MicFirmwareBegin {
                device_id,
                staged_id,
                ..
            } => self.mic_firmware_begin(&device_id, &staged_id),
            Command::MicFirmwareStatus { transaction_id, .. } => {
                self.mic_firmware_status(&transaction_id)
            }
            Command::MicFirmwareForget { staged_id, .. } => self.mic_firmware_forget(&staged_id),
            Command::OutputDspGet { .. } => self.output_dsp_get(),
            Command::OutputDspLiveUpdate { profile, .. } => self.output_dsp_live_update(profile),
            Command::OutputDspResetFactory { device_class, .. } => {
                self.output_dsp_reset_factory(device_class)
            }
        }
    }

    fn output_dsp_get(&mut self) -> Reply {
        let status = OutputDspClient::default().status();
        match status {
            Ok(status) => Reply::OutputDspState {
                profile: self.output_dsp_profile.clone(),
                live: status.live,
                generation: status.generation,
                target_device_id: status
                    .active_device_id
                    .unwrap_or_else(|| DEFAULT_OUTPUT_DSP_TARGET.into()),
            },
            Err(_) => Reply::OutputDspState {
                profile: self.output_dsp_profile.clone(),
                live: false,
                generation: self.output_dsp_generation,
                target_device_id: DEFAULT_OUTPUT_DSP_TARGET.into(),
            },
        }
    }

    fn output_dsp_live_update(&mut self, profile: OutputDspProfile) -> Reply {
        if let Err(error) = profile.validate(48_000.0, 2) {
            return Reply::error("output_dsp_invalid", error.to_string());
        }
        if let Err(error) = save_output_dsp_profile(&profile) {
            return error_reply(error);
        }
        match OutputDspClient::default().apply(profile.clone()) {
            Ok(generation) => {
                self.output_dsp_profile = profile.clone();
                self.output_dsp_generation = generation;
                Reply::OutputDspState {
                    profile,
                    live: true,
                    generation,
                    target_device_id: DEFAULT_OUTPUT_DSP_TARGET.into(),
                }
            }
            Err(error) => {
                self.output_dsp_profile = profile;
                Reply::error("aetherstream_output_dsp", error)
            }
        }
    }

    fn output_dsp_reset_factory(&mut self, device_class: OutputDeviceClass) -> Reply {
        self.output_dsp_live_update(OutputDspProfile::factory(device_class))
    }

    fn refresh_reply(&mut self, prefix: &str) -> Reply {
        match self.refresh_devices() {
            Ok(()) => Reply::Ok {
                message: format!(
                    "{prefix}: {} HyperX / {} total",
                    self.hyperx_devices.len(),
                    self.all_devices.len()
                ),
            },
            Err(e) => error_reply(e),
        }
    }
    fn backend_rescan(&mut self) -> Reply {
        let mut notes = Vec::new();
        if self.external_backends {
            if let Err(e) = self.openrgb.rescan() {
                notes.push(format!("OpenRGB: {e}"));
            }
        }
        match self.refresh_devices() {
            Ok(()) => Reply::Ok {
                message: if notes.is_empty() {
                    "backends rescanned".into()
                } else {
                    format!("backends refreshed with notes: {}", notes.join("; "))
                },
            },
            Err(e) => error_reply(e),
        }
    }
    fn backend_ensure(&mut self) -> Reply {
        if !self.external_backends {
            return Reply::error(
                "backend_unavailable",
                "external control backends are disabled",
            );
        }
        let (ready, errors) = self.backend_services.ensure_all();
        let refresh = self.refresh_devices();
        if let Err(error) = refresh {
            return error_reply(error);
        }
        let mut message = if ready.is_empty() {
            "backend activation requested".to_owned()
        } else {
            ready.join("; ")
        };
        if !errors.is_empty() {
            message.push_str(&format!("; notes: {}", errors.join("; ")));
        }
        Reply::Ok { message }
    }
    fn backend_restart(&mut self, backend: BackendKind) -> Reply {
        if !self.external_backends
            && !matches!(
                backend,
                BackendKind::ForgeHxNative | BackendKind::ForgeHxDsp | BackendKind::Diagnostic
            )
        {
            return Reply::error(
                "backend_unavailable",
                "external control backends are disabled",
            );
        }
        match self.backend_services.restart(backend) {
            Ok(message) => match self.refresh_devices() {
                Ok(()) => Reply::Ok { message },
                Err(error) => error_reply(error),
            },
            Err(error) => error_reply(backend_error(error)),
        }
    }
    fn mic_dsp_get(&self, id: &DeviceId) -> Reply {
        if let Err(error) = self.owner(id, Capability::MicDsp) {
            return error_reply(error);
        }
        match self.mic_dsp.state(id) {
            Ok(state) => Reply::MicDspState {
                state: Box::new(state),
            },
            Err(error) => error_reply(error),
        }
    }
    fn mic_dsp_save(&self, id: &DeviceId, config: &forgehx_core::MicrophoneDspConfig) -> Reply {
        if let Err(error) = self.owner(id, Capability::MicDsp) {
            return error_reply(error);
        }
        match self.mic_dsp.save(config) {
            Ok(()) => Reply::Ok {
                message: format!("saved microphone DSP profile {}", config.name),
            },
            Err(error) => error_reply(error),
        }
    }
    fn mic_dsp_live_update(
        &self,
        id: &DeviceId,
        config: &forgehx_core::MicrophoneDspConfig,
    ) -> Reply {
        if let Err(error) = self.owner(id, Capability::MicDsp) {
            return error_reply(error);
        }
        match self.mic_dsp.live_update(id, config) {
            Ok(state) => Reply::MicDspState {
                state: Box::new(state),
            },
            Err(error) => error_reply(error),
        }
    }

    fn mic_dsp_apply(&self, id: &DeviceId, name: &str) -> Reply {
        if let Err(error) = self.owner(id, Capability::MicDsp) {
            return error_reply(error);
        }
        let raw_source_target = match self.live_microphone_source_target(id) {
            Ok(value) => value,
            Err(error) => return error_reply(error),
        };
        match self
            .mic_dsp
            .apply_with_source_name(id, name, &raw_source_target)
        {
            Ok(state) => Reply::MicDspState {
                state: Box::new(state),
            },
            Err(error) => error_reply(error),
        }
    }
    fn mic_dsp_bypass(&self, id: &DeviceId) -> Reply {
        if let Err(error) = self.owner(id, Capability::MicDsp) {
            return error_reply(error);
        }
        match self.mic_dsp.bypass(id).and_then(|_| self.mic_dsp.state(id)) {
            Ok(state) => Reply::MicDspState {
                state: Box::new(state),
            },
            Err(error) => error_reply(error),
        }
    }
    fn mic_voice_enroll_start(&self, id: &DeviceId) -> Reply {
        if let Err(error) = self.owner(id, Capability::MicDsp) {
            return error_reply(error);
        }
        match self.mic_dsp.start_voice_enrollment(id) {
            Ok(state) => Reply::MicDspState {
                state: Box::new(state),
            },
            Err(error) => error_reply(error),
        }
    }
    fn mic_voice_enroll_cancel(&self, id: &DeviceId) -> Reply {
        if let Err(error) = self.owner(id, Capability::MicDsp) {
            return error_reply(error);
        }
        match self.mic_dsp.cancel_voice_enrollment(id) {
            Ok(state) => Reply::MicDspState {
                state: Box::new(state),
            },
            Err(error) => error_reply(error),
        }
    }
    fn mic_voice_forget(&self, id: &DeviceId) -> Reply {
        if let Err(error) = self.owner(id, Capability::MicDsp) {
            return error_reply(error);
        }
        match self.mic_dsp.forget_voice(id) {
            Ok(state) => Reply::MicDspState {
                state: Box::new(state),
            },
            Err(error) => error_reply(error),
        }
    }
    fn mic_monitor_set(&self, id: &DeviceId, enabled: bool, level_percent: f32) -> Reply {
        if let Err(error) = self.owner(id, Capability::MicDsp) {
            return error_reply(error);
        }
        let target = if enabled {
            let nodes = match self.audio.discover() {
                Ok(nodes) => nodes,
                Err(error) => return error_reply(error),
            };
            match select_wired_monitor_sink(&nodes) {
                Some(node) => Some(node.name),
                None => return Reply::error("audio_unavailable", "No wired analog/headphone PipeWire sink is available; ForgeHX will not route mic monitoring to Bluetooth"),
            }
        } else {
            None
        };
        match self.mic_dsp.set_monitor(id, enabled, target, level_percent) {
            Ok(state) => Reply::MicDspState {
                state: Box::new(state),
            },
            Err(error) => error_reply(error),
        }
    }
    fn mic_firmware_identity(&self, id: &DeviceId) -> Result<FirmwareIdentity, Reply> {
        let device = self.device(id).map_err(error_reply)?;
        let adapter = FirmwareRegistry::for_device(device).ok_or_else(|| {
            Reply::error(
                "firmware_unsupported",
                "no HyperX microphone firmware adapter matches this device",
            )
        })?;
        Ok(FirmwareIdentity {
            device_id: device.id.clone(),
            model: adapter.model.into(),
            hardware_revision: None,
            firmware_version: None,
            firmware_version_source: None,
            bootloader_version: None,
            support_level: adapter.support_level,
        })
    }

    fn mic_firmware_get(&self, id: &DeviceId) -> Reply {
        match self.mic_firmware_identity(id) {
            Ok(identity) => Reply::FirmwareIdentity { identity },
            Err(reply) => reply,
        }
    }

    fn mic_firmware_stage(&mut self, id: &DeviceId, path: &str) -> Reply {
        if let Err(reply) = self.mic_firmware_identity(id) {
            return reply;
        }
        match self.firmware_stager.stage(id, &PathBuf::from(path)) {
            Ok(staged) => {
                let device = match self.device(id) {
                    Ok(device) => device,
                    Err(error) => return error_reply(error),
                };
                let adapter = match FirmwareRegistry::for_device(device) {
                    Some(value) => value,
                    None => {
                        return Reply::error(
                            "firmware_unsupported",
                            "firmware adapter disappeared during staging",
                        )
                    }
                };
                let package = validate_package(&adapter, &staged);
                self.validated_firmware
                    .insert(staged.staged_id.clone(), package.clone());
                self.staged_firmware
                    .insert(staged.staged_id.clone(), staged);
                Reply::FirmwarePackage { package }
            }
            Err(error) => firmware_error_reply(error),
        }
    }

    fn mic_firmware_validate(&mut self, id: &DeviceId, staged_id: &str) -> Reply {
        let device = match self.device(id) {
            Ok(device) => device.clone(),
            Err(error) => return error_reply(error),
        };
        let adapter = match FirmwareRegistry::for_device(&device) {
            Some(value) => value,
            None => {
                return Reply::error(
                    "firmware_unsupported",
                    "no HyperX microphone firmware adapter matches this device",
                )
            }
        };
        let staged = match self.staged_firmware.get(staged_id) {
            Some(value) if &value.device_id == id => value.clone(),
            Some(_) => {
                return Reply::error(
                    "firmware_target_mismatch",
                    "staged firmware belongs to another device",
                )
            }
            None => return Reply::error("firmware_stage_missing", "staged firmware was not found"),
        };
        let package = validate_package(&adapter, &staged);
        self.validated_firmware
            .insert(staged_id.to_owned(), package.clone());
        Reply::FirmwarePackage { package }
    }

    fn mic_firmware_begin(&mut self, id: &DeviceId, staged_id: &str) -> Reply {
        let device = match self.device(id) {
            Ok(device) => device.clone(),
            Err(error) => return error_reply(error),
        };
        let adapter = match FirmwareRegistry::for_device(&device) {
            Some(value) => value,
            None => {
                return Reply::error(
                    "firmware_unsupported",
                    "no HyperX microphone firmware adapter matches this device",
                )
            }
        };
        if !adapter.support_level.can_update() {
            return Reply::error("firmware_update_unavailable", format!("{} firmware support is inventory-only; no verified update transport is enabled", adapter.model));
        }
        let staged = match self.staged_firmware.get(staged_id) {
            Some(value) if value.device_id == *id => value,
            Some(_) => {
                return Reply::error(
                    "firmware_target_mismatch",
                    "staged firmware belongs to another device",
                )
            }
            None => return Reply::error("firmware_stage_missing", "staged firmware was not found"),
        };
        let package = self
            .validated_firmware
            .get(staged_id)
            .cloned()
            .unwrap_or_else(|| validate_package(&adapter, staged));
        if package.validation != FirmwareValidation::Valid {
            return Reply::error(
                "firmware_package_not_valid",
                format!("firmware package validation is {:?}", package.validation),
            );
        }
        Reply::error(
            "firmware_update_unavailable",
            "adapter is marked update-capable but no verified writer is installed",
        )
    }

    fn mic_firmware_status(&self, transaction_id: &str) -> Reply {
        match self.firmware_journal.load(transaction_id) {
            Ok(status) => Reply::FirmwareTransaction { status },
            Err(error) => firmware_error_reply(error),
        }
    }

    fn mic_firmware_forget(&mut self, staged_id: &str) -> Reply {
        let Some(staged) = self.staged_firmware.remove(staged_id) else {
            return Reply::error("firmware_stage_missing", "staged firmware was not found");
        };
        self.validated_firmware.remove(staged_id);
        match self.firmware_stager.forget(&staged) {
            Ok(()) => Reply::Ok {
                message: format!("forgot staged firmware {staged_id}"),
            },
            Err(error) => firmware_error_reply(error),
        }
    }

    fn live_microphone_source_target(&self, id: &DeviceId) -> Result<String, ForgeHxError> {
        let device = self.device(id)?;
        let associated = device
            .interfaces
            .iter()
            .filter_map(|interface| interface.audio_node_id)
            .collect::<Vec<_>>();
        if associated.is_empty() {
            return Err(ForgeHxError::AudioUnavailable(
                "HyperX microphone has no associated PipeWire source".into(),
            ));
        }
        let live = self.audio.discover()?;
        live.into_iter()
            .find(|node| node.kind == "source" && associated.contains(&node.id))
            .map(|node| node.name)
            .ok_or_else(|| {
                ForgeHxError::AudioUnavailable(
                    "associated HyperX microphone source is no longer live".into(),
                )
            })
    }

    fn device(&self, id: &DeviceId) -> Result<&DeviceInfo, ForgeHxError> {
        self.all_devices
            .iter()
            .find(|d| &d.id == id)
            .ok_or_else(|| ForgeHxError::DeviceNotFound(id.clone()))
    }
    fn owner(
        &self,
        id: &DeviceId,
        capability: Capability,
    ) -> Result<CapabilityOwner, ForgeHxError> {
        let device = self.device(id)?;
        device
            .selected_owner(capability)
            .cloned()
            .ok_or(ForgeHxError::Unsupported(capability))
    }
    fn lighting_metadata(&self, id: &DeviceId) -> Reply {
        let owner = match self.owner(id, Capability::Lighting) {
            Ok(o) => o,
            Err(e) => return error_reply(e),
        };
        match owner.backend {
            BackendKind::OpenRgb => {
                let Some(index) = owner
                    .backend_device_id
                    .as_deref()
                    .and_then(|v| v.parse::<u32>().ok())
                else {
                    return Reply::error("backend_protocol", "invalid OpenRGB controller mapping");
                };
                match self.openrgb.controllers() {
                    Ok(values) => Reply::LightingController {
                        metadata: values
                            .into_iter()
                            .find(|c| c.metadata.backend_device_id == index.to_string())
                            .map(|c| c.metadata),
                    },
                    Err(e) => error_reply(backend_error(e)),
                }
            }
            BackendKind::ForgeHxNative => {
                let device = match self.device(id) {
                    Ok(device) => device,
                    Err(error) => return error_reply(error),
                };
                if device.protocol.as_deref() == Some(PULSEFIRE_HASTE_DRIVER_ID) {
                    let location = device
                        .interfaces
                        .iter()
                        .find(|interface| {
                            interface.interface_number == Some(2)
                                && interface.source == InterfaceSource::Hid
                        })
                        .map(|interface| interface.path.clone())
                        .unwrap_or_else(|| "native HID interface 2".into());
                    Reply::LightingController {
                        metadata: Some(forgehx_core::LightingControllerMetadata {
                            backend_device_id: PULSEFIRE_HASTE_DRIVER_ID.into(),
                            name: "HyperX Pulsefire Haste Wireless".into(),
                            vendor: "HyperX".into(),
                            description: "ForgeHX Native single-logo RGB with onboard persistence"
                                .into(),
                            version: "Haste v1 64-byte HID".into(),
                            serial: device.serial.clone().unwrap_or_default(),
                            location,
                            led_count: 1,
                            modes: vec![
                                forgehx_core::LightingModeMetadata {
                                    index: 0,
                                    name: "Static".into(),
                                    speed_min: 0,
                                    speed_max: 100,
                                    brightness_min: Some(0),
                                    brightness_max: Some(100),
                                },
                                forgehx_core::LightingModeMetadata {
                                    index: 1,
                                    name: "Breathing".into(),
                                    speed_min: 0,
                                    speed_max: 100,
                                    brightness_min: Some(0),
                                    brightness_max: Some(100),
                                },
                                forgehx_core::LightingModeMetadata {
                                    index: 2,
                                    name: "Spectrum".into(),
                                    speed_min: 0,
                                    speed_max: 100,
                                    brightness_min: Some(0),
                                    brightness_max: Some(100),
                                },
                            ],
                            zones: vec![forgehx_core::LightingZoneMetadata {
                                index: 0,
                                name: "Logo".into(),
                                led_count: 1,
                            }],
                            protocol_version: IPC_PROTOCOL_VERSION,
                        }),
                    }
                } else {
                    Reply::LightingController { metadata: None }
                }
            }
            _ => error_reply(ForgeHxError::Unsupported(Capability::Lighting)),
        }
    }
    fn set_lighting(&self, id: &DeviceId, config: &forgehx_core::LightingConfig) -> Reply {
        let owner = match self.owner(id, Capability::Lighting) {
            Ok(o) => o,
            Err(e) => return error_reply(e),
        };
        if !owner.writable {
            return Reply::error("unsupported", "selected lighting backend is read-only");
        }
        match owner.backend {
            BackendKind::OpenRgb => {
                let Some(index) = owner
                    .backend_device_id
                    .as_deref()
                    .and_then(|v| v.parse::<u32>().ok())
                else {
                    return Reply::error("backend_protocol", "invalid OpenRGB controller mapping");
                };
                match self.openrgb.set_lighting(index, config) {
                    Ok(()) => Reply::Ok {
                        message: format!("lighting applied through {}", owner.backend),
                    },
                    Err(e) => error_reply(backend_error(e)),
                }
            }
            BackendKind::ForgeHxNative => {
                let device = match self.device(id) {
                    Ok(device) => device,
                    Err(error) => return error_reply(error),
                };
                match device.protocol.as_deref() {
                    Some(PULSEFIRE_HASTE_DRIVER_ID) => match PulsefireHasteWireless::from_device(device).and_then(|mouse| mouse.set_lighting(config)) {
                        Ok(()) => Reply::Ok { message: "Pulsefire Haste Wireless lighting updated and saved through ForgeHX Native".into() },
                        Err(error) => error_reply(mouse_error(error)),
                    },
                    Some(SAGA_PRO_DRIVER_ID) => match SagaPro::from_device(device).and_then(|mouse| mouse.set_lighting(config)) {
                        Ok(()) => Reply::Ok { message: "Pulsefire Saga Pro static lighting updated through ForgeHX Native".into() },
                        Err(error) => error_reply(mouse_error(error)),
                    },
                    _ => Reply::error("protocol_unimplemented", "native lighting writer is not implemented for this driver"),
                }
            }
            _ => error_reply(ForgeHxError::Unsupported(Capability::Lighting)),
        }
    }
    fn mouse_capabilities(&self, id: &DeviceId) -> Reply {
        let device = match self.device(id) {
            Ok(device) => device,
            Err(error) => return error_reply(error),
        };
        let model = model_for(device.vendor_id, device.product_id, &device.name)
            .map(|matched| matched.info());
        Reply::MouseCapabilities { model }
    }

    fn keyboard_capabilities(&self, id: &DeviceId) -> Reply {
        let device = match self.device(id) {
            Ok(device) => device,
            Err(error) => return error_reply(error),
        };
        if device.device_class != DeviceClass::Keyboard {
            return Reply::error("not_keyboard", "device is not classified as a keyboard");
        }
        let model =
            keyboard_model_for(device.vendor_id, device.product_id, &device.name).map(|matched| {
                let protocol_family = match matched.definition.protocol_family {
                    RegistryKeyboardProtocolFamily::AlloyLegacy => {
                        CoreKeyboardProtocolFamily::AlloyLegacy
                    }
                    RegistryKeyboardProtocolFamily::AlloyOrigins => {
                        CoreKeyboardProtocolFamily::AlloyOrigins
                    }
                    RegistryKeyboardProtocolFamily::AlloyRise => {
                        CoreKeyboardProtocolFamily::AlloyRise
                    }
                    RegistryKeyboardProtocolFamily::Origins2 => {
                        CoreKeyboardProtocolFamily::Origins2
                    }
                    RegistryKeyboardProtocolFamily::Eve => CoreKeyboardProtocolFamily::Eve,
                    RegistryKeyboardProtocolFamily::Unknown => CoreKeyboardProtocolFamily::Unknown,
                };
                let lighting = match matched.limits.lighting {
                    RegistryKeyboardLightingTopology::None => CoreKeyboardLightingTopology::None,
                    RegistryKeyboardLightingTopology::Zone(zones) => {
                        CoreKeyboardLightingTopology::Zone { zones }
                    }
                    RegistryKeyboardLightingTopology::PerKey => {
                        CoreKeyboardLightingTopology::PerKey
                    }
                };
                KeyboardModelInfo {
                    id: matched.definition.id.into(),
                    name: matched.definition.name.into(),
                    protocol_family,
                    limits: CoreKeyboardLimits {
                        lighting,
                        polling_rates_hz: matched.limits.polling_rates_hz,
                        onboard_profiles: matched.limits.onboard_profiles,
                        wireless: matched.limits.wireless,
                        battery: matched.limits.battery,
                        hall_effect: matched.limits.hall_effect,
                        rapid_trigger: matched.limits.rapid_trigger,
                    },
                    exact_hardware_match: matched.exact_hardware_match,
                    native_driver: matched.definition.native_driver.map(str::to_owned),
                    complete: matched.definition.complete,
                }
            });
        Reply::KeyboardCapabilities { model }
    }

    fn mouse_state(&self, id: &DeviceId) -> Reply {
        let owner = self
            .owner(id, Capability::Dpi)
            .or_else(|_| self.owner(id, Capability::PollingRate))
            .or_else(|_| self.owner(id, Capability::Profiles));
        let owner = match owner {
            Ok(o) => o,
            Err(e) => return error_reply(e),
        };
        match owner.backend {
            BackendKind::Ratbag => match owner.backend_device_id {
                Some(device) => match self.ratbag.state(&device) {
                    Ok(state) => Reply::MouseState { state },
                    Err(e) => error_reply(backend_error(e)),
                },
                None => Reply::error("backend_protocol", "missing ratbag device mapping"),
            },
            BackendKind::ForgeHxNative => {
                let device = match self.device(id) {
                    Ok(device) => device,
                    Err(error) => return error_reply(error),
                };
                if device.protocol.as_deref() == Some(PULSEFIRE_HASTE_DRIVER_ID) {
                    match PulsefireHasteWireless::from_device(device)
                        .and_then(|mouse| mouse.state())
                    {
                        Ok(state) => Reply::MouseState { state },
                        Err(error) => error_reply(mouse_error(error)),
                    }
                } else {
                    Reply::error(
                        "protocol_unimplemented",
                        "native mouse state is not implemented for this driver",
                    )
                }
            }
            _ => error_reply(ForgeHxError::Unsupported(Capability::Dpi)),
        }
    }
    fn set_dpi(&self, id: &DeviceId, config: &forgehx_core::DpiConfig) -> Reply {
        if config.stages.is_empty() || config.active_stage >= config.stages.len() {
            return Reply::error("profile", "invalid DPI stages");
        }
        let owner = match self.owner(id, Capability::Dpi) {
            Ok(o) => o,
            Err(e) => return error_reply(e),
        };
        match owner.backend {
            BackendKind::Ratbag => {
                let Some(device) = owner.backend_device_id else {
                    return Reply::error("backend_protocol", "missing ratbag mapping");
                };
                let state = match self.ratbag.state(&device) {
                    Ok(s) => s,
                    Err(e) => return error_reply(backend_error(e)),
                };
                let profile = state.active_profile.unwrap_or(0);
                for (i, dpi) in config.stages.iter().enumerate() {
                    if let Err(e) = self.ratbag.set_dpi(&device, profile, i, *dpi) {
                        return error_reply(backend_error(e));
                    }
                }
                match self
                    .ratbag
                    .set_active_resolution(&device, profile, config.active_stage)
                {
                    Ok(()) => Reply::Ok {
                        message: "DPI updated through libratbag".into(),
                    },
                    Err(e) => error_reply(backend_error(e)),
                }
            }
            BackendKind::ForgeHxNative => {
                let device = match self.device(id) {
                    Ok(device) => device,
                    Err(error) => return error_reply(error),
                };
                if device.protocol.as_deref() != Some(PULSEFIRE_HASTE_DRIVER_ID) {
                    return Reply::error(
                        "protocol_unimplemented",
                        "native DPI writer is not implemented for this driver",
                    );
                }
                match PulsefireHasteWireless::from_device(device)
                    .and_then(|mouse| mouse.set_dpi(config))
                {
                    Ok(()) => Reply::Ok {
                        message: "Pulsefire Haste DPI updated through ForgeHX Native".into(),
                    },
                    Err(error) => error_reply(mouse_error(error)),
                }
            }
            _ => error_reply(ForgeHxError::Unsupported(Capability::Dpi)),
        }
    }
    fn set_polling_rate(&self, id: &DeviceId, hz: u16) -> Reply {
        let owner = match self.owner(id, Capability::PollingRate) {
            Ok(o) => o,
            Err(e) => return error_reply(e),
        };
        match owner.backend {
            BackendKind::Ratbag => {
                let Some(device) = owner.backend_device_id else {
                    return Reply::error("backend_protocol", "missing ratbag mapping");
                };
                let profile = self
                    .ratbag
                    .state(&device)
                    .ok()
                    .and_then(|s| s.active_profile)
                    .unwrap_or(0);
                match self.ratbag.set_report_rate(&device, profile, hz) {
                    Ok(()) => Reply::Ok {
                        message: format!("polling rate set to {hz}Hz"),
                    },
                    Err(e) => error_reply(backend_error(e)),
                }
            }
            BackendKind::ForgeHxNative => {
                let device = match self.device(id) {
                    Ok(device) => device,
                    Err(error) => return error_reply(error),
                };
                if device.protocol.as_deref() != Some(PULSEFIRE_HASTE_DRIVER_ID) {
                    return Reply::error(
                        "protocol_unimplemented",
                        "native polling-rate writer is not implemented for this driver",
                    );
                }
                match PulsefireHasteWireless::from_device(device)
                    .and_then(|mouse| mouse.set_polling_rate(hz))
                {
                    Ok(()) => Reply::Ok {
                        message: format!(
                            "Pulsefire Haste polling rate set to {hz} Hz through ForgeHX Native"
                        ),
                    },
                    Err(error) => error_reply(mouse_error(error)),
                }
            }
            _ => error_reply(ForgeHxError::Unsupported(Capability::PollingRate)),
        }
    }
    fn set_mouse_profile(&self, id: &DeviceId, profile: u8) -> Reply {
        let owner = match self.owner(id, Capability::Profiles) {
            Ok(o) => o,
            Err(e) => return error_reply(e),
        };
        match owner.backend {
            BackendKind::Ratbag => match owner.backend_device_id {
                Some(device) => match self.ratbag.set_profile(&device, profile) {
                    Ok(()) => Reply::Ok {
                        message: format!("mouse profile {profile} activated"),
                    },
                    Err(e) => error_reply(backend_error(e)),
                },
                None => Reply::error("backend_protocol", "missing ratbag mapping"),
            },
            BackendKind::ForgeHxNative => {
                let device = match self.device(id) {
                    Ok(device) => device,
                    Err(error) => return error_reply(error),
                };
                if device.protocol.as_deref() != Some(PULSEFIRE_HASTE_DRIVER_ID) {
                    return Reply::error(
                        "protocol_unimplemented",
                        "native profile writer is not implemented for this driver",
                    );
                }
                match PulsefireHasteWireless::from_device(device)
                    .and_then(|mouse| mouse.save_onboard_profile(profile))
                {
                    Ok(()) => Reply::Ok {
                        message: "Pulsefire Haste Wireless onboard profile saved".into(),
                    },
                    Err(error) => error_reply(mouse_error(error)),
                }
            }
            _ => error_reply(ForgeHxError::Unsupported(Capability::Profiles)),
        }
    }
    fn set_button_assignment(&self, id: &DeviceId, button: u32, action: &str) -> Reply {
        let owner = match self.owner(id, Capability::Bindings) {
            Ok(o) => o,
            Err(e) => return error_reply(e),
        };
        match owner.backend {
            BackendKind::Ratbag => {
                let Some(device) = owner.backend_device_id else {
                    return Reply::error("backend_protocol", "missing ratbag mapping");
                };
                let profile = self
                    .ratbag
                    .state(&device)
                    .ok()
                    .and_then(|s| s.active_profile)
                    .unwrap_or(0);
                match self
                    .ratbag
                    .set_button_action(&device, profile, button, action)
                {
                    Ok(()) => Reply::Ok {
                        message: format!("button {button} assignment updated"),
                    },
                    Err(e) => error_reply(backend_error(e)),
                }
            }
            BackendKind::ForgeHxNative => {
                let device = match self.device(id) {
                    Ok(device) => device,
                    Err(error) => return error_reply(error),
                };
                if device.protocol.as_deref() != Some(PULSEFIRE_HASTE_DRIVER_ID) {
                    return Reply::error(
                        "protocol_unimplemented",
                        "native button writer is not implemented for this driver",
                    );
                }
                match PulsefireHasteWireless::from_device(device).and_then(|mouse|mouse.set_button_assignment(button, action)){
                    Ok(())=>Reply::Ok{message:format!("Pulsefire Haste Wireless button {button} assignment saved through ForgeHX Native")},Err(error)=>error_reply(mouse_error(error)),
                }
            }
            _ => error_reply(ForgeHxError::Unsupported(Capability::Bindings)),
        }
    }
    fn set_lift_off_distance(&self, id: &DeviceId, mm: u8) -> Reply {
        let owner = match self.owner(id, Capability::Dpi) {
            Ok(o) => o,
            Err(e) => return error_reply(e),
        };
        if owner.backend != BackendKind::ForgeHxNative {
            return Reply::error(
                "unsupported",
                "lift-off distance requires the ForgeHX Native Haste driver",
            );
        }
        let device = match self.device(id) {
            Ok(device) => device,
            Err(error) => return error_reply(error),
        };
        if device.protocol.as_deref() != Some(PULSEFIRE_HASTE_DRIVER_ID) {
            return Reply::error(
                "protocol_unimplemented",
                "native lift-off distance writer is not implemented for this driver",
            );
        }
        match PulsefireHasteWireless::from_device(device)
            .and_then(|mouse| mouse.set_lift_off_distance(mm))
        {
            Ok(()) => Reply::Ok {
                message: format!("Pulsefire Haste Wireless lift-off distance set to {mm} mm"),
            },
            Err(error) => error_reply(mouse_error(error)),
        }
    }

    fn apply_profile(&mut self, name: &str, device_id: &DeviceId) -> Reply {
        let profile = match self.profiles.load(name) {
            Ok(p) => p,
            Err(e) => return error_reply(e),
        };
        let device = match self.device(device_id) {
            Ok(d) => d.clone(),
            Err(e) => return error_reply(e),
        };
        let mut skipped = Vec::new();
        if let Some(lighting) = &profile.lighting {
            if matches!(self.set_lighting(device_id, lighting), Reply::Error { .. }) {
                skipped.push("lighting".into());
            }
        }
        if let Some(mouse) = &profile.mouse {
            if !mouse.dpi.stages.is_empty()
                && matches!(self.set_dpi(device_id, &mouse.dpi), Reply::Error { .. })
            {
                skipped.push("dpi".into());
            }
            if let Some(rate) = mouse.report_rate_hz {
                if matches!(self.set_polling_rate(device_id, rate), Reply::Error { .. }) {
                    skipped.push("polling_rate".into());
                }
            }
            if let Some(p) = mouse.active_profile {
                if matches!(self.set_mouse_profile(device_id, p), Reply::Error { .. }) {
                    skipped.push("mouse_profile".into());
                }
            }
            for binding in &mouse.button_assignments {
                if let Ok(button) = binding.key.parse::<u32>() {
                    if matches!(
                        self.set_button_assignment(device_id, button, &binding.action),
                        Reply::Error { .. }
                    ) {
                        skipped.push(format!("button_{button}"));
                    }
                }
            }
            if let Some(mm) = mouse.lift_off_distance_mm {
                if matches!(
                    self.set_lift_off_distance(device_id, mm),
                    Reply::Error { .. }
                ) {
                    skipped.push("lift_off_distance".into());
                }
            }
        }
        let live_audio = self.audio.discover().unwrap_or_default();
        let device_audio_ids = device
            .interfaces
            .iter()
            .filter_map(|i| i.audio_node_id)
            .collect::<Vec<_>>();
        if let Some(audio) = &profile.audio {
            if let Some(node) = live_audio
                .iter()
                .find(|n| n.kind == "sink" && device_audio_ids.contains(&n.id))
                .map(|n| n.id)
            {
                if self.audio.set_volume(node, audio.volume).is_err() {
                    skipped.push("audio_volume".into());
                }
                if self.audio.set_mute(node, audio.muted).is_err() {
                    skipped.push("audio_mute".into());
                }
            } else {
                skipped.push("audio".into());
            }
        }
        if let Some(mic) = &profile.microphone {
            if let Some(node) = live_audio
                .iter()
                .find(|n| n.kind == "source" && device_audio_ids.contains(&n.id))
                .map(|n| n.id)
            {
                if self.audio.set_volume(node, mic.volume).is_err() {
                    skipped.push("microphone_volume".into());
                }
                if self.audio.set_mute(node, mic.muted).is_err() {
                    skipped.push("microphone_mute".into());
                }
            } else {
                skipped.push("microphone".into());
            }
        }
        if let Some(eq) = &profile.eq {
            if let Some(node) = live_audio
                .iter()
                .find(|n| n.kind == "sink" && device_audio_ids.contains(&n.id))
            {
                let mut eq = eq.clone();
                eq.target_node_id = Some(node.id);
                if self
                    .eq
                    .save(&eq)
                    .and_then(|_| self.eq.apply(&eq.name))
                    .is_err()
                {
                    skipped.push("eq".into());
                }
            } else {
                skipped.push("eq".into());
            }
        }
        skipped.sort();
        skipped.dedup();
        Reply::Applied { skipped }
    }

    #[cfg(test)]
    pub fn empty_for_test() -> Self {
        Self::new(
            ProfileStore::new(test_root()),
            Box::new(EmptyDiscovery),
            Box::new(EmptyAudio),
        )
    }
}

fn assign_capability_owners(
    info: &mut DeviceInfo,
    source_node_ids: &[u32],
    linux_health: &BackendHealth,
    openrgb_health: &BackendHealth,
    openrgb: &[ExternalIdentity],
    ratbag_health: &BackendHealth,
    ratbag: &[ExternalIdentity],
    ratbag_client: &RatbagClient,
) {
    let mut candidates: BTreeMap<Capability, Vec<BackendCandidate>> = BTreeMap::new();
    add_candidate(
        &mut candidates,
        CapabilityOwner {
            capability: Capability::Diagnostics,
            backend: BackendKind::Diagnostic,
            backend_device_id: None,
            verified: true,
            writable: false,
            detail: Some("read-only".into()),
        },
        BackendHealth::Ready,
    );
    if info.protocol.is_some() {
        for cap in info
            .capabilities
            .iter()
            .copied()
            .filter(|c| *c != Capability::Diagnostics)
        {
            let writable = cap != Capability::BatteryStatus;
            add_candidate(
                &mut candidates,
                CapabilityOwner {
                    capability: cap,
                    backend: BackendKind::ForgeHxNative,
                    backend_device_id: info.protocol.clone(),
                    verified: true,
                    writable,
                    detail: Some(if writable {
                        "verified native driver".into()
                    } else {
                        "verified native telemetry".into()
                    }),
                },
                BackendHealth::Ready,
            );
        }
    }
    for cap in info.generic_capabilities.iter().copied() {
        add_candidate(
            &mut candidates,
            CapabilityOwner {
                capability: cap,
                backend: BackendKind::LinuxStandard,
                backend_device_id: info
                    .interfaces
                    .iter()
                    .find_map(|i| i.audio_node_id)
                    .map(|v| v.to_string()),
                verified: true,
                writable: cap != Capability::BatteryStatus,
                detail: Some("Linux standard interface".into()),
            },
            linux_health.clone(),
        );
    }
    if info.vendor_family == VendorFamily::HyperX && info.device_class == DeviceClass::Microphone {
        if let Some(node_id) = info
            .interfaces
            .iter()
            .filter_map(|interface| interface.audio_node_id)
            .find(|node_id| source_node_ids.contains(node_id))
        {
            add_candidate(
                &mut candidates,
                CapabilityOwner {
                    capability: Capability::MicDsp,
                    backend: BackendKind::ForgeHxDsp,
                    backend_device_id: Some(node_id.to_string()),
                    verified: true,
                    writable: true,
                    detail: Some("input-only PipeWire capture graph".into()),
                },
                linux_health.clone(),
            );
        }
    }
    if let Some(adapter) = FirmwareRegistry::for_device(info) {
        add_candidate(
            &mut candidates,
            CapabilityOwner {
                capability: Capability::MicFirmwareInventory,
                backend: BackendKind::ForgeHxNative,
                backend_device_id: Some(adapter.id.into()),
                verified: adapter.exact_hardware_match,
                writable: false,
                detail: Some(if adapter.exact_hardware_match {
                    "exact HyperX microphone firmware inventory".into()
                } else {
                    "HyperX microphone model inventory only".into()
                }),
            },
            BackendHealth::Ready,
        );
        if adapter.support_level.can_update() {
            add_candidate(
                &mut candidates,
                CapabilityOwner {
                    capability: Capability::MicFirmwareUpdate,
                    backend: BackendKind::ForgeHxNative,
                    backend_device_id: Some(adapter.id.into()),
                    verified: true,
                    writable: true,
                    detail: Some("verified model-specific firmware updater".into()),
                },
                BackendHealth::Ready,
            );
        }
        if adapter.support_level.can_recover() {
            add_candidate(
                &mut candidates,
                CapabilityOwner {
                    capability: Capability::MicFirmwareRecovery,
                    backend: BackendKind::ForgeHxNative,
                    backend_device_id: Some(adapter.id.into()),
                    verified: true,
                    writable: true,
                    detail: Some("verified model-specific firmware recovery".into()),
                },
                BackendHealth::Ready,
            );
        }
    }
    match match_external_device(info, openrgb) {
        Ok(external) => add_candidate(
            &mut candidates,
            CapabilityOwner {
                capability: Capability::Lighting,
                backend: BackendKind::OpenRgb,
                backend_device_id: Some(external.id.clone()),
                verified: true,
                writable: true,
                detail: Some("deterministic OpenRGB match".into()),
            },
            openrgb_health.clone(),
        ),
        Err(MatchError::Ambiguous) => add_candidate(
            &mut candidates,
            CapabilityOwner {
                capability: Capability::Lighting,
                backend: BackendKind::OpenRgb,
                backend_device_id: None,
                verified: false,
                writable: false,
                detail: Some("ambiguous OpenRGB match".into()),
            },
            BackendHealth::AmbiguousDeviceMatch,
        ),
        Err(MatchError::NoMatch) => {}
    }
    if let Ok(external) = match_external_device(info, ratbag) {
        if let Ok(state) = ratbag_client.state(&external.id) {
            if !state.dpi_stages.is_empty() {
                add_candidate(
                    &mut candidates,
                    CapabilityOwner {
                        capability: Capability::Dpi,
                        backend: BackendKind::Ratbag,
                        backend_device_id: Some(external.id.clone()),
                        verified: true,
                        writable: true,
                        detail: Some("ratbagd resolution".into()),
                    },
                    ratbag_health.clone(),
                );
            }
            if state.report_rate_hz.is_some() {
                add_candidate(
                    &mut candidates,
                    CapabilityOwner {
                        capability: Capability::PollingRate,
                        backend: BackendKind::Ratbag,
                        backend_device_id: Some(external.id.clone()),
                        verified: true,
                        writable: true,
                        detail: Some("ratbagd report rate".into()),
                    },
                    ratbag_health.clone(),
                );
            }
            if state.active_profile.is_some() {
                add_candidate(
                    &mut candidates,
                    CapabilityOwner {
                        capability: Capability::Profiles,
                        backend: BackendKind::Ratbag,
                        backend_device_id: Some(external.id.clone()),
                        verified: true,
                        writable: true,
                        detail: Some("ratbagd profiles".into()),
                    },
                    ratbag_health.clone(),
                );
            }
            if state.button_count.unwrap_or(0) > 0 {
                for cap in [Capability::Bindings, Capability::MacroAssignments] {
                    add_candidate(
                        &mut candidates,
                        CapabilityOwner {
                            capability: cap,
                            backend: BackendKind::Ratbag,
                            backend_device_id: Some(external.id.clone()),
                            verified: true,
                            writable: true,
                            detail: Some("ratbagd button actions".into()),
                        },
                        ratbag_health.clone(),
                    );
                }
            }
        }
    }
    info.capability_owners = candidates
        .into_iter()
        .filter_map(|(cap, values)| select_owner_with_health(cap, &values))
        .collect();
    info.capability_owners.sort_by_key(|o| o.capability);
}
fn add_candidate(
    map: &mut BTreeMap<Capability, Vec<BackendCandidate>>,
    owner: CapabilityOwner,
    health: BackendHealth,
) {
    map.entry(owner.capability)
        .or_default()
        .push(BackendCandidate { owner, health });
}

fn refresh_native_mouse_status(info: &mut DeviceInfo) {
    match info.protocol.as_deref() {
        Some(PULSEFIRE_HASTE_DRIVER_ID) => {
            let Ok(mouse) = PulsefireHasteWireless::from_device(info) else {
                return;
            };
            let Ok(status) = mouse.battery_status() else {
                return;
            };
            info.battery_percent = status.percent;
            info.battery_state = match (status.full, status.charging, status.wired) {
                (Some(true), _, _) => Some("full".into()),
                (_, Some(true), Some(true)) => Some("charging (wired)".into()),
                (_, Some(false), Some(false)) => Some("wireless".into()),
                (_, Some(true), _) => Some("charging".into()),
                (_, Some(false), _) => Some("battery".into()),
                _ => None,
            };
        }
        Some(SAGA_PRO_DRIVER_ID) => {
            let Ok(mouse) = SagaPro::from_device(info) else {
                return;
            };
            let Ok(status) = mouse.battery_status() else {
                return;
            };
            info.battery_percent = status.percent;
            info.battery_state = match (status.full, status.charging) {
                (Some(true), _) => Some("full".into()),
                (_, Some(true)) => Some("charging".into()),
                (_, Some(false)) => Some("battery".into()),
                _ => None,
            };
        }
        _ => {}
    }
}

fn project_devices_for_protocol(devices: &[DeviceInfo], protocol_version: u32) -> Vec<DeviceInfo> {
    devices
        .iter()
        .cloned()
        .map(|device| project_device_for_protocol(device, protocol_version))
        .collect()
}

fn project_device_for_protocol(mut device: DeviceInfo, protocol_version: u32) -> DeviceInfo {
    if protocol_version < 4 {
        device
            .capabilities
            .retain(|capability| *capability != Capability::MicDsp);
        device
            .generic_capabilities
            .retain(|capability| *capability != Capability::MicDsp);
        device.capability_owners.retain(|owner| {
            owner.capability != Capability::MicDsp && owner.backend != BackendKind::ForgeHxDsp
        });
        normalize_device_support(&mut device);
    }
    if protocol_version < 6 {
        let firmware_capability = |capability: Capability| {
            matches!(
                capability,
                Capability::MicFirmwareInventory
                    | Capability::MicFirmwareUpdate
                    | Capability::MicFirmwareRecovery
            )
        };
        device
            .capabilities
            .retain(|capability| !firmware_capability(*capability));
        device
            .generic_capabilities
            .retain(|capability| !firmware_capability(*capability));
        device
            .capability_owners
            .retain(|owner| !firmware_capability(owner.capability));
        normalize_device_support(&mut device);
    }
    device
}

fn project_backends_for_protocol(
    backends: &[BackendStatus],
    protocol_version: u32,
) -> Vec<BackendStatus> {
    backends
        .iter()
        .filter(|status| protocol_version >= 4 || status.backend != BackendKind::ForgeHxDsp)
        .cloned()
        .collect()
}

fn complete_native_support(device: &DeviceInfo, registered_native_level: SupportLevel) -> bool {
    registered_native_level == SupportLevel::FullySupported
        && device.protocol.is_some()
        && device
            .capabilities
            .iter()
            .filter(|capability| **capability != Capability::Diagnostics)
            .all(|capability| {
                device.selected_owner(*capability).is_some_and(|owner| {
                    owner.backend == BackendKind::ForgeHxNative && owner.verified
                })
            })
}

fn normalize_device_support(device: &mut DeviceInfo) {
    let registered_native_level = device.support_level;
    device.generic_capabilities.sort();
    device.generic_capabilities.dedup();
    device.capabilities.sort();
    device.capabilities.dedup();
    let complete_native = complete_native_support(device, registered_native_level);
    let complete_microphone = complete_microphone_support(device);
    let owners = &device.capability_owners;
    let useful = owners
        .iter()
        .filter(|o| o.capability != Capability::Diagnostics)
        .collect::<Vec<_>>();
    if complete_native || complete_microphone {
        device.support_level = SupportLevel::FullySupported;
    } else if useful.is_empty() {
        device.support_level = if device.interfaces.is_empty() {
            SupportLevel::Unavailable
        } else {
            SupportLevel::DiagnosticOnly
        };
    } else if useful
        .iter()
        .all(|o| o.backend == BackendKind::LinuxStandard)
    {
        device.support_level = SupportLevel::GenericControls;
    } else if device.protocol.is_some()
        && useful
            .iter()
            .all(|o| o.backend == BackendKind::ForgeHxNative)
    {
        device.support_level = SupportLevel::PartiallySupported;
    } else {
        device.support_level = SupportLevel::PartiallySupported;
    }
}

fn should_keep_forgehx_mic_source(device: &DeviceInfo) -> bool {
    device.vendor_family == VendorFamily::HyperX && device.device_class == DeviceClass::Microphone
}

fn attach_audio_nodes(discovered: &mut Vec<DiscoveredDevice>, nodes: Vec<forgehx_core::AudioNode>) {
    for node in nodes {
        let normalized = normalize_name(&node.name);
        let match_index = discovered.iter().position(|device| {
            let name = normalize_name(&device.info.name);
            !name.is_empty()
                && !normalized.is_empty()
                && (normalized.contains(&name) || name.contains(&normalized))
        });
        if let Some(index) = match_index {
            let device = &mut discovered[index];
            let is_source = node.kind == "source";
            push_generic_audio_capabilities(&mut device.info, is_source);
            device.info.interfaces.push(DeviceInterface {
                source: InterfaceSource::Audio,
                path: format!("pipewire:{}", node.id),
                vendor_id: (device.info.vendor_id != 0).then_some(device.info.vendor_id),
                product_id: (device.info.product_id != 0).then_some(device.info.product_id),
                interface_number: None,
                usage_page: None,
                usage: None,
                audio_node_id: Some(node.id),
                usb_parent: None,
            });
            device.info.connection_kind = ConnectionKind::Composite;
            device.raw_interfaces.push(RawInterface {
                source: InterfaceSource::Audio,
                path: format!("pipewire:{}", node.id),
                vendor_id: device.info.vendor_id,
                product_id: device.info.product_id,
                manufacturer: device.info.manufacturer.clone(),
                product: Some(node.name.clone()),
                serial: device.info.serial.clone(),
                interface_number: None,
                usage_page: None,
                usage: None,
                audio_node_id: Some(node.id),
                usb_parent: None,
                usb_class_codes: vec![0x01],
            });
            continue;
        }
        let raw = RawInterface {
            source: InterfaceSource::Audio,
            path: format!("pipewire:{}", node.id),
            vendor_id: 0,
            product_id: 0,
            manufacturer: vendor_name_from_audio(&node.name),
            product: Some(node.name.clone()),
            serial: None,
            interface_number: None,
            usage_page: None,
            usage: None,
            audio_node_id: Some(node.id),
            usb_parent: None,
            usb_class_codes: vec![0x01],
        };
        let vendor_family = classify::vendor_family(&raw);
        let mut info = DeviceInfo {
            id: stable_audio_device_id(&node.name),
            name: node.name.clone(),
            manufacturer: raw.manufacturer.clone(),
            vendor_id: 0,
            product_id: 0,
            serial: None,
            vendor_family,
            device_class: if node.kind == "source" {
                DeviceClass::Microphone
            } else {
                DeviceClass::Headset
            },
            support_level: SupportLevel::GenericControls,
            connection_kind: ConnectionKind::Audio,
            interfaces: vec![DeviceInterface::from(&raw)],
            capabilities: vec![Capability::Diagnostics],
            generic_capabilities: vec![],
            capability_owners: vec![],
            battery_percent: None,
            battery_state: None,
            protocol: None,
        };
        push_generic_audio_capabilities(&mut info, node.kind == "source");
        discovered.push(DiscoveredDevice {
            info,
            raw_interfaces: vec![raw],
        });
    }
}
fn push_generic_audio_capabilities(device: &mut DeviceInfo, is_source: bool) {
    for cap in [Capability::Audio, Capability::Volume, Capability::Mute] {
        if !device.generic_capabilities.contains(&cap) {
            device.generic_capabilities.push(cap);
        }
    }
    if is_source {
        if !device
            .generic_capabilities
            .contains(&Capability::Microphone)
        {
            device.generic_capabilities.push(Capability::Microphone);
        }
    } else if !device.generic_capabilities.contains(&Capability::Eq) {
        device.generic_capabilities.push(Capability::Eq);
    }
}
fn select_wired_monitor_sink(nodes: &[AudioNode]) -> Option<AudioNode> {
    nodes
        .iter()
        .filter(|node| node.kind == "sink")
        .filter_map(|node| {
            let lower = node.name.to_ascii_lowercase();
            if lower.contains("bluez") || lower.contains("bluetooth") {
                return None;
            }
            let priority = if lower.contains("headphone") || lower.contains("headphones") {
                0u8
            } else if lower.contains("analog") && lower.contains("alsa") {
                1u8
            } else {
                return None;
            };
            Some((priority, node.id, node))
        })
        .min_by_key(|(priority, id, _)| (*priority, *id))
        .map(|(_, _, node)| node.clone())
}

fn stable_audio_device_id(node_name: &str) -> DeviceId {
    let normalized = normalize_name(node_name);
    let seed = if normalized.is_empty() {
        node_name
    } else {
        normalized.as_str()
    };
    let mut hash = 0xcbf29ce484222325u64;
    for byte in seed.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    DeviceId(format!("audio-name:{hash:016x}"))
}
fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}
fn vendor_name_from_audio(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    for vendor in [
        "HyperX",
        "Logitech",
        "Corsair",
        "Razer",
        "SteelSeries",
        "Kingston",
        "HP",
    ] {
        if lower.contains(&vendor.to_ascii_lowercase()) {
            return Some(vendor.into());
        }
    }
    None
}
fn parse_ratbag_usb_model(model: &str) -> Option<(u16, u16)> {
    let text = model.strip_prefix("usb:")?;
    let mut parts = text.split(':');
    let vid = u16::from_str_radix(parts.next()?, 16).ok()?;
    let pid = u16::from_str_radix(parts.next()?, 16).ok()?;
    Some((vid, pid))
}
fn health_from_backend_error(error: &BackendError) -> BackendHealth {
    match error {
        BackendError::Unavailable(_) => BackendHealth::Unavailable,
        BackendError::Protocol(_) => BackendHealth::ProtocolMismatch,
        BackendError::Ambiguous(_) => BackendHealth::AmbiguousDeviceMatch,
        BackendError::Invalid(msg) | BackendError::Io(msg) => {
            BackendHealth::BackendError(msg.clone())
        }
    }
}
fn mouse_error(error: MouseError) -> ForgeHxError {
    let message = error.to_string();
    if message.to_ascii_lowercase().contains("permission denied") {
        ForgeHxError::Permission(message)
    } else if matches!(error, MouseError::Unsupported(_) | MouseError::Invalid(_)) {
        ForgeHxError::BackendProtocol(message)
    } else {
        ForgeHxError::Io(message)
    }
}
fn backend_error(error: BackendError) -> ForgeHxError {
    match error {
        BackendError::Unavailable(v) => ForgeHxError::BackendUnavailable(v),
        BackendError::Protocol(v) => ForgeHxError::BackendProtocol(v),
        BackendError::Ambiguous(v) => ForgeHxError::AmbiguousDevice(v),
        BackendError::Invalid(v) => ForgeHxError::BackendProtocol(v),
        BackendError::Io(v) => ForgeHxError::Io(v),
    }
}
fn firmware_error_reply(error: forgehx_firmware::FirmwareError) -> Reply {
    let code = match error {
        forgehx_firmware::FirmwareError::NotRegularFile(_) => "firmware_source_invalid",
        forgehx_firmware::FirmwareError::AdapterNotFound => "firmware_unsupported",
        forgehx_firmware::FirmwareError::UpdateNotEnabled => "firmware_update_unavailable",
        forgehx_firmware::FirmwareError::InvalidTransition(_) => "firmware_state",
        forgehx_firmware::FirmwareError::Journal(_) => "firmware_journal",
        forgehx_firmware::FirmwareError::Io(_) => "firmware_io",
    };
    Reply::error(code, error.to_string())
}
fn error_reply(error: ForgeHxError) -> Reply {
    let code = match &error {
        ForgeHxError::Unsupported(_) => "unsupported",
        ForgeHxError::DeviceNotFound(_) => "device_not_found",
        ForgeHxError::BackendUnavailable(_) => "backend_unavailable",
        ForgeHxError::BackendProtocol(_) => "backend_protocol",
        ForgeHxError::AmbiguousDevice(_) => "ambiguous_device",
        ForgeHxError::Permission(_) => "permission",
        ForgeHxError::AudioUnavailable(_) => "audio_unavailable",
        ForgeHxError::ProtocolVersion { .. } => "protocol_version",
        ForgeHxError::InvalidProfile(_) | ForgeHxError::UnsupportedSchema(_) => "profile",
        ForgeHxError::Io(_) => "io",
    };
    Reply::error(code, error.to_string())
}

pub fn socket_path() -> PathBuf {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime).join("forgehx/forgehx.sock")
    } else {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
        PathBuf::from(format!("/tmp/forgehx-{user}/forgehx.sock"))
    }
}
pub async fn run_server(state: DaemonState) -> io::Result<()> {
    let socket = socket_path();
    if let Some(parent) = socket.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if socket.exists() {
        let _ = tokio::fs::remove_file(&socket).await;
    }
    let listener = UnixListener::bind(&socket)?;
    info!(path=%socket.display(),"ForgeHX daemon listening");
    let state = Arc::new(Mutex::new(state));
    {
        let mut initial = state.lock().await;
        if let Err(e) = initial.refresh_devices() {
            warn!(%e,"initial discovery failed");
        }
    }
    let refresh_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut state = refresh_state.lock().await;
            if let Err(error) = state.refresh_devices() {
                warn!(%error,"device refresh failed; retaining previous coherent inventory");
            }
        }
    });
    loop {
        let (stream, _) = listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(error) = handle_stream(stream, state).await {
                warn!(%error,"IPC client failed");
            }
        });
    }
}
async fn handle_stream(mut stream: UnixStream, state: Arc<Mutex<DaemonState>>) -> io::Result<()> {
    let command: Command = read_frame(&mut stream).await?;
    let reply = {
        let mut state = state.lock().await;
        state.handle(command)
    };
    write_frame(&mut stream, &reply).await
}
async fn read_frame<T: serde::de::DeserializeOwned>(stream: &mut UnixStream) -> io::Result<T> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length).await?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > 4 * 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid ForgeHX frame length",
        ));
    }
    let mut payload = vec![0u8; length];
    stream.read_exact(&mut payload).await?;
    serde_json::from_slice(&payload).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
async fn write_frame<T: serde::Serialize>(stream: &mut UnixStream, value: &T) -> io::Result<()> {
    let payload =
        serde_json::to_vec(value).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let length = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "ForgeHX frame too large"))?;
    stream.write_all(&length.to_be_bytes()).await?;
    stream.write_all(&payload).await?;
    stream.flush().await
}

#[cfg(test)]
fn test_root() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("forgehx-test-{}-{nanos}", std::process::id()))
}
#[cfg(test)]
struct EmptyDiscovery;
#[cfg(test)]
impl DiscoveryBackend for EmptyDiscovery {
    fn enumerate_hid(
        &self,
    ) -> Result<Vec<forgehx_device::HidDescriptor>, forgehx_device::DeviceError> {
        Ok(vec![])
    }
    fn enumerate_usb(
        &self,
    ) -> Result<Vec<forgehx_device::UsbDescriptor>, forgehx_device::DeviceError> {
        Ok(vec![])
    }
}
#[cfg(test)]
struct EmptyAudio;
#[cfg(test)]
impl AudioBackend for EmptyAudio {
    fn discover(&self) -> Result<Vec<forgehx_core::AudioNode>, ForgeHxError> {
        Ok(vec![])
    }
    fn set_volume(&self, _: u32, _: f32) -> Result<(), ForgeHxError> {
        Ok(())
    }
    fn set_mute(&self, _: u32, _: bool) -> Result<(), ForgeHxError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_device::{HidDescriptor, UsbDescriptor};
    #[derive(Clone)]
    struct FakeDiscovery {
        hid: Vec<HidDescriptor>,
    }
    impl DiscoveryBackend for FakeDiscovery {
        fn enumerate_hid(&self) -> Result<Vec<HidDescriptor>, forgehx_device::DeviceError> {
            Ok(self.hid.clone())
        }
        fn enumerate_usb(&self) -> Result<Vec<UsbDescriptor>, forgehx_device::DeviceError> {
            Ok(vec![])
        }
    }
    struct FakeAudio {
        nodes: Vec<forgehx_core::AudioNode>,
    }
    impl AudioBackend for FakeAudio {
        fn discover(&self) -> Result<Vec<forgehx_core::AudioNode>, ForgeHxError> {
            Ok(self.nodes.clone())
        }
        fn set_volume(&self, _: u32, _: f32) -> Result<(), ForgeHxError> {
            Ok(())
        }
        fn set_mute(&self, _: u32, _: bool) -> Result<(), ForgeHxError> {
            Ok(())
        }
    }
    struct SequenceAudio {
        batches: std::sync::Mutex<std::collections::VecDeque<Vec<forgehx_core::AudioNode>>>,
    }
    impl SequenceAudio {
        fn new(batches: Vec<Vec<forgehx_core::AudioNode>>) -> Self {
            Self {
                batches: std::sync::Mutex::new(batches.into()),
            }
        }
    }
    impl AudioBackend for SequenceAudio {
        fn discover(&self) -> Result<Vec<forgehx_core::AudioNode>, ForgeHxError> {
            Ok(self.batches.lock().unwrap().pop_front().unwrap_or_default())
        }
        fn set_volume(&self, _: u32, _: f32) -> Result<(), ForgeHxError> {
            Ok(())
        }
        fn set_mute(&self, _: u32, _: bool) -> Result<(), ForgeHxError> {
            Ok(())
        }
    }
    fn hid(
        vendor: u16,
        product: u16,
        manufacturer: &str,
        name: &str,
        serial: &str,
        usage: u16,
    ) -> HidDescriptor {
        HidDescriptor {
            path: format!("/dev/hidraw-{serial}"),
            vendor_id: vendor,
            product_id: product,
            manufacturer: Some(manufacturer.into()),
            product: Some(name.into()),
            serial: Some(serial.into()),
            interface_number: 0,
            usage_page: 1,
            usage,
            usb_parent: None,
        }
    }
    fn sink(id: u32, name: &str) -> AudioNode {
        AudioNode {
            id,
            name: name.into(),
            kind: "sink".into(),
            volume: Some(1.0),
            muted: Some(false),
        }
    }
    #[test]
    fn forgehx_mic_identity_survives_missing_raw_source() {
        let device = DeviceInfo {
            id: DeviceId("03f0:0fbf:1H552803ZH".into()),
            name: "HyperX SoloCast 2".into(),
            manufacturer: Some("HP, Inc".into()),
            vendor_id: 0x03f0,
            product_id: 0x0fbf,
            serial: Some("1H552803ZH".into()),
            vendor_family: VendorFamily::HyperX,
            device_class: DeviceClass::Microphone,
            support_level: SupportLevel::PartiallySupported,
            connection_kind: ConnectionKind::Usb,
            interfaces: Vec::new(),
            capabilities: vec![Capability::Diagnostics],
            generic_capabilities: Vec::new(),
            capability_owners: Vec::new(),
            battery_percent: None,
            battery_state: None,
            protocol: None,
        };
        assert!(should_keep_forgehx_mic_source(&device));
        assert!(device.selected_owner(Capability::MicDsp).is_none());
    }

    #[test]
    fn stable_audio_device_id_is_independent_of_pipewire_node_number() {
        let first = AudioNode {
            id: 81,
            name: "alsa_input.usb-HP__Inc_HyperX_SoloCast_2_SERIAL-00.analog-stereo".into(),
            kind: "source".into(),
            volume: None,
            muted: None,
        };
        let second = AudioNode {
            id: 84,
            name: first.name.clone(),
            kind: "source".into(),
            volume: None,
            muted: None,
        };
        assert_eq!(
            stable_audio_device_id(&first.name),
            stable_audio_device_id(&second.name)
        );
        assert_ne!(
            stable_audio_device_id(&first.name),
            stable_audio_device_id("alsa_input.usb-Logitech_Brio_100_OTHER-02.mono-fallback")
        );
    }
    #[test]
    fn wired_monitor_never_selects_bluetooth() {
        let nodes = vec![
            sink(10, "bluez_output.11_22_33.a2dp-sink"),
            sink(11, "Bluetooth Headphones"),
            sink(12, "alsa_output.pci-0000_00_1f.3.analog-stereo"),
        ];
        let selected = select_wired_monitor_sink(&nodes).unwrap();
        assert_eq!(selected.id, 12);
    }
    #[test]
    fn wired_monitor_prefers_explicit_headphone_sink_over_generic_analog() {
        let nodes = vec![
            sink(20, "alsa_output.pci-0000_00_1f.3.analog-stereo"),
            sink(21, "alsa_output.usb-DAC.Headphones"),
        ];
        let selected = select_wired_monitor_sink(&nodes).unwrap();
        assert_eq!(selected.id, 21);
    }
    #[test]
    fn wired_monitor_rejects_hdmi_only_configuration() {
        let nodes = vec![
            sink(30, "alsa_output.pci-0000_03_00.1.hdmi-stereo"),
            sink(31, "bluez_output.headset"),
        ];
        assert!(select_wired_monitor_sink(&nodes).is_none());
    }
    #[test]
    fn hyperx_and_all_device_views_are_separate() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(FakeDiscovery {
                hid: vec![
                    hid(0x0951, 0x1111, "HyperX", "HyperX Alloy", "HX", 6),
                    hid(0x046d, 0x2222, "Logitech", "Gaming Mouse", "LG", 2),
                ],
            }),
            Box::new(EmptyAudio),
        );
        state.refresh_devices().unwrap();
        assert_eq!(state.hyperx_devices.len(), 1);
        assert_eq!(state.all_devices.len(), 2);
    }
    #[test]
    fn generic_audio_gets_linux_standard_owner() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(EmptyDiscovery),
            Box::new(FakeAudio {
                nodes: vec![forgehx_core::AudioNode {
                    id: 77,
                    name: "Logitech USB Headset".into(),
                    kind: "sink".into(),
                    volume: Some(0.8),
                    muted: Some(false),
                }],
            }),
        );
        state.refresh_devices().unwrap();
        let device = &state.all_devices[0];
        assert_eq!(
            device.selected_owner(Capability::Volume).unwrap().backend,
            BackendKind::LinuxStandard
        );
        assert!(device.supports(Capability::Eq));
    }
    #[test]
    fn owner_selection_never_promotes_diagnostic_over_writable_linux() {
        let mut info = DeviceInfo {
            id: DeviceId("x".into()),
            name: "Headset".into(),
            manufacturer: None,
            vendor_id: 0,
            product_id: 0,
            serial: None,
            vendor_family: VendorFamily::Unknown,
            device_class: DeviceClass::Headset,
            support_level: SupportLevel::DiagnosticOnly,
            connection_kind: ConnectionKind::Audio,
            interfaces: vec![],
            capabilities: vec![Capability::Diagnostics],
            generic_capabilities: vec![Capability::Volume],
            capability_owners: vec![],
            battery_percent: None,
            battery_state: None,
            protocol: None,
        };
        assign_capability_owners(
            &mut info,
            &[],
            &BackendHealth::Ready,
            &BackendHealth::Unavailable,
            &[],
            &BackendHealth::Unavailable,
            &[],
            &RatbagClient,
        );
        assert_eq!(
            info.selected_owner(Capability::Volume).unwrap().backend,
            BackendKind::LinuxStandard
        );
    }
    #[test]
    fn native_lighting_owner_beats_openrgb_fallback() {
        let mut info = DeviceInfo {
            id: DeviceId("hx".into()),
            name: "HyperX Keyboard".into(),
            manufacturer: Some("HyperX".into()),
            vendor_id: 0x0951,
            product_id: 0x1234,
            serial: Some("ABC".into()),
            vendor_family: VendorFamily::HyperX,
            device_class: DeviceClass::Keyboard,
            support_level: SupportLevel::PartiallySupported,
            connection_kind: ConnectionKind::Hid,
            interfaces: vec![],
            capabilities: vec![Capability::Diagnostics, Capability::Lighting],
            generic_capabilities: vec![],
            capability_owners: vec![],
            battery_percent: None,
            battery_state: None,
            protocol: Some("verified-test".into()),
        };
        let rgb = vec![ExternalIdentity {
            id: "3".into(),
            name: "HyperX Keyboard".into(),
            vendor: Some("HyperX".into()),
            serial: Some("ABC".into()),
            location: None,
            vendor_id: Some(0x0951),
            product_id: Some(0x1234),
        }];
        assign_capability_owners(
            &mut info,
            &[],
            &BackendHealth::Ready,
            &BackendHealth::Ready,
            &rgb,
            &BackendHealth::Unavailable,
            &[],
            &RatbagClient,
        );
        assert_eq!(
            info.selected_owner(Capability::Lighting).unwrap().backend,
            BackendKind::ForgeHxNative
        );
    }
    #[test]
    fn unavailable_openrgb_never_owns_lighting() {
        let mut info = DeviceInfo {
            id: DeviceId("hx".into()),
            name: "HyperX Keyboard".into(),
            manufacturer: Some("HyperX".into()),
            vendor_id: 0x0951,
            product_id: 0x1234,
            serial: None,
            vendor_family: VendorFamily::HyperX,
            device_class: DeviceClass::Keyboard,
            support_level: SupportLevel::DiagnosticOnly,
            connection_kind: ConnectionKind::Hid,
            interfaces: vec![],
            capabilities: vec![Capability::Diagnostics],
            generic_capabilities: vec![],
            capability_owners: vec![],
            battery_percent: None,
            battery_state: None,
            protocol: None,
        };
        let rgb = vec![ExternalIdentity {
            id: "3".into(),
            name: "HyperX Keyboard".into(),
            vendor: Some("HyperX".into()),
            serial: None,
            location: None,
            vendor_id: Some(0x0951),
            product_id: Some(0x1234),
        }];
        assign_capability_owners(
            &mut info,
            &[],
            &BackendHealth::Ready,
            &BackendHealth::Unavailable,
            &rgb,
            &BackendHealth::Unavailable,
            &[],
            &RatbagClient,
        );
        assert!(info.selected_owner(Capability::Lighting).is_none());
    }

    #[test]
    fn exact_legacy_keyboard_without_native_driver_does_not_gain_native_lighting() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(FakeDiscovery {
                hid: vec![hid(
                    0x03f0,
                    0x0591,
                    "HP, Inc",
                    "HyperX Alloy Origins",
                    "KBD1",
                    6,
                )],
            }),
            Box::new(EmptyAudio),
        );
        state.refresh_devices().unwrap();
        let device = &state.all_devices[0];
        assert_eq!(device.device_class, DeviceClass::Keyboard);
        assert!(device.protocol.is_none());
        assert!(device.selected_owner(Capability::Lighting).is_none());
    }

    #[test]
    fn deterministic_openrgb_keyboard_match_owns_lighting() {
        let mut info = DeviceInfo {
            id: DeviceId("kbd".into()),
            name: "HyperX Alloy Origins".into(),
            manufacturer: Some("HP, Inc".into()),
            vendor_id: 0x03f0,
            product_id: 0x0591,
            serial: Some("KBD2".into()),
            vendor_family: VendorFamily::HyperX,
            device_class: DeviceClass::Keyboard,
            support_level: SupportLevel::DiagnosticOnly,
            connection_kind: ConnectionKind::Hid,
            interfaces: vec![],
            capabilities: vec![Capability::Diagnostics],
            generic_capabilities: vec![],
            capability_owners: vec![],
            battery_percent: None,
            battery_state: None,
            protocol: None,
        };
        let rgb = vec![ExternalIdentity {
            id: "openrgb:alloy-origins".into(),
            name: "HyperX Alloy Origins".into(),
            vendor: Some("HyperX".into()),
            serial: Some("KBD2".into()),
            location: None,
            vendor_id: Some(0x03f0),
            product_id: Some(0x0591),
        }];
        assign_capability_owners(
            &mut info,
            &[],
            &BackendHealth::Ready,
            &BackendHealth::Ready,
            &rgb,
            &BackendHealth::Unavailable,
            &[],
            &RatbagClient,
        );
        let owner = info
            .selected_owner(Capability::Lighting)
            .expect("OpenRGB lighting owner");
        assert_eq!(owner.backend, BackendKind::OpenRgb);
        assert!(owner.writable);
        assert!(info.protocol.is_none());
    }

    #[test]
    fn pulsefire_haste_wireless_is_promoted_with_native_mouse_owners() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(FakeDiscovery {
                hid: vec![HidDescriptor {
                    path: "/dev/hidraw2".into(),
                    vendor_id: 0x03f0,
                    product_id: 0x028e,
                    manufacturer: Some("HP, Inc".into()),
                    product: Some("HyperX Pulsefire Haste Wireless".into()),
                    serial: None,
                    interface_number: 2,
                    usage_page: 0xff00,
                    usage: 1,
                    usb_parent: Some("1-2".into()),
                }],
            }),
            Box::new(EmptyAudio),
        );
        state.refresh_devices().unwrap();
        let device = &state.all_devices[0];
        assert_eq!(device.support_level, SupportLevel::FullySupported);
        assert_eq!(device.protocol.as_deref(), Some(PULSEFIRE_HASTE_DRIVER_ID));
        for cap in [
            Capability::Lighting,
            Capability::Dpi,
            Capability::PollingRate,
            Capability::Profiles,
            Capability::Bindings,
            Capability::MacroAssignments,
            Capability::BatteryStatus,
        ] {
            assert_eq!(
                device.selected_owner(cap).unwrap().backend,
                BackendKind::ForgeHxNative
            );
        }
        assert!(
            !device
                .selected_owner(Capability::BatteryStatus)
                .unwrap()
                .writable
        );
    }
    #[test]
    fn pulsefire_haste_wireless_wired_transport_uses_same_native_driver() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(FakeDiscovery {
                hid: vec![HidDescriptor {
                    path: "/dev/hidraw7".into(),
                    vendor_id: 0x03f0,
                    product_id: 0x048e,
                    manufacturer: Some("HP, Inc".into()),
                    product: Some("HyperX Pulsefire Haste Wireless".into()),
                    serial: None,
                    interface_number: 2,
                    usage_page: 0xff00,
                    usage: 1,
                    usb_parent: Some("7-4".into()),
                }],
            }),
            Box::new(EmptyAudio),
        );
        state.refresh_devices().unwrap();
        let device = &state.all_devices[0];
        assert_eq!(device.protocol.as_deref(), Some(PULSEFIRE_HASTE_DRIVER_ID));
        assert_eq!(device.support_level, SupportLevel::FullySupported);
    }
    #[test]
    fn pulsefire_haste_complete_driver_stays_full_with_compatibility_owners() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(FakeDiscovery {
                hid: vec![HidDescriptor {
                    path: "/dev/hidraw7".into(),
                    vendor_id: 0x03f0,
                    product_id: 0x048e,
                    manufacturer: Some("HP, Inc".into()),
                    product: Some("HyperX Pulsefire Haste Wireless".into()),
                    serial: None,
                    interface_number: 2,
                    usage_page: 0xff00,
                    usage: 1,
                    usb_parent: Some("7-4".into()),
                }],
            }),
            Box::new(EmptyAudio),
        );
        state.refresh_devices().unwrap();
        let device = &mut state.all_devices[0];
        device.generic_capabilities.push(Capability::Audio);
        device.capability_owners.push(CapabilityOwner {
            capability: Capability::Audio,
            backend: BackendKind::LinuxStandard,
            backend_device_id: Some("compat".into()),
            verified: true,
            writable: true,
            detail: Some("compatibility owner".into()),
        });
        normalize_device_support(device);
        assert_eq!(device.support_level, SupportLevel::FullySupported);
    }

    #[test]
    fn solocast_2_exact_composite_stack_is_fully_supported() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(FakeDiscovery {
                hid: vec![HidDescriptor {
                    path: "/dev/hidraw1".into(),
                    vendor_id: 0x03f0,
                    product_id: 0x0fbf,
                    manufacturer: Some("HP, Inc".into()),
                    product: Some("HyperX SoloCast 2".into()),
                    serial: Some("1H552803ZH".into()),
                    interface_number: 2,
                    usage_page: 0xff90,
                    usage: 1,
                    usb_parent: Some("3-1".into()),
                }],
            }),
            Box::new(FakeAudio {
                nodes: vec![forgehx_core::AudioNode {
                    id: 68,
                    name: "HyperX SoloCast 2 Analog Stereo".into(),
                    kind: "source".into(),
                    volume: Some(1.0),
                    muted: Some(false),
                }],
            }),
        );
        state.refresh_devices().unwrap();
        let device = state
            .all_devices
            .iter()
            .find(|device| device.vendor_id == 0x03f0 && device.product_id == 0x0fbf)
            .unwrap();
        assert_eq!(device.support_level, SupportLevel::FullySupported);
        assert_eq!(
            device.selected_owner(Capability::MicDsp).unwrap().backend,
            BackendKind::ForgeHxDsp
        );
        assert_eq!(
            device
                .selected_owner(Capability::Microphone)
                .unwrap()
                .backend,
            BackendKind::LinuxStandard
        );
        assert_eq!(
            device
                .selected_owner(Capability::MicFirmwareInventory)
                .unwrap()
                .backend,
            BackendKind::ForgeHxNative
        );
        assert!(
            device.protocol.is_none(),
            "full ForgeHX support must not imply undocumented raw-HID authority"
        );
    }
    #[test]
    fn saga_pro_exact_match_owns_only_verified_native_capabilities() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(FakeDiscovery {
                hid: vec![HidDescriptor {
                    path: "/dev/hidraw-saga".into(),
                    vendor_id: 0x03f0,
                    product_id: 0x04bf,
                    manufacturer: Some("HP, Inc".into()),
                    product: Some("HyperX Pulsefire Saga Pro".into()),
                    serial: None,
                    interface_number: 2,
                    usage_page: 0xff00,
                    usage: 1,
                    usb_parent: Some("1-3".into()),
                }],
            }),
            Box::new(EmptyAudio),
        );
        state.refresh_devices().unwrap();
        let device = &state.all_devices[0];
        assert_eq!(device.protocol.as_deref(), Some(SAGA_PRO_DRIVER_ID));
        assert_eq!(
            device.selected_owner(Capability::Lighting).unwrap().backend,
            BackendKind::ForgeHxNative
        );
        assert_eq!(
            device
                .selected_owner(Capability::BatteryStatus)
                .unwrap()
                .backend,
            BackendKind::ForgeHxNative
        );
        assert!(device.selected_owner(Capability::Dpi).is_none());
        assert!(device.selected_owner(Capability::PollingRate).is_none());
    }

    #[test]
    fn mouse_capabilities_reports_registry_limits_without_promoting_name_only_native_writes() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(FakeDiscovery {
                hid: vec![HidDescriptor {
                    path: "/dev/hidraw-future".into(),
                    vendor_id: 0x03f0,
                    product_id: 0xffff,
                    manufacturer: Some("HP, Inc".into()),
                    product: Some("HyperX Pulsefire Haste 2 S Wireless".into()),
                    serial: None,
                    interface_number: 0,
                    usage_page: 1,
                    usage: 2,
                    usb_parent: Some("1-4".into()),
                }],
            }),
            Box::new(EmptyAudio),
        );
        state.refresh_devices().unwrap();
        let id = state.all_devices[0].id.clone();
        let reply = state.handle(Command::MouseCapabilities {
            protocol_version: 5,
            device_id: id,
        });
        let Reply::MouseCapabilities { model: Some(model) } = reply else {
            panic!("expected mouse model")
        };
        assert_eq!(model.id, "pulsefire-haste-2-s-wireless");
        assert!(!model.exact_hardware_match);
        assert!(state.all_devices[0].protocol.is_none());
    }
    #[test]
    fn keyboard_capabilities_reports_specific_name_without_promoting_native_writes() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(FakeDiscovery {
                hid: vec![HidDescriptor {
                    path: "/dev/hidraw-keyboard-future".into(),
                    vendor_id: 0x03f0,
                    product_id: 0xffff,
                    manufacturer: Some("HP, Inc".into()),
                    product: Some("HyperX Alloy Rise 75 Wireless".into()),
                    serial: None,
                    interface_number: 0,
                    usage_page: 1,
                    usage: 6,
                    usb_parent: Some("1-5".into()),
                }],
            }),
            Box::new(EmptyAudio),
        );
        state.refresh_devices().unwrap();
        let id = state.all_devices[0].id.clone();
        let reply = state.handle(Command::KeyboardCapabilities {
            protocol_version: 7,
            device_id: id,
        });
        let Reply::KeyboardCapabilities { model: Some(model) } = reply else {
            panic!("expected keyboard model")
        };
        assert_eq!(model.id, "alloy-rise-75-wireless");
        assert!(!model.exact_hardware_match);
        assert!(model.native_driver.is_none());
        assert!(state.all_devices[0].protocol.is_none());
        assert!(state.all_devices[0]
            .selected_owner(Capability::Lighting)
            .is_none());
    }

    #[test]
    fn profile_store_migrates_v1_on_load() {
        let root = test_root();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Legacy.json"),r#"{"schema_version":1,"name":"Legacy","match_vendor_id":null,"match_product_id":null,"match_serial":null,"lighting":null,"dpi":{"stages":[800],"active_stage":0},"bindings":[],"audio":null,"microphone":null}"#).unwrap();
        let p = ProfileStore::new(root.clone()).load("Legacy").unwrap();
        assert_eq!(p.schema_version, 2);
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn ping_accepts_mismatched_version_for_negotiation() {
        let mut state = DaemonState::empty_for_test();
        let reply = state.handle(Command::Ping {
            protocol_version: 99,
        });
        assert!(matches!(
            reply,
            Reply::Pong {
                protocol_version: IPC_PROTOCOL_VERSION,
                min_protocol_version: IPC_MIN_PROTOCOL_VERSION
            }
        ));
    }
    #[test]
    fn v2_inventory_command_is_accepted() {
        let mut state = DaemonState::empty_for_test();
        let reply = state.handle(Command::ListAllDevices {
            protocol_version: 2,
        });
        assert!(matches!(reply, Reply::Devices { .. }));
    }
    #[test]
    fn v2_cannot_use_v3_backend_commands() {
        let mut state = DaemonState::empty_for_test();
        let reply = state.handle(Command::BackendStatus {
            protocol_version: 2,
        });
        assert!(matches!(reply,Reply::Error{code,..} if code=="protocol_version"));
    }

    #[test]
    fn hyperx_microphone_gets_dsp_owner_but_logitech_does_not() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(EmptyDiscovery),
            Box::new(FakeAudio {
                nodes: vec![
                    forgehx_core::AudioNode {
                        id: 81,
                        name: "HyperX QuadCast S Microphone".into(),
                        kind: "source".into(),
                        volume: Some(0.8),
                        muted: Some(false),
                    },
                    forgehx_core::AudioNode {
                        id: 82,
                        name: "Logitech Blue Yeti Microphone".into(),
                        kind: "source".into(),
                        volume: Some(0.8),
                        muted: Some(false),
                    },
                ],
            }),
        );
        state.refresh_devices().unwrap();
        let hyperx = state
            .all_devices
            .iter()
            .find(|device| device.name.contains("HyperX"))
            .unwrap();
        let logitech = state
            .all_devices
            .iter()
            .find(|device| device.name.contains("Logitech"))
            .unwrap();
        assert_eq!(
            hyperx.selected_owner(Capability::MicDsp).unwrap().backend,
            BackendKind::ForgeHxDsp
        );
        assert!(logitech.selected_owner(Capability::MicDsp).is_none());
    }

    #[test]
    fn v3_inventory_projection_hides_v4_mic_dsp_values() {
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(EmptyDiscovery),
            Box::new(FakeAudio {
                nodes: vec![forgehx_core::AudioNode {
                    id: 83,
                    name: "HyperX SoloCast Microphone".into(),
                    kind: "source".into(),
                    volume: Some(1.0),
                    muted: Some(false),
                }],
            }),
        );
        state.refresh_devices().unwrap();
        let Reply::Devices { devices } = state.handle(Command::ListAllDevices {
            protocol_version: 3,
        }) else {
            panic!("expected devices")
        };
        assert!(devices
            .iter()
            .all(|device| !device.capabilities.contains(&Capability::MicDsp)));
        assert!(devices
            .iter()
            .all(|device| !device.generic_capabilities.contains(&Capability::MicDsp)));
        assert!(devices
            .iter()
            .flat_map(|device| &device.capability_owners)
            .all(|owner| owner.capability != Capability::MicDsp
                && owner.backend != BackendKind::ForgeHxDsp));
    }

    #[test]
    fn v3_backend_status_projection_hides_v4_dsp_backend() {
        let mut state = DaemonState::empty_for_test();
        state.backend_statuses.push(BackendStatus {
            backend: BackendKind::ForgeHxDsp,
            health: BackendHealth::Ready,
            detail: None,
        });
        let Reply::BackendStatuses { backends } = state.handle(Command::BackendStatus {
            protocol_version: 3,
        }) else {
            panic!("expected backend statuses")
        };
        assert!(backends
            .iter()
            .all(|status| status.backend != BackendKind::ForgeHxDsp));
    }

    #[test]
    fn mic_dsp_apply_rejects_a_stale_pipewire_source_before_touching_the_graph() {
        let source = forgehx_core::AudioNode {
            id: 84,
            name: "HyperX QuadCast 2 S Microphone".into(),
            kind: "source".into(),
            volume: Some(1.0),
            muted: Some(false),
        };
        let mut state = DaemonState::new(
            ProfileStore::new(test_root()),
            Box::new(EmptyDiscovery),
            Box::new(SequenceAudio::new(vec![vec![source], vec![]])),
        );
        state.refresh_devices().unwrap();
        let device_id = state
            .all_devices
            .iter()
            .find(|device| device.supports(Capability::MicDsp))
            .unwrap()
            .id
            .clone();
        let reply = state.handle(Command::MicDspApply {
            protocol_version: 4,
            device_id,
            name: "Broadcast".into(),
        });
        assert!(matches!(reply, Reply::Error { code, .. } if code == "audio_unavailable"));
    }
}
