pub mod adaptive_noise;
pub mod aetherstream_bridge;
pub mod background_rejection;
pub mod direct_pipewire;
pub mod engine;
pub mod noise_monitor;
pub mod runtime;
pub mod speaker_lock;
pub mod transient;
pub mod voice_only;
pub mod voicepilot;
pub use adaptive_noise::{AdaptiveNoiseController, AdaptiveNoiseTargets};
pub use engine::{VoiceProcessingEngine, FRAME_SAMPLES, SAMPLE_RATE};
pub use noise_monitor::{ExtraneousNoiseMonitor, NoiseObservation};
pub use runtime::DspRuntimeRegistry;
pub use speaker_lock::{PlaybackLeakGuard, SpeakerDecision, SpeakerVerifier, VOICEPRINT_LEN};
pub use transient::TransientSuppressor;
pub use voice_only::{AlignedReference, VoiceOnlyProcessor, VoiceOnlySubtraction};
pub use voicepilot::{AdaptiveTargets, VoicePilotAnalyzer};

use forgehx_core::{DeviceId, ForgeHxError, MicrophoneDspConfig, MicrophoneDspState};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const AETHERSTREAM_SYSTEM_SOURCE_NAME: &str =
    aetherstream_bridge::AETHERSTREAM_SYSTEM_SOURCE_NAME;
pub const FORGEHX_PERMANENT_DSP: bool = true;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActiveMicState {
    profile_name: String,
    raw_source_node_name: String,
}

#[derive(Debug, Clone)]
pub struct MicDspManager {
    config_home: PathBuf,
    runtimes: DspRuntimeRegistry,
}

impl Default for MicDspManager {
    fn default() -> Self {
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(|| PathBuf::from(".config"));
        Self::new(root)
    }
}

impl MicDspManager {
    pub fn new(config_home: PathBuf) -> Self {
        Self {
            config_home,
            runtimes: DspRuntimeRegistry::default(),
        }
    }

    fn profile_dir(&self) -> PathBuf {
        self.config_home.join("forgehx/microphone/profiles")
    }
    fn active_dir(&self) -> PathBuf {
        self.config_home.join("forgehx/microphone/active")
    }
    fn pipewire_dir(&self) -> PathBuf {
        self.config_home.join("pipewire/pipewire.conf.d")
    }

    fn profile_path(&self, name: &str) -> Result<PathBuf, ForgeHxError> {
        validate_profile_name(name)?;
        Ok(self.profile_dir().join(format!("{name}.json")))
    }

    fn active_path(&self, device_id: &DeviceId) -> PathBuf {
        self.active_dir()
            .join(format!("{}.json", device_key(device_id)))
    }

    fn dropin_path(&self, device_id: &DeviceId) -> PathBuf {
        self.pipewire_dir()
            .join(format!("91-forgehx-mic-{}.conf", device_key(device_id)))
    }

    pub fn list(&self) -> Result<Vec<String>, ForgeHxError> {
        if !self.profile_dir().exists() {
            return Ok(Vec::new());
        }
        let mut names = fs::read_dir(self.profile_dir())
            .map_err(io_error)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                (path.extension().and_then(|v| v.to_str()) == Some("json"))
                    .then(|| path.file_stem()?.to_str().map(str::to_owned))
                    .flatten()
            })
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        Ok(names)
    }

    pub fn load(&self, name: &str) -> Result<MicrophoneDspConfig, ForgeHxError> {
        let path = self.profile_path(name)?;
        let bytes = fs::read(&path).map_err(io_error)?;
        let config: MicrophoneDspConfig = serde_json::from_slice(&bytes).map_err(|error| {
            ForgeHxError::InvalidProfile(format!("{}: {error}", path.display()))
        })?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, config: &MicrophoneDspConfig) -> Result<(), ForgeHxError> {
        config.validate()?;
        fs::create_dir_all(self.profile_dir()).map_err(io_error)?;
        let bytes = serde_json::to_vec_pretty(config)
            .map_err(|error| ForgeHxError::InvalidProfile(error.to_string()))?;
        atomic_write(&self.profile_path(&config.name)?, &bytes)
    }

    pub fn state(&self, device_id: &DeviceId) -> Result<MicrophoneDspState, ForgeHxError> {
        let active = self.read_active(device_id)?;
        let mut config = match &active {
            Some(active) => self.load(&active.profile_name).unwrap_or_default(),
            None => MicrophoneDspConfig::default(),
        };
        let key = device_key(device_id);
        if let Some(voiceprint) = self.runtimes.take_completed_voiceprint(&key) {
            config.speaker_lock.voiceprint = voiceprint;
            if active.is_some() {
                self.save(&config)?;
            }
        }
        Ok(MicrophoneDspState {
            device_id: device_id.clone(),
            config: config.clone(),
            applied: active.is_some(),
            raw_source_node_name: active
                .as_ref()
                .map(|value| value.raw_source_node_name.clone()),
            processed_source_node_name: AETHERSTREAM_SYSTEM_SOURCE_NAME.to_owned(),
            unavailable_processors: if active.is_some() && !self.runtimes.is_active(&key) {
                vec!["realtime_engine: inactive".into()]
            } else {
                Vec::new()
            },
            voicepilot_telemetry: self.runtimes.telemetry(&key).unwrap_or_default(),
            voice_isolation_telemetry: self
                .runtimes
                .voice_isolation_telemetry(&key)
                .unwrap_or_default(),
            noise_scene_telemetry: self
                .runtimes
                .noise_scene_telemetry(&key)
                .unwrap_or_default(),
            monitor: self.runtimes.monitor_state(&key).unwrap_or_default(),
        })
    }

    pub fn apply(
        &self,
        device_id: &DeviceId,
        profile_name: &str,
        raw_source_node_id: u32,
    ) -> Result<MicrophoneDspState, ForgeHxError> {
        let raw_source_node_name = resolve_node_target(raw_source_node_id)?;
        self.apply_with_source_name(device_id, profile_name, &raw_source_node_name)
    }

    pub fn apply_with_source_name(
        &self,
        device_id: &DeviceId,
        profile_name: &str,
        raw_source_node_name: &str,
    ) -> Result<MicrophoneDspState, ForgeHxError> {
        let mut config = self.load(profile_name)?;
        config.enabled = true;
        config.validate()?;
        self.save(&config)?;
        self.cleanup_legacy_bridge(device_id)?;
        fs::create_dir_all(self.active_dir()).map_err(io_error)?;
        let active = ActiveMicState {
            profile_name: config.name.clone(),
            raw_source_node_name: raw_source_node_name.to_owned(),
        };
        self.write_active(device_id, &active)?;
        let key = device_key(device_id);
        self.runtimes
            .start(
                &key,
                raw_source_node_name,
                &runtime_stream_identity(device_id),
                config.clone(),
            )
            .map_err(ForgeHxError::AudioUnavailable)?;
        Ok(MicrophoneDspState {
            device_id: device_id.clone(),
            config,
            applied: true,
            raw_source_node_name: Some(raw_source_node_name.to_owned()),
            processed_source_node_name: AETHERSTREAM_SYSTEM_SOURCE_NAME.to_owned(),
            unavailable_processors: Vec::new(),
            voicepilot_telemetry: self.runtimes.telemetry(&key).unwrap_or_default(),
            voice_isolation_telemetry: self
                .runtimes
                .voice_isolation_telemetry(&key)
                .unwrap_or_default(),
            noise_scene_telemetry: self
                .runtimes
                .noise_scene_telemetry(&key)
                .unwrap_or_default(),
            monitor: self.runtimes.monitor_state(&key).unwrap_or_default(),
        })
    }

    /// Ensure the ForgeHX-owned app-facing source exists using the last remembered
    /// physical HyperX PipeWire node as the direct capture target.
    pub fn ensure_direct_source(&self, device_id: &DeviceId) -> Result<(), ForgeHxError> {
        let key = device_key(device_id);
        if self.runtimes.is_active(&key) {
            return Ok(());
        }
        let mut active = self.read_active(device_id)?.ok_or_else(|| {
            ForgeHxError::AudioUnavailable(
                "ForgeHX has no remembered physical microphone target yet".into(),
            )
        })?;
        let mut config = self.load(&active.profile_name).unwrap_or_default();
        if !config.enabled {
            config.enabled = true;
        }
        self.save(&config)?;
        if active.profile_name != config.name {
            active.profile_name = config.name.clone();
            self.write_active(device_id, &active)?;
        }
        self.cleanup_legacy_bridge(device_id)?;
        self.runtimes
            .start(
                &key,
                &active.raw_source_node_name,
                &runtime_stream_identity(device_id),
                config,
            )
            .map_err(ForgeHxError::AudioUnavailable)
    }

    pub fn ensure_always_on(
        &self,
        device_id: &DeviceId,
        raw_source_node_id: u32,
    ) -> Result<MicrophoneDspState, ForgeHxError> {
        let raw_source_node_name = resolve_node_target(raw_source_node_id)?;
        self.ensure_always_on_with_source_name(device_id, &raw_source_node_name)
    }

    pub fn ensure_always_on_with_source_name(
        &self,
        device_id: &DeviceId,
        raw_source_node_name: &str,
    ) -> Result<MicrophoneDspState, ForgeHxError> {
        let current = self.read_active(device_id)?;
        let (mut config, mut needs_save) = match &current {
            Some(active) => match self.load(&active.profile_name) {
                Ok(config) => (config, false),
                Err(_) => (MicrophoneDspConfig::default(), true),
            },
            None => (MicrophoneDspConfig::default(), true),
        };
        if !config.enabled {
            config.enabled = true;
            needs_save = true;
        }
        if needs_save {
            self.save(&config)?;
        }
        self.cleanup_legacy_bridge(device_id)?;

        let key = device_key(device_id);
        let source_changed = current
            .as_ref()
            .map(|active| active.raw_source_node_name.as_str())
            != Some(raw_source_node_name);
        let profile_changed = current.as_ref().map(|active| active.profile_name.as_str())
            != Some(config.name.as_str());
        if current.is_none() || source_changed || profile_changed {
            self.write_active(
                device_id,
                &ActiveMicState {
                    profile_name: config.name.clone(),
                    raw_source_node_name: raw_source_node_name.to_owned(),
                },
            )?;
        }

        if !self.runtimes.is_active(&key) || source_changed {
            self.runtimes
                .start(
                    &key,
                    raw_source_node_name,
                    &runtime_stream_identity(device_id),
                    config.clone(),
                )
                .map_err(ForgeHxError::AudioUnavailable)?;
        } else {
            self.runtimes
                .update(&key, config.clone())
                .map_err(ForgeHxError::AudioUnavailable)?;
        }
        self.state(device_id)
    }

    pub fn live_update(
        &self,
        device_id: &DeviceId,
        config: &MicrophoneDspConfig,
    ) -> Result<MicrophoneDspState, ForgeHxError> {
        let mut config = config.clone();
        config.enabled = true;
        config.validate()?;
        self.save(&config)?;
        let key = device_key(device_id);
        if let Some(mut active) = self.read_active(device_id)? {
            if active.profile_name != config.name {
                active.profile_name = config.name.clone();
                self.write_active(device_id, &active)?;
            }
            if self.runtimes.is_active(&key) {
                self.runtimes
                    .update(&key, config.clone())
                    .map_err(ForgeHxError::AudioUnavailable)?;
            } else {
                self.runtimes
                    .start(
                        &key,
                        &active.raw_source_node_name,
                        &runtime_stream_identity(device_id),
                        config.clone(),
                    )
                    .map_err(ForgeHxError::AudioUnavailable)?;
            }
        }
        let mut state = self.state(device_id)?;
        state.config = config;
        Ok(state)
    }

    /// AetherStream owns system/default microphone publication and routing.
    pub fn set_monitor(
        &self,
        device_id: &DeviceId,
        enabled: bool,
        monitor_target: Option<String>,
        level_percent: f32,
    ) -> Result<MicrophoneDspState, ForgeHxError> {
        if !(0.0..=100.0).contains(&level_percent) {
            return Err(ForgeHxError::InvalidProfile(
                "microphone monitor level must be 0..=100 percent".into(),
            ));
        }
        let key = device_key(device_id);
        if !self.runtimes.is_active(&key) {
            if enabled {
                return Err(ForgeHxError::AudioUnavailable(
                    "ForgeHX Mic is not active; live monitoring cannot start".into(),
                ));
            }
            return self.state(device_id);
        }
        self.runtimes
            .set_monitor(&key, enabled, monitor_target, level_percent)
            .map_err(ForgeHxError::AudioUnavailable)?;
        self.state(device_id)
    }

    pub fn start_voice_enrollment(
        &self,
        device_id: &DeviceId,
    ) -> Result<MicrophoneDspState, ForgeHxError> {
        let key = device_key(device_id);
        self.runtimes
            .start_enrollment(&key)
            .map_err(ForgeHxError::AudioUnavailable)?;
        self.state(device_id)
    }

    pub fn cancel_voice_enrollment(
        &self,
        device_id: &DeviceId,
    ) -> Result<MicrophoneDspState, ForgeHxError> {
        let key = device_key(device_id);
        self.runtimes
            .cancel_enrollment(&key)
            .map_err(ForgeHxError::AudioUnavailable)?;
        self.state(device_id)
    }

    pub fn forget_voice(&self, device_id: &DeviceId) -> Result<MicrophoneDspState, ForgeHxError> {
        let key = device_key(device_id);
        if let Some(active) = self.read_active(device_id)? {
            let mut config = self.load(&active.profile_name).unwrap_or_default();
            config.speaker_lock.voiceprint.clear();
            self.save(&config)?;
            if self.runtimes.is_active(&key) {
                let _ = self.runtimes.update(&key, config);
                let _ = self.runtimes.forget_voice(&key);
            }
        }
        self.state(device_id)
    }

    pub fn bypass(&self, _device_id: &DeviceId) -> Result<(), ForgeHxError> {
        Err(ForgeHxError::AudioUnavailable(
            "permanent DSP cannot be bypassed".into(),
        ))
    }

    fn cleanup_legacy_bridge(&self, device_id: &DeviceId) -> Result<(), ForgeHxError> {
        // Remove the pre-10.0.12 PipeWire drop-in if an older ForgeHX release left
        // one behind. The 10.0.12 path is entirely daemon-owned and does not need
        // a PipeWire/WirePlumber restart for normal microphone reconnects.
        let legacy = self.dropin_path(device_id);
        if legacy.exists() {
            fs::remove_file(legacy).map_err(io_error)?;
        }
        Ok(())
    }

    fn write_active(
        &self,
        device_id: &DeviceId,
        active: &ActiveMicState,
    ) -> Result<(), ForgeHxError> {
        fs::create_dir_all(self.active_dir()).map_err(io_error)?;
        atomic_write(
            &self.active_path(device_id),
            &serde_json::to_vec_pretty(active)
                .map_err(|error| ForgeHxError::InvalidProfile(error.to_string()))?,
        )
    }

    fn read_active(&self, device_id: &DeviceId) -> Result<Option<ActiveMicState>, ForgeHxError> {
        let path = self.active_path(device_id);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path).map_err(io_error)?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| ForgeHxError::InvalidProfile(format!("{}: {error}", path.display())))
    }
}

fn runtime_stream_identity(device_id: &DeviceId) -> String {
    format!("forgehx_bridge_{}", device_key(device_id))
}

fn device_key(device_id: &DeviceId) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in device_id.0.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn validate_profile_name(name: &str) -> Result<(), ForgeHxError> {
    if name.trim().is_empty()
        || name.len() > 96
        || name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
    {
        Err(ForgeHxError::InvalidProfile(
            "invalid microphone DSP profile name".into(),
        ))
    } else {
        Ok(())
    }
}

fn io_error(error: std::io::Error) -> ForgeHxError {
    ForgeHxError::Io(error.to_string())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ForgeHxError> {
    let parent = path
        .parent()
        .ok_or_else(|| ForgeHxError::Io("microphone DSP path has no parent".into()))?;
    fs::create_dir_all(parent).map_err(io_error)?;
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("file")
    ));
    fs::write(&tmp, bytes).map_err(io_error)?;
    fs::rename(tmp, path).map_err(io_error)
}

fn wait_for_source_node_id(source_name: &str) -> Result<u32, ForgeHxError> {
    for _ in 0..20 {
        let output = Command::new("wpctl")
            .args(["status", "-n"])
            .output()
            .map_err(|error| {
                ForgeHxError::AudioUnavailable(format!("wpctl is not available: {error}"))
            })?;
        if output.status.success() {
            if let Some(id) =
                parse_named_source_node_id(&String::from_utf8_lossy(&output.stdout), source_name)
            {
                return Ok(id);
            }
        }
        if let Some(id) = find_source_node_id_via_pw_cli(source_name) {
            return Ok(id);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err(ForgeHxError::AudioUnavailable(format!(
        "ForgeHX Mic source {source_name} did not appear in the WirePlumber view or PipeWire registry"
    )))
}

fn find_source_node_id_via_pw_cli(source_name: &str) -> Option<u32> {
    let output = Command::new("pw-cli").args(["ls", "Node"]).output().ok()?;
    output.status.success().then_some(())?;
    parse_pw_cli_named_node_id(&String::from_utf8_lossy(&output.stdout), source_name)
}

pub fn parse_pw_cli_named_node_id(text: &str, source_name: &str) -> Option<u32> {
    let mut current_id = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if let Some(rest) = line.strip_prefix("id ") {
            current_id = rest
                .split(',')
                .next()
                .and_then(|value| value.trim().parse::<u32>().ok());
            continue;
        }
        let Some(value) = line.strip_prefix("node.name =") else {
            continue;
        };
        if value.trim().trim_matches('"') == source_name {
            return current_id;
        }
    }
    None
}

pub fn parse_named_source_node_id(status: &str, source_name: &str) -> Option<u32> {
    let mut in_sources = false;
    for raw_line in status.lines() {
        let line = raw_line.trim();
        if line.contains("Sources:") {
            in_sources = true;
            continue;
        }
        if in_sources && line.ends_with(':') && !line.contains("Sources:") {
            in_sources = false;
        }
        if !in_sources || !line.contains(source_name) {
            continue;
        }
        let digit_start = line.char_indices().find(|(_, ch)| ch.is_ascii_digit())?.0;
        let tail = &line[digit_start..];
        let dot = tail.find('.')?;
        if let Ok(id) = tail[..dot].trim().parse::<u32>() {
            return Some(id);
        }
    }
    None
}

fn resolve_node_target(node_id: u32) -> Result<String, ForgeHxError> {
    let output = Command::new("wpctl")
        .args(["inspect", &node_id.to_string()])
        .output()
        .map_err(|error| {
            ForgeHxError::AudioUnavailable(format!("wpctl inspect is not available: {error}"))
        })?;
    if !output.status.success() {
        return Err(ForgeHxError::AudioUnavailable(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    parse_pipewire_target(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| ForgeHxError::AudioUnavailable(format!("PipeWire source {node_id} has neither node.name nor object.serial; processed-mic routing for this source is unavailable")))
}

fn parse_pipewire_property(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} =");
    text.lines().find_map(|line| {
        let line = line.trim().trim_start_matches('*').trim();
        let value = line
            .strip_prefix(&prefix)?
            .trim()
            .trim_matches('"')
            .trim()
            .to_owned();
        (!value.is_empty()).then_some(value)
    })
}

pub fn parse_pipewire_target(text: &str) -> Option<String> {
    parse_pipewire_property(text, "node.name").or_else(|| {
        parse_pipewire_property(text, "object.serial")
            .filter(|serial| serial.parse::<u64>().is_ok())
    })
}

pub fn parse_node_name(text: &str) -> Option<String> {
    parse_pipewire_property(text, "node.name")
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::MicrophoneDspConfig;

    fn test_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("forgehx-mic-dsp-{name}-{}", std::process::id()))
    }

    #[test]
    fn pw_cli_registry_finds_virtual_processed_source_outside_wpctl_sources_view() {
        let text = r#"id 87, type PipeWire:Interface:Node/3
    object.serial = "413543"
    node.description = "ForgeHX Mic"
    node.name = "forgehx_processed_mic_dev123"
    media.class = "Audio/Source"
"#;
        assert_eq!(
            parse_pw_cli_named_node_id(text, "forgehx_processed_mic_dev123"),
            Some(87)
        );
    }

    #[test]
    fn direct_runtime_targets_aetherstream_system_microphone() {
        let id = DeviceId("hyperx-mic".into());
        assert!(runtime_stream_identity(&id).starts_with("forgehx_bridge_"));
        assert_eq!(
            AETHERSTREAM_SYSTEM_SOURCE_NAME,
            "aetherstream.system.microphone"
        );
    }

    #[test]
    fn profile_store_is_confined_to_config_root() {
        let root = test_root("profile");
        let manager = MicDspManager::new(root.clone());
        let config = MicrophoneDspConfig::default();
        manager.save(&config).unwrap();
        assert!(root
            .join("forgehx/microphone/profiles/Broadcast Full.json")
            .exists());
        let mut bad = config;
        bad.name = "../escape".into();
        assert!(manager.save(&bad).is_err());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parses_stable_pipewire_node_name() {
        assert_eq!(
            parse_node_name("  node.name = \"alsa_input.usb-HyperX-00.mono\"\n"),
            Some("alsa_input.usb-HyperX-00.mono".into())
        );
    }

    #[test]
    fn falls_back_to_pipewire_object_serial_when_node_name_is_missing() {
        assert_eq!(
            parse_pipewire_target("  object.serial = \"734\"\n"),
            Some("734".into())
        );
        assert_eq!(
            parse_pipewire_target("* object.serial = 735\n"),
            Some("735".into())
        );
    }

    #[test]
    fn pipewire_target_prefers_node_name_over_serial() {
        assert_eq!(
            parse_pipewire_target(
                "  object.serial = 734\n  node.name = \"alsa_input.usb-HyperX-00.mono\"\n"
            ),
            Some("alsa_input.usb-HyperX-00.mono".into())
        );
    }
}
