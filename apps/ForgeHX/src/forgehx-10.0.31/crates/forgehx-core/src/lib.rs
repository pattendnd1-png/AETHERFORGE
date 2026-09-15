use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

pub mod output_dsp;
pub use output_dsp::{
    OutputDeviceClass, OutputDspProfile, OutputDspProfileError, OutputEnhancement, OutputFilter,
    OutputFilterKind, OutputGamingMode, OutputLimiter,
};

pub const IPC_PROTOCOL_VERSION: u32 = 12;
pub const PRESERVED_AUDIO_ENDPOINT: &str = "HyperX SoloCast 2 Analog Stereo";
pub const IPC_MIN_PROTOCOL_VERSION: u32 = 2;
pub const PROFILE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Diagnostics,
    Lighting,
    Dpi,
    PollingRate,
    Profiles,
    Bindings,
    MacroAssignments,
    Audio,
    Microphone,
    MicDsp,
    MicFirmwareInventory,
    MicFirmwareUpdate,
    MicFirmwareRecovery,
    Eq,
    AudioRouting,
    BatteryStatus,
    Volume,
    Mute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    ForgeHxNative,
    ForgeHxDsp,
    LinuxStandard,
    OpenRgb,
    Ratbag,
    Diagnostic,
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ForgeHxNative => "ForgeHX Native",
            Self::ForgeHxDsp => "ForgeHX DSP",
            Self::LinuxStandard => "Linux Standard",
            Self::OpenRgb => "OpenRGB",
            Self::Ratbag => "libratbag",
            Self::Diagnostic => "Diagnostic",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendHealth {
    Ready,
    Unavailable,
    Disconnected,
    PermissionDenied,
    ProtocolMismatch,
    AmbiguousDeviceMatch,
    BackendError(String),
}

impl BackendHealth {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendStatus {
    pub backend: BackendKind,
    pub health: BackendHealth,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityOwner {
    pub capability: Capability,
    pub backend: BackendKind,
    pub backend_device_id: Option<String>,
    pub verified: bool,
    pub writable: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VendorFamily {
    HyperX,
    Kingston,
    Hp,
    Logitech,
    Corsair,
    Razer,
    SteelSeries,
    Generic(String),
    Unknown,
}

impl fmt::Display for VendorFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HyperX => f.write_str("HyperX"),
            Self::Kingston => f.write_str("Kingston"),
            Self::Hp => f.write_str("HP"),
            Self::Logitech => f.write_str("Logitech"),
            Self::Corsair => f.write_str("Corsair"),
            Self::Razer => f.write_str("Razer"),
            Self::SteelSeries => f.write_str("SteelSeries"),
            Self::Generic(name) => f.write_str(name),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClass {
    Keyboard,
    Mouse,
    Headset,
    Microphone,
    Controller,
    Webcam,
    Mousepad,
    Monitor,
    AudioInterface,
    UsbAudio,
    GenericHid,
    Other,
}

impl fmt::Display for DeviceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Keyboard => "Keyboard",
            Self::Mouse => "Mouse",
            Self::Headset => "Headset",
            Self::Microphone => "Microphone",
            Self::Controller => "Controller",
            Self::Webcam => "Webcam",
            Self::Mousepad => "Mousepad",
            Self::Monitor => "Monitor",
            Self::AudioInterface => "Audio Interface",
            Self::UsbAudio => "USB Audio",
            Self::GenericHid => "HID Device",
            Self::Other => "Other",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportLevel {
    FullySupported,
    PartiallySupported,
    GenericControls,
    DiagnosticOnly,
    Unavailable,
}

impl fmt::Display for SupportLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::FullySupported => "Fully Supported",
            Self::PartiallySupported => "Partially Supported",
            Self::GenericControls => "Generic Controls",
            Self::DiagnosticOnly => "Diagnostic Only",
            Self::Unavailable => "Unavailable",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    Usb,
    Hid,
    Audio,
    Composite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceSource {
    Hid,
    Usb,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInterface {
    pub source: InterfaceSource,
    pub path: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub interface_number: Option<i32>,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
    pub audio_node_id: Option<u32>,
    pub usb_parent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub manufacturer: Option<String>,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial: Option<String>,
    pub vendor_family: VendorFamily,
    pub device_class: DeviceClass,
    pub support_level: SupportLevel,
    pub connection_kind: ConnectionKind,
    pub interfaces: Vec<DeviceInterface>,
    /// Compatibility projection: capabilities exposed by verified native drivers.
    pub capabilities: Vec<Capability>,
    /// Compatibility projection: capabilities exposed by safe Linux-standard interfaces.
    pub generic_capabilities: Vec<Capability>,
    #[serde(default)]
    pub capability_owners: Vec<CapabilityOwner>,
    pub battery_percent: Option<u8>,
    pub battery_state: Option<String>,
    /// Verified ForgeHX native protocol/driver ID. None blocks raw vendor writes.
    pub protocol: Option<String>,
}

impl DeviceInfo {
    pub fn selected_owner(&self, capability: Capability) -> Option<&CapabilityOwner> {
        self.capability_owners
            .iter()
            .find(|owner| owner.capability == capability)
    }

    pub fn supports(&self, capability: Capability) -> bool {
        self.selected_owner(capability).is_some()
            || self.capabilities.contains(&capability)
            || self.generic_capabilities.contains(&capability)
    }

    pub fn supports_vendor(&self, capability: Capability) -> bool {
        self.selected_owner(capability)
            .map(|owner| {
                owner.backend == BackendKind::ForgeHxNative && owner.writable && owner.verified
            })
            .unwrap_or_else(|| self.protocol.is_some() && self.capabilities.contains(&capability))
    }

    pub fn supports_generic(&self, capability: Capability) -> bool {
        self.selected_owner(capability)
            .map(|owner| owner.backend == BackendKind::LinuxStandard && owner.writable)
            .unwrap_or_else(|| self.generic_capabilities.contains(&capability))
    }

    pub fn all_capabilities(&self) -> Vec<Capability> {
        let mut values: Vec<_> = self
            .capability_owners
            .iter()
            .map(|owner| owner.capability)
            .collect();
        values.extend(self.capabilities.iter().copied());
        values.extend(self.generic_capabilities.iter().copied());
        values.sort();
        values.dedup();
        values
    }

    pub fn is_hyperx(&self) -> bool {
        self.vendor_family == VendorFamily::HyperX
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightingEffect {
    Static,
    Breathing,
    Wave,
    Spectrum,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightingConfig {
    pub effect: LightingEffect,
    pub color: [u8; 3],
    pub brightness: u8,
    pub speed: u8,
}

impl Default for LightingConfig {
    fn default() -> Self {
        Self {
            effect: LightingEffect::Static,
            color: [255, 255, 255],
            brightness: 100,
            speed: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DpiConfig {
    pub stages: Vec<u16>,
    pub active_stage: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MouseConfig {
    #[serde(default)]
    pub dpi: DpiConfig,
    pub report_rate_hz: Option<u16>,
    pub active_profile: Option<u8>,
    #[serde(default)]
    pub button_assignments: Vec<KeyBinding>,
    #[serde(default)]
    pub lift_off_distance_mm: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqBand {
    pub hz: u32,
    pub gain_db: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqFilterKind {
    Peaking,
    LowShelf,
    HighShelf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParametricEqBand {
    pub enabled: bool,
    pub kind: EqFilterKind,
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqConfig {
    pub name: String,
    pub preamp_db: f32,
    #[serde(default)]
    pub filters: Vec<ParametricEqBand>,
    #[serde(default = "default_channels")]
    pub channels: u8,
    #[serde(default = "default_channel_map")]
    pub channel_map: Vec<String>,
    pub target_node_id: Option<u32>,
}

fn default_channels() -> u8 {
    2
}
fn default_channel_map() -> Vec<String> {
    vec!["FL".into(), "FR".into()]
}

impl Default for EqConfig {
    fn default() -> Self {
        Self {
            name: "Flat".into(),
            preamp_db: 0.0,
            filters: Vec::new(),
            channels: default_channels(),
            channel_map: default_channel_map(),
            target_node_id: None,
        }
    }
}

impl EqConfig {
    pub fn validate(&self) -> Result<(), ForgeHxError> {
        if self.name.trim().is_empty() || self.name.contains('/') || self.name.contains('\\') {
            return Err(ForgeHxError::InvalidProfile("EQ name is invalid".into()));
        }
        if !(-30.0..=12.0).contains(&self.preamp_db) {
            return Err(ForgeHxError::InvalidProfile(
                "EQ preamp must be -30dB..=12dB".into(),
            ));
        }
        if self.channels == 0
            || self.channels > 8
            || self.channel_map.len() != self.channels as usize
        {
            return Err(ForgeHxError::InvalidProfile(
                "EQ channel map does not match channel count".into(),
            ));
        }
        if self.filters.len() > 32 {
            return Err(ForgeHxError::InvalidProfile(
                "EQ supports at most 32 filters".into(),
            ));
        }
        for band in &self.filters {
            if !(10.0..=24000.0).contains(&band.frequency_hz)
                || !(-24.0..=24.0).contains(&band.gain_db)
                || !(0.05..=20.0).contains(&band.q)
            {
                return Err(ForgeHxError::InvalidProfile(
                    "EQ band is outside safe limits".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioConfig {
    pub volume: f32,
    pub muted: bool,
    #[serde(default)]
    pub eq: Vec<EqBand>,
    #[serde(default)]
    pub routing_target: Option<String>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            volume: 1.0,
            muted: false,
            eq: Vec::new(),
            routing_target: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MicEqFilterKind {
    #[default]
    Bell,
    LowShelf,
    HighShelf,
    HighPass,
    LowPass,
    Notch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MicEqBand {
    pub enabled: bool,
    #[serde(default)]
    pub kind: MicEqFilterKind,
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseSuppressionConfig {
    pub enabled: bool,
    pub vad_threshold_percent: f32,
    pub grace_ms: u32,
    pub retroactive_grace_ms: u32,
    #[serde(default = "default_noise_strength")]
    pub strength_percent: f32,
}

fn default_noise_strength() -> f32 {
    45.0
}

impl Default for NoiseSuppressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            vad_threshold_percent: 96.0,
            grace_ms: 120,
            retroactive_grace_ms: 0,
            strength_percent: 95.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EchoCancellationConfig {
    pub enabled: bool,
    pub automatic_reference: bool,
    pub automatic_delay: bool,
    pub residual_suppression: bool,
    pub strength_percent: f32,
}

impl Default for EchoCancellationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            automatic_reference: true,
            automatic_delay: true,
            residual_suppression: true,
            strength_percent: 100.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeakerLockConfig {
    pub enabled: bool,
    #[serde(default)]
    pub voiceprint: Vec<f32>,
    #[serde(default = "default_continuous_voice_learning")]
    pub continuous_learning: bool,
    pub match_threshold: f32,
    pub min_speech_db: f32,
}

fn default_continuous_voice_learning() -> bool {
    true
}

impl Default for SpeakerLockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            voiceprint: Vec::new(),
            continuous_learning: true,
            match_threshold: 0.86,
            min_speech_db: -52.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaybackRejectionConfig {
    pub enabled: bool,
    pub hard_block: bool,
    pub correlation_threshold: f32,
    pub history_ms: u32,
}

impl Default for PlaybackRejectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hard_block: true,
            correlation_threshold: 0.62,
            history_ms: 2000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClickSuppressionConfig {
    pub enabled: bool,
    pub sensitivity_percent: f32,
    pub suppression_db: f32,
    pub max_click_ms: f32,
    pub recovery_ms: f32,
}

impl Default for ClickSuppressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity_percent: 90.0,
            suppression_db: 54.0,
            max_click_ms: 14.0,
            recovery_ms: 18.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceIsolationTelemetry {
    pub enrolled: bool,
    pub enrollment_active: bool,
    pub enrollment_progress_percent: u8,
    pub speaker_match_score: f32,
    pub playback_leak_score: f32,
    pub speaker_rejected: bool,
    pub playback_rejected: bool,
}

impl Default for VoiceIsolationTelemetry {
    fn default() -> Self {
        Self {
            enrolled: false,
            enrollment_active: false,
            enrollment_progress_percent: 0,
            speaker_match_score: 0.0,
            playback_leak_score: 0.0,
            speaker_rejected: false,
            playback_rejected: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToneControlsConfig {
    pub bass_boost_db: f32,
    pub bass_frequency_hz: f32,
    pub treble_boost_db: f32,
    pub treble_frequency_hz: f32,
}

impl Default for ToneControlsConfig {
    fn default() -> Self {
        Self {
            bass_boost_db: 2.0,
            bass_frequency_hz: 120.0,
            treble_boost_db: 1.5,
            treble_frequency_hz: 9000.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultibandEqBand {
    pub enabled: bool,
    pub low_hz: f32,
    pub high_hz: f32,
    pub gain_db: f32,
}

fn default_multiband_eq() -> Vec<MultibandEqBand> {
    vec![
        MultibandEqBand {
            enabled: true,
            low_hz: 20.0,
            high_hz: 180.0,
            gain_db: 0.0,
        },
        MultibandEqBand {
            enabled: true,
            low_hz: 180.0,
            high_hz: 700.0,
            gain_db: 0.0,
        },
        MultibandEqBand {
            enabled: true,
            low_hz: 700.0,
            high_hz: 4500.0,
            gain_db: 0.0,
        },
        MultibandEqBand {
            enabled: true,
            low_hz: 4500.0,
            high_hz: 20000.0,
            gain_db: 0.0,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicEqBand {
    pub enabled: bool,
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}

fn default_dynamic_eq() -> Vec<DynamicEqBand> {
    vec![
        DynamicEqBand {
            enabled: true,
            frequency_hz: 240.0,
            gain_db: -3.0,
            q: 1.2,
            threshold_db: -24.0,
            ratio: 2.0,
            attack_ms: 18.0,
            release_ms: 180.0,
        },
        DynamicEqBand {
            enabled: true,
            frequency_hz: 3500.0,
            gain_db: 2.0,
            q: 1.0,
            threshold_db: -30.0,
            ratio: 1.5,
            attack_ms: 25.0,
            release_ms: 220.0,
        },
        DynamicEqBand {
            enabled: true,
            frequency_hz: 6800.0,
            gain_db: -3.0,
            q: 2.0,
            threshold_db: -20.0,
            ratio: 3.0,
            attack_ms: 4.0,
            release_ms: 90.0,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateConfig {
    pub enabled: bool,
    pub open_threshold_db: f32,
    pub close_threshold_db: f32,
    pub attack_ms: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            open_threshold_db: -46.0,
            close_threshold_db: -52.0,
            attack_ms: 4.0,
            hold_ms: 70.0,
            release_ms: 90.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressorConfig {
    pub enabled: bool,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_db: f32,
    #[serde(default = "default_compressor_knee")]
    pub knee_db: f32,
    #[serde(default = "default_mix_percent")]
    pub mix_percent: f32,
}

fn default_compressor_knee() -> f32 {
    6.0
}
fn default_mix_percent() -> f32 {
    100.0
}

impl Default for CompressorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_db: 0.0,
            knee_db: default_compressor_knee(),
            mix_percent: default_mix_percent(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultibandCompressorBand {
    pub enabled: bool,
    pub low_hz: f32,
    pub high_hz: f32,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_db: f32,
}

fn default_multiband_compressor() -> Vec<MultibandCompressorBand> {
    vec![
        MultibandCompressorBand {
            enabled: true,
            low_hz: 20.0,
            high_hz: 180.0,
            threshold_db: -18.0,
            ratio: 2.0,
            attack_ms: 20.0,
            release_ms: 180.0,
            makeup_db: 0.0,
        },
        MultibandCompressorBand {
            enabled: true,
            low_hz: 180.0,
            high_hz: 700.0,
            threshold_db: -20.0,
            ratio: 2.5,
            attack_ms: 12.0,
            release_ms: 150.0,
            makeup_db: 0.0,
        },
        MultibandCompressorBand {
            enabled: true,
            low_hz: 700.0,
            high_hz: 4500.0,
            threshold_db: -18.0,
            ratio: 2.2,
            attack_ms: 8.0,
            release_ms: 120.0,
            makeup_db: 0.0,
        },
        MultibandCompressorBand {
            enabled: true,
            low_hz: 4500.0,
            high_hz: 20000.0,
            threshold_db: -16.0,
            ratio: 1.8,
            attack_ms: 4.0,
            release_ms: 100.0,
            makeup_db: 0.0,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeEsserConfig {
    pub enabled: bool,
    pub frequency_hz: f32,
    pub threshold_db: f32,
    pub ratio: f32,
    #[serde(default = "default_deesser_attack")]
    pub attack_ms: f32,
    #[serde(default = "default_deesser_release")]
    pub release_ms: f32,
    #[serde(default = "default_deesser_amount")]
    pub amount_percent: f32,
}

fn default_deesser_attack() -> f32 {
    3.0
}
fn default_deesser_release() -> f32 {
    80.0
}
fn default_deesser_amount() -> f32 {
    60.0
}

impl Default for DeEsserConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            frequency_hz: 6500.0,
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: default_deesser_attack(),
            release_ms: default_deesser_release(),
            amount_percent: default_deesser_amount(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceEnhancerConfig {
    pub warmth: f32,
    pub body: f32,
    pub clarity: f32,
    pub presence: f32,
    pub air: f32,
    pub depth: f32,
}

impl Default for VoiceEnhancerConfig {
    fn default() -> Self {
        Self {
            warmth: 45.0,
            body: 55.0,
            clarity: 65.0,
            presence: 50.0,
            air: 35.0,
            depth: 35.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaturationConfig {
    pub enabled: bool,
    pub drive_db: f32,
    pub mix_percent: f32,
}

impl Default for SaturationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            drive_db: 1.5,
            mix_percent: 12.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LimiterConfig {
    pub enabled: bool,
    pub ceiling_db: f32,
    pub release_ms: f32,
    #[serde(default = "default_limiter_lookahead")]
    pub lookahead_ms: f32,
    #[serde(default = "default_true_peak")]
    pub true_peak: bool,
}

fn default_limiter_lookahead() -> f32 {
    2.0
}
fn default_true_peak() -> bool {
    true
}

impl Default for LimiterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ceiling_db: -1.0,
            release_ms: 5.0,
            lookahead_ms: default_limiter_lookahead(),
            true_peak: default_true_peak(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VoicePilotMode {
    #[default]
    Auto,
    Manual,
    Locked,
    Bypassed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum VoicePilotTarget {
    Natural,
    Clear,
    Warm,
    Full,
    Broadcast,
    #[default]
    BroadcastFull,
    DeepAndFull,
    CrispStream,
    VoiceChat,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoicePilotConfig {
    pub enabled: bool,
    pub target: VoicePilotTarget,
    pub adaptation_strength: f32,
    pub auto_noise: VoicePilotMode,
    pub auto_aec: VoicePilotMode,
    pub auto_eq: VoicePilotMode,
    pub auto_tone: VoicePilotMode,
    pub auto_dynamics: VoicePilotMode,
    pub auto_de_esser: VoicePilotMode,
    pub auto_loudness: VoicePilotMode,
}

impl Default for VoicePilotConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target: VoicePilotTarget::BroadcastFull,
            adaptation_strength: 50.0,
            auto_noise: VoicePilotMode::Auto,
            auto_aec: VoicePilotMode::Auto,
            auto_eq: VoicePilotMode::Auto,
            auto_tone: VoicePilotMode::Auto,
            auto_dynamics: VoicePilotMode::Auto,
            auto_de_esser: VoicePilotMode::Auto,
            auto_loudness: VoicePilotMode::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoicePilotTelemetry {
    pub active: bool,
    pub reference_available: bool,
    pub input_rms_db: f32,
    pub output_rms_db: f32,
    pub noise_floor_db: f32,
    pub bass_target_db: f32,
    pub treble_target_db: f32,
    pub presence_target_db: f32,
    pub de_esser_reduction_db: f32,
    pub compressor_reduction_db: f32,
}

impl Default for VoicePilotTelemetry {
    fn default() -> Self {
        Self {
            active: false,
            reference_available: false,
            input_rms_db: -90.0,
            output_rms_db: -90.0,
            noise_floor_db: -90.0,
            bass_target_db: 0.0,
            treble_target_db: 0.0,
            presence_target_db: 0.0,
            de_esser_reduction_db: 0.0,
            compressor_reduction_db: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtraneousNoiseMonitorConfig {
    pub enabled: bool,
    pub auto_adapt: bool,
    pub sensitivity_percent: f32,
    pub max_adaptive_suppression_db: f32,
    pub speaker_rejection_enabled: bool,
}

impl Default for ExtraneousNoiseMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_adapt: true,
            sensitivity_percent: 85.0,
            max_adaptive_suppression_db: 72.0,
            speaker_rejection_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NoiseAdaptationState {
    #[default]
    Learning,
    Holding,
    Suppressing,
    SpeechProtected,
    ReferenceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseBandLevels {
    pub sub_rumble_dbfs: f32,
    pub low_dbfs: f32,
    pub low_mid_dbfs: f32,
    pub mid_dbfs: f32,
    pub presence_dbfs: f32,
    pub high_dbfs: f32,
    pub air_dbfs: f32,
}

impl Default for NoiseBandLevels {
    fn default() -> Self {
        Self {
            sub_rumble_dbfs: -90.0,
            low_dbfs: -90.0,
            low_mid_dbfs: -90.0,
            mid_dbfs: -90.0,
            presence_dbfs: -90.0,
            high_dbfs: -90.0,
            air_dbfs: -90.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NoiseClassScores {
    pub white_like: f32,
    pub pink_like: f32,
    pub broadband: f32,
    pub hum_rumble: f32,
    pub narrowband_whine: f32,
    pub transient: f32,
    pub speaker_leak: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseSceneTelemetry {
    pub active: bool,
    pub reference_available: bool,
    pub adaptation_state: NoiseAdaptationState,
    pub overall_noise_dbfs: f32,
    pub classes: NoiseClassScores,
    pub band_floor_dbfs: NoiseBandLevels,
    pub dominant_bands: Vec<String>,
    pub adaptive_suppression_db: f32,
    pub sonora_noise_target_percent: f32,
    pub speech_protected: bool,
    pub double_talk: bool,
}

impl Default for NoiseSceneTelemetry {
    fn default() -> Self {
        Self {
            active: false,
            reference_available: false,
            adaptation_state: NoiseAdaptationState::ReferenceUnavailable,
            overall_noise_dbfs: -90.0,
            classes: NoiseClassScores::default(),
            band_floor_dbfs: NoiseBandLevels::default(),
            dominant_bands: Vec::new(),
            adaptive_suppression_db: 0.0,
            sonora_noise_target_percent: 0.0,
            speech_protected: false,
            double_talk: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MicrophoneDspConfig {
    pub name: String,
    pub enabled: bool,
    pub input_gain_db: f32,
    pub high_pass_hz: Option<f32>,
    #[serde(default)]
    pub eq: Vec<MicEqBand>,
    #[serde(default)]
    pub noise_suppression: NoiseSuppressionConfig,
    #[serde(default)]
    pub echo_cancellation: EchoCancellationConfig,
    #[serde(default)]
    pub speaker_lock: SpeakerLockConfig,
    #[serde(default)]
    pub playback_rejection: PlaybackRejectionConfig,
    #[serde(default)]
    pub click_suppression: ClickSuppressionConfig,
    #[serde(default)]
    pub extraneous_noise_monitor: ExtraneousNoiseMonitorConfig,
    #[serde(default)]
    pub tone: ToneControlsConfig,
    #[serde(default = "default_multiband_eq")]
    pub multiband_eq: Vec<MultibandEqBand>,
    #[serde(default = "default_dynamic_eq")]
    pub dynamic_eq: Vec<DynamicEqBand>,
    #[serde(default)]
    pub gate: GateConfig,
    #[serde(default)]
    pub compressor: CompressorConfig,
    #[serde(default = "default_multiband_compressor")]
    pub multiband_compressor: Vec<MultibandCompressorBand>,
    #[serde(default)]
    pub de_esser: DeEsserConfig,
    #[serde(default)]
    pub voice_enhancer: VoiceEnhancerConfig,
    #[serde(default)]
    pub saturation: SaturationConfig,
    #[serde(default)]
    pub limiter: LimiterConfig,
    #[serde(default)]
    pub voicepilot: VoicePilotConfig,
    pub output_gain_db: f32,
}

impl Default for MicrophoneDspConfig {
    fn default() -> Self {
        Self {
            name: "Broadcast Full".into(),
            enabled: true,
            input_gain_db: 0.0,
            high_pass_hz: Some(80.0),
            eq: vec![
                MicEqBand {
                    enabled: true,
                    kind: MicEqFilterKind::Bell,
                    frequency_hz: 250.0,
                    gain_db: -1.5,
                    q: 1.0,
                },
                MicEqBand {
                    enabled: true,
                    kind: MicEqFilterKind::Bell,
                    frequency_hz: 3500.0,
                    gain_db: 1.5,
                    q: 0.9,
                },
                MicEqBand {
                    enabled: true,
                    kind: MicEqFilterKind::HighShelf,
                    frequency_hz: 12000.0,
                    gain_db: 1.0,
                    q: 0.7,
                },
            ],
            noise_suppression: NoiseSuppressionConfig::default(),
            echo_cancellation: EchoCancellationConfig::default(),
            speaker_lock: SpeakerLockConfig::default(),
            playback_rejection: PlaybackRejectionConfig::default(),
            click_suppression: ClickSuppressionConfig::default(),
            extraneous_noise_monitor: ExtraneousNoiseMonitorConfig::default(),
            tone: ToneControlsConfig::default(),
            multiband_eq: default_multiband_eq(),
            dynamic_eq: default_dynamic_eq(),
            gate: GateConfig::default(),
            compressor: CompressorConfig::default(),
            multiband_compressor: default_multiband_compressor(),
            de_esser: DeEsserConfig::default(),
            voice_enhancer: VoiceEnhancerConfig::default(),
            saturation: SaturationConfig::default(),
            limiter: LimiterConfig::default(),
            voicepilot: VoicePilotConfig::default(),
            output_gain_db: 0.0,
        }
    }
}

impl MicrophoneDspConfig {
    pub fn validate(&self) -> Result<(), ForgeHxError> {
        if self.name.trim().is_empty()
            || self.name.len() > 96
            || self.name.contains('/')
            || self.name.contains('\\')
            || self.name == "."
            || self.name == ".."
        {
            return Err(ForgeHxError::InvalidProfile(
                "microphone DSP profile name is invalid".into(),
            ));
        }
        if !(-30.0..=24.0).contains(&self.input_gain_db)
            || !(-30.0..=24.0).contains(&self.output_gain_db)
        {
            return Err(ForgeHxError::InvalidProfile(
                "microphone DSP gain must be -30dB..=24dB".into(),
            ));
        }
        if self
            .high_pass_hz
            .is_some_and(|hz| !(20.0..=500.0).contains(&hz))
        {
            return Err(ForgeHxError::InvalidProfile(
                "microphone high-pass must be 20Hz..=500Hz".into(),
            ));
        }
        if self.eq.len() > 12 {
            return Err(ForgeHxError::InvalidProfile(
                "microphone EQ supports at most 12 bands".into(),
            ));
        }
        for band in &self.eq {
            if !(20.0..=20000.0).contains(&band.frequency_hz)
                || !(-24.0..=24.0).contains(&band.gain_db)
                || !(0.1..=18.0).contains(&band.q)
            {
                return Err(ForgeHxError::InvalidProfile(
                    "microphone EQ band is outside safe limits".into(),
                ));
            }
        }
        if !(0.0..=100.0).contains(&self.noise_suppression.vad_threshold_percent)
            || !(0.0..=100.0).contains(&self.noise_suppression.strength_percent)
            || self.noise_suppression.grace_ms > 2000
            || self.noise_suppression.retroactive_grace_ms > 500
        {
            return Err(ForgeHxError::InvalidProfile(
                "noise suppression parameters are outside safe limits".into(),
            ));
        }
        if !(0.0..=100.0).contains(&self.echo_cancellation.strength_percent) {
            return Err(ForgeHxError::InvalidProfile(
                "echo cancellation strength must be 0..=100".into(),
            ));
        }
        if !(0.50..=0.99).contains(&self.speaker_lock.match_threshold)
            || !(-80.0..=-20.0).contains(&self.speaker_lock.min_speech_db)
            || (!self.speaker_lock.voiceprint.is_empty()
                && self.speaker_lock.voiceprint.len() != 12)
        {
            return Err(ForgeHxError::InvalidProfile(
                "speaker lock parameters or voiceprint are invalid".into(),
            ));
        }
        if !(0.20..=0.99).contains(&self.playback_rejection.correlation_threshold)
            || !(100..=2000).contains(&self.playback_rejection.history_ms)
        {
            return Err(ForgeHxError::InvalidProfile(
                "playback rejection parameters are outside safe limits".into(),
            ));
        }
        if !(0.0..=100.0).contains(&self.click_suppression.sensitivity_percent)
            || !(0.0..=72.0).contains(&self.click_suppression.suppression_db)
            || !(1.0..=30.0).contains(&self.click_suppression.max_click_ms)
            || !(0.0..=100.0).contains(&self.click_suppression.recovery_ms)
        {
            return Err(ForgeHxError::InvalidProfile(
                "keyboard/mouse click suppression parameters are outside safe limits".into(),
            ));
        }
        if !(0.0..=100.0).contains(&self.extraneous_noise_monitor.sensitivity_percent)
            || !(0.0..=72.0).contains(&self.extraneous_noise_monitor.max_adaptive_suppression_db)
        {
            return Err(ForgeHxError::InvalidProfile(
                "extraneous noise monitor parameters are outside safe limits".into(),
            ));
        }
        if !(0.0..=12.0).contains(&self.tone.bass_boost_db)
            || !(40.0..=300.0).contains(&self.tone.bass_frequency_hz)
            || !(0.0..=12.0).contains(&self.tone.treble_boost_db)
            || !(2000.0..=16000.0).contains(&self.tone.treble_frequency_hz)
        {
            return Err(ForgeHxError::InvalidProfile(
                "bass/treble boost parameters are outside safe limits".into(),
            ));
        }
        if self.multiband_eq.len() > 8
            || self.dynamic_eq.len() > 8
            || self.multiband_compressor.len() > 4
        {
            return Err(ForgeHxError::InvalidProfile(
                "multiband DSP exceeds supported band count".into(),
            ));
        }
        for band in &self.multiband_eq {
            if !(20.0..=20000.0).contains(&band.low_hz)
                || !(20.0..=20000.0).contains(&band.high_hz)
                || band.low_hz >= band.high_hz
                || !(-24.0..=24.0).contains(&band.gain_db)
            {
                return Err(ForgeHxError::InvalidProfile(
                    "multiband EQ band is outside safe limits".into(),
                ));
            }
        }
        for band in &self.dynamic_eq {
            if !(20.0..=20000.0).contains(&band.frequency_hz)
                || !(-24.0..=24.0).contains(&band.gain_db)
                || !(0.1..=18.0).contains(&band.q)
                || !(-80.0..=0.0).contains(&band.threshold_db)
                || !(1.0..=20.0).contains(&band.ratio)
                || !(0.1..=500.0).contains(&band.attack_ms)
                || !(1.0..=5000.0).contains(&band.release_ms)
            {
                return Err(ForgeHxError::InvalidProfile(
                    "dynamic EQ band is outside safe limits".into(),
                ));
            }
        }
        if !(-90.0..=0.0).contains(&self.gate.open_threshold_db)
            || !(-90.0..=0.0).contains(&self.gate.close_threshold_db)
            || self.gate.close_threshold_db > self.gate.open_threshold_db
            || !(0.1..=500.0).contains(&self.gate.attack_ms)
            || !(0.0..=5000.0).contains(&self.gate.hold_ms)
            || !(1.0..=5000.0).contains(&self.gate.release_ms)
        {
            return Err(ForgeHxError::InvalidProfile(
                "gate parameters are outside safe limits".into(),
            ));
        }
        if !(-60.0..=0.0).contains(&self.compressor.threshold_db)
            || !(1.0..=20.0).contains(&self.compressor.ratio)
            || !(0.1..=500.0).contains(&self.compressor.attack_ms)
            || !(1.0..=5000.0).contains(&self.compressor.release_ms)
            || !(-24.0..=24.0).contains(&self.compressor.makeup_db)
            || !(0.0..=24.0).contains(&self.compressor.knee_db)
            || !(0.0..=100.0).contains(&self.compressor.mix_percent)
        {
            return Err(ForgeHxError::InvalidProfile(
                "compressor parameters are outside safe limits".into(),
            ));
        }
        for band in &self.multiband_compressor {
            if !(20.0..=20000.0).contains(&band.low_hz)
                || !(20.0..=20000.0).contains(&band.high_hz)
                || band.low_hz >= band.high_hz
                || !(-60.0..=0.0).contains(&band.threshold_db)
                || !(1.0..=20.0).contains(&band.ratio)
                || !(0.1..=500.0).contains(&band.attack_ms)
                || !(1.0..=5000.0).contains(&band.release_ms)
                || !(-24.0..=24.0).contains(&band.makeup_db)
            {
                return Err(ForgeHxError::InvalidProfile(
                    "multiband compressor band is outside safe limits".into(),
                ));
            }
        }
        if !(2000.0..=12000.0).contains(&self.de_esser.frequency_hz)
            || !(-60.0..=0.0).contains(&self.de_esser.threshold_db)
            || !(1.0..=20.0).contains(&self.de_esser.ratio)
            || !(0.1..=100.0).contains(&self.de_esser.attack_ms)
            || !(1.0..=1000.0).contains(&self.de_esser.release_ms)
            || !(0.0..=100.0).contains(&self.de_esser.amount_percent)
        {
            return Err(ForgeHxError::InvalidProfile(
                "de-esser parameters are outside safe limits".into(),
            ));
        }
        for value in [
            self.voice_enhancer.warmth,
            self.voice_enhancer.body,
            self.voice_enhancer.clarity,
            self.voice_enhancer.presence,
            self.voice_enhancer.air,
            self.voice_enhancer.depth,
        ] {
            if !(0.0..=100.0).contains(&value) {
                return Err(ForgeHxError::InvalidProfile(
                    "voice enhancer values must be 0..=100".into(),
                ));
            }
        }
        if !(0.0..=24.0).contains(&self.saturation.drive_db)
            || !(0.0..=100.0).contains(&self.saturation.mix_percent)
        {
            return Err(ForgeHxError::InvalidProfile(
                "saturation parameters are outside safe limits".into(),
            ));
        }
        if !(-12.0..=0.0).contains(&self.limiter.ceiling_db)
            || !(0.25..=20.0).contains(&self.limiter.release_ms)
            || !(0.0..=20.0).contains(&self.limiter.lookahead_ms)
        {
            return Err(ForgeHxError::InvalidProfile(
                "limiter parameters are outside safe limits".into(),
            ));
        }
        if !(0.0..=100.0).contains(&self.voicepilot.adaptation_strength) {
            return Err(ForgeHxError::InvalidProfile(
                "VoicePilot adaptation strength must be 0..=100".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MicrophoneMonitorState {
    pub enabled: bool,
    pub active: bool,
    pub level_percent: f32,
    pub sink_node_name: Option<String>,
    pub error: Option<String>,
}

impl Default for MicrophoneMonitorState {
    fn default() -> Self {
        Self {
            enabled: false,
            active: false,
            level_percent: 50.0,
            sink_node_name: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MicrophoneDspState {
    pub device_id: DeviceId,
    pub config: MicrophoneDspConfig,
    pub applied: bool,
    pub raw_source_node_name: Option<String>,
    pub processed_source_node_name: String,
    #[serde(default)]
    pub unavailable_processors: Vec<String>,
    #[serde(default)]
    pub voicepilot_telemetry: VoicePilotTelemetry,
    #[serde(default)]
    pub voice_isolation_telemetry: VoiceIsolationTelemetry,
    #[serde(default)]
    pub noise_scene_telemetry: NoiseSceneTelemetry,
    #[serde(default)]
    pub monitor: MicrophoneMonitorState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub schema_version: u32,
    pub name: String,
    pub match_vendor_id: Option<u16>,
    pub match_product_id: Option<u16>,
    pub match_serial: Option<String>,
    pub lighting: Option<LightingConfig>,
    /// Legacy v1 DPI field; retained for migration/compatibility.
    #[serde(default)]
    pub dpi: Option<DpiConfig>,
    /// Legacy v1 bindings field; retained for migration/compatibility.
    #[serde(default)]
    pub bindings: Vec<KeyBinding>,
    #[serde(default)]
    pub mouse: Option<MouseConfig>,
    pub audio: Option<AudioConfig>,
    pub microphone: Option<AudioConfig>,
    #[serde(default)]
    pub eq: Option<EqConfig>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            schema_version: PROFILE_SCHEMA_VERSION,
            name: "Default".to_owned(),
            match_vendor_id: None,
            match_product_id: None,
            match_serial: None,
            lighting: None,
            dpi: None,
            bindings: Vec::new(),
            mouse: None,
            audio: None,
            microphone: None,
            eq: None,
        }
    }
}

impl Profile {
    pub fn migrate(mut self) -> Result<Self, ForgeHxError> {
        match self.schema_version {
            PROFILE_SCHEMA_VERSION => Ok(self),
            1 => {
                if self.mouse.is_none() && (self.dpi.is_some() || !self.bindings.is_empty()) {
                    self.mouse = Some(MouseConfig {
                        dpi: self.dpi.clone().unwrap_or_default(),
                        report_rate_hz: None,
                        active_profile: None,
                        button_assignments: self.bindings.clone(),
                        lift_off_distance_mm: None,
                    });
                }
                if self.eq.is_none() {
                    if let Some(audio) = &self.audio {
                        if !audio.eq.is_empty() {
                            self.eq = Some(EqConfig {
                                name: format!("{} EQ", self.name),
                                filters: audio
                                    .eq
                                    .iter()
                                    .map(|band| ParametricEqBand {
                                        enabled: true,
                                        kind: EqFilterKind::Peaking,
                                        frequency_hz: band.hz as f32,
                                        gain_db: band.gain_db,
                                        q: 1.0,
                                    })
                                    .collect(),
                                ..EqConfig::default()
                            });
                        }
                    }
                }
                self.schema_version = PROFILE_SCHEMA_VERSION;
                Ok(self)
            }
            other => Err(ForgeHxError::UnsupportedSchema(other)),
        }
    }

    pub fn validate(&self) -> Result<(), ForgeHxError> {
        if self.schema_version != PROFILE_SCHEMA_VERSION {
            return Err(ForgeHxError::UnsupportedSchema(self.schema_version));
        }
        if self.name.trim().is_empty() {
            return Err(ForgeHxError::InvalidProfile(
                "profile name cannot be empty".into(),
            ));
        }
        if self.name.contains('/')
            || self.name.contains('\\')
            || self.name == "."
            || self.name == ".."
        {
            return Err(ForgeHxError::InvalidProfile(
                "profile name cannot contain path separators".into(),
            ));
        }
        if let Some(lighting) = &self.lighting {
            if lighting.brightness > 100 || lighting.speed > 100 {
                return Err(ForgeHxError::InvalidProfile(
                    "lighting brightness and speed must be between 0 and 100".into(),
                ));
            }
        }
        if let Some(dpi) = &self.dpi {
            validate_dpi(dpi)?;
        }
        if let Some(mouse) = &self.mouse {
            validate_dpi(&mouse.dpi)?;
            if let Some(rate) = mouse.report_rate_hz {
                if ![125, 250, 500, 1000, 2000, 4000, 8000].contains(&rate) {
                    return Err(ForgeHxError::InvalidProfile(
                        "unsupported mouse report rate".into(),
                    ));
                }
            }
        }
        for cfg in [&self.audio, &self.microphone].into_iter().flatten() {
            if !(0.0..=1.5).contains(&cfg.volume) {
                return Err(ForgeHxError::InvalidProfile(
                    "audio volume must be 0.0..=1.5".into(),
                ));
            }
            if cfg
                .eq
                .iter()
                .any(|band| !(-24.0..=24.0).contains(&band.gain_db))
            {
                return Err(ForgeHxError::InvalidProfile(
                    "EQ gain must be -24dB..=24dB".into(),
                ));
            }
        }
        if let Some(eq) = &self.eq {
            eq.validate()?;
        }
        Ok(())
    }
}

fn validate_dpi(dpi: &DpiConfig) -> Result<(), ForgeHxError> {
    if dpi.stages.is_empty() || dpi.stages.len() > 8 {
        return Err(ForgeHxError::InvalidProfile(
            "DPI must contain 1 to 8 stages".into(),
        ));
    }
    if dpi.active_stage >= dpi.stages.len() {
        return Err(ForgeHxError::InvalidProfile(
            "active DPI stage is out of range".into(),
        ));
    }
    if dpi.stages.iter().any(|dpi| *dpi < 100 || *dpi > 30000) {
        return Err(ForgeHxError::InvalidProfile(
            "DPI stages must be 100..=30000".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub device_id: DeviceId,
    pub source: InterfaceSource,
    pub path: String,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
    pub interface_number: Option<i32>,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
    pub audio_node_id: Option<u32>,
    pub usb_parent: Option<String>,
    pub protocol_match: Option<String>,
    pub support_level: SupportLevel,
    pub write_protected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioNode {
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub volume: Option<f32>,
    pub muted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightingModeMetadata {
    pub index: u32,
    pub name: String,
    pub speed_min: u32,
    pub speed_max: u32,
    pub brightness_min: Option<u32>,
    pub brightness_max: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightingZoneMetadata {
    pub index: u32,
    pub name: String,
    pub led_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightingControllerMetadata {
    pub backend_device_id: String,
    pub name: String,
    pub vendor: String,
    pub description: String,
    pub version: String,
    pub serial: String,
    pub location: String,
    pub led_count: u16,
    pub modes: Vec<LightingModeMetadata>,
    pub zones: Vec<LightingZoneMetadata>,
    pub protocol_version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MouseDeviceState {
    pub backend_device_id: String,
    pub active_profile: Option<u8>,
    pub dpi_stages: Vec<u16>,
    pub active_dpi_stage: Option<usize>,
    pub report_rate_hz: Option<u16>,
    pub button_count: Option<u16>,
    #[serde(default)]
    pub button_assignments: Vec<KeyBinding>,
    #[serde(default)]
    pub lift_off_distance_mm: Option<u8>,
    #[serde(default)]
    pub lighting: Option<LightingConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseProtocolFamily {
    HasteV1,
    LegacyPulsefire,
    Dart,
    Haste2,
    Fuse,
    Saga,
    Unknown,
}

impl fmt::Display for MouseProtocolFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::HasteV1 => "Haste v1",
            Self::LegacyPulsefire => "Legacy Pulsefire",
            Self::Dart => "Dart",
            Self::Haste2 => "Haste 2",
            Self::Fuse => "Fuse",
            Self::Saga => "Saga",
            Self::Unknown => "Unknown",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MouseLimits {
    pub dpi_min: Option<u16>,
    pub dpi_max: Option<u16>,
    pub dpi_step: Option<u16>,
    pub max_dpi_stages: Option<u8>,
    #[serde(default)]
    pub polling_rates_hz: Vec<u16>,
    pub button_count: Option<u16>,
    pub onboard_profiles: Option<u8>,
    pub wireless: bool,
    pub battery: bool,
    #[serde(default)]
    pub lift_off_distances_mm: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseModelInfo {
    pub id: String,
    pub name: String,
    pub protocol_family: MouseProtocolFamily,
    pub limits: MouseLimits,
    pub exact_hardware_match: bool,
    pub native_driver: Option<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardProtocolFamily {
    AlloyLegacy,
    AlloyOrigins,
    AlloyRise,
    Origins2,
    Eve,
    Unknown,
}

impl fmt::Display for KeyboardProtocolFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlloyLegacy => "Alloy Legacy",
            Self::AlloyOrigins => "Alloy Origins",
            Self::AlloyRise => "Alloy Rise",
            Self::Origins2 => "Origins 2",
            Self::Eve => "Eve",
            Self::Unknown => "Unknown",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum KeyboardLightingTopology {
    #[default]
    None,
    Zone {
        zones: u16,
    },
    PerKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KeyboardLimits {
    pub lighting: KeyboardLightingTopology,
    #[serde(default)]
    pub polling_rates_hz: Vec<u16>,
    pub onboard_profiles: Option<u8>,
    pub wireless: bool,
    pub battery: bool,
    pub hall_effect: bool,
    pub rapid_trigger: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyboardModelInfo {
    pub id: String,
    pub name: String,
    pub protocol_family: KeyboardProtocolFamily,
    pub limits: KeyboardLimits,
    pub exact_hardware_match: bool,
    pub native_driver: Option<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareSupportLevel {
    InventoryOnly,
    ValidatedPackages,
    UpdateCapable,
    RecoveryCapable,
}

impl FirmwareSupportLevel {
    pub fn can_update(self) -> bool {
        matches!(self, Self::UpdateCapable | Self::RecoveryCapable)
    }

    pub fn can_recover(self) -> bool {
        matches!(self, Self::RecoveryCapable)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareIdentity {
    pub device_id: DeviceId,
    pub model: String,
    pub hardware_revision: Option<String>,
    pub firmware_version: Option<String>,
    pub firmware_version_source: Option<String>,
    pub bootloader_version: Option<String>,
    pub support_level: FirmwareSupportLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareValidation {
    Valid,
    UnsupportedFormat,
    ModelMismatch,
    HardwareRevisionMismatch,
    VersionUnknown,
    HashMismatch,
    SignatureInvalid,
    AmbiguousTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwarePackageInfo {
    pub staged_id: String,
    pub original_filename: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub parsed_format: String,
    pub claimed_model: Option<String>,
    pub claimed_hardware_revision: Option<String>,
    pub claimed_firmware_version: Option<String>,
    pub validation: FirmwareValidation,
    pub vendor_signature_verified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareTransactionState {
    Idle,
    PackageStaged,
    Validated,
    PreflightPassed,
    AudioDetached,
    EnteringUpdateMode,
    AwaitingUpdateDevice,
    Flashing,
    Finalizing,
    AwaitingNormalDevice,
    Verifying,
    RestoringProfile,
    Completed,
    Rejected,
    PreflightFailed,
    UpdateDeviceMissing,
    FlashFailed,
    NormalDeviceMissing,
    VerificationFailed,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirmwareTransactionStatus {
    pub transaction_id: String,
    pub device_id: DeviceId,
    pub staged_id: String,
    pub state: FirmwareTransactionState,
    pub message: Option<String>,
    pub progress_percent: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    Ping {
        protocol_version: u32,
    },
    DaemonStatus {
        protocol_version: u32,
    },
    ListDevices {
        protocol_version: u32,
    },
    ListHyperxDevices {
        protocol_version: u32,
    },
    ListAllDevices {
        protocol_version: u32,
    },
    RefreshDevices {
        protocol_version: u32,
    },
    Doctor {
        protocol_version: u32,
        device_id: Option<DeviceId>,
    },
    ListAudio {
        protocol_version: u32,
    },
    BackendStatus {
        protocol_version: u32,
    },
    BackendRescan {
        protocol_version: u32,
    },
    BackendEnsure {
        protocol_version: u32,
    },
    BackendRestart {
        protocol_version: u32,
        backend: BackendKind,
    },
    ProfileList {
        protocol_version: u32,
    },
    ProfileGet {
        protocol_version: u32,
        name: String,
    },
    ProfileSave {
        protocol_version: u32,
        profile: Profile,
    },
    ProfileApply {
        protocol_version: u32,
        name: String,
        device_id: DeviceId,
    },
    LightingMetadata {
        protocol_version: u32,
        device_id: DeviceId,
    },
    SetLighting {
        protocol_version: u32,
        device_id: DeviceId,
        config: LightingConfig,
    },
    MouseState {
        protocol_version: u32,
        device_id: DeviceId,
    },
    MouseCapabilities {
        protocol_version: u32,
        device_id: DeviceId,
    },
    KeyboardCapabilities {
        protocol_version: u32,
        device_id: DeviceId,
    },
    SetDpi {
        protocol_version: u32,
        device_id: DeviceId,
        config: DpiConfig,
    },
    SetPollingRate {
        protocol_version: u32,
        device_id: DeviceId,
        hz: u16,
    },
    SetMouseProfile {
        protocol_version: u32,
        device_id: DeviceId,
        profile: u8,
    },
    SetButtonAssignment {
        protocol_version: u32,
        device_id: DeviceId,
        button: u32,
        action: String,
    },
    SetLiftOffDistance {
        protocol_version: u32,
        device_id: DeviceId,
        mm: u8,
    },
    AudioSetVolume {
        protocol_version: u32,
        node_id: u32,
        volume: f32,
    },
    AudioSetMute {
        protocol_version: u32,
        node_id: u32,
        muted: bool,
    },
    EqList {
        protocol_version: u32,
    },
    EqGet {
        protocol_version: u32,
        name: String,
    },
    EqSave {
        protocol_version: u32,
        config: EqConfig,
    },
    EqDelete {
        protocol_version: u32,
        name: String,
    },
    EqApply {
        protocol_version: u32,
        name: String,
    },
    EqBypass {
        protocol_version: u32,
    },
    MicDspGet {
        protocol_version: u32,
        device_id: DeviceId,
    },
    MicDspSave {
        protocol_version: u32,
        device_id: DeviceId,
        config: MicrophoneDspConfig,
    },
    MicDspLiveUpdate {
        protocol_version: u32,
        device_id: DeviceId,
        config: MicrophoneDspConfig,
    },
    MicDspApply {
        protocol_version: u32,
        device_id: DeviceId,
        name: String,
    },
    MicDspBypass {
        protocol_version: u32,
        device_id: DeviceId,
    },
    MicVoiceEnrollStart {
        protocol_version: u32,
        device_id: DeviceId,
    },
    MicVoiceEnrollCancel {
        protocol_version: u32,
        device_id: DeviceId,
    },
    MicVoiceForget {
        protocol_version: u32,
        device_id: DeviceId,
    },
    MicMonitorSet {
        protocol_version: u32,
        device_id: DeviceId,
        enabled: bool,
        level_percent: f32,
    },
    MicFirmwareGet {
        protocol_version: u32,
        device_id: DeviceId,
    },
    MicFirmwareStage {
        protocol_version: u32,
        device_id: DeviceId,
        path: String,
    },
    MicFirmwareValidate {
        protocol_version: u32,
        device_id: DeviceId,
        staged_id: String,
    },
    MicFirmwareBegin {
        protocol_version: u32,
        device_id: DeviceId,
        staged_id: String,
    },
    MicFirmwareStatus {
        protocol_version: u32,
        transaction_id: String,
    },
    MicFirmwareForget {
        protocol_version: u32,
        staged_id: String,
    },
    OutputDspGet {
        protocol_version: u32,
    },
    OutputDspLiveUpdate {
        protocol_version: u32,
        profile: OutputDspProfile,
    },
    OutputDspResetFactory {
        protocol_version: u32,
        device_class: OutputDeviceClass,
    },
}

impl Command {
    pub fn protocol_version(&self) -> u32 {
        match self {
            Self::Ping { protocol_version }
            | Self::DaemonStatus { protocol_version }
            | Self::ListDevices { protocol_version }
            | Self::ListHyperxDevices { protocol_version }
            | Self::ListAllDevices { protocol_version }
            | Self::RefreshDevices { protocol_version }
            | Self::Doctor {
                protocol_version, ..
            }
            | Self::ListAudio { protocol_version }
            | Self::BackendStatus { protocol_version }
            | Self::BackendRescan { protocol_version }
            | Self::BackendEnsure { protocol_version }
            | Self::BackendRestart {
                protocol_version, ..
            }
            | Self::ProfileList { protocol_version }
            | Self::ProfileGet {
                protocol_version, ..
            }
            | Self::ProfileSave {
                protocol_version, ..
            }
            | Self::ProfileApply {
                protocol_version, ..
            }
            | Self::LightingMetadata {
                protocol_version, ..
            }
            | Self::SetLighting {
                protocol_version, ..
            }
            | Self::MouseState {
                protocol_version, ..
            }
            | Self::MouseCapabilities {
                protocol_version, ..
            }
            | Self::KeyboardCapabilities {
                protocol_version, ..
            }
            | Self::SetDpi {
                protocol_version, ..
            }
            | Self::SetPollingRate {
                protocol_version, ..
            }
            | Self::SetMouseProfile {
                protocol_version, ..
            }
            | Self::SetButtonAssignment {
                protocol_version, ..
            }
            | Self::SetLiftOffDistance {
                protocol_version, ..
            }
            | Self::AudioSetVolume {
                protocol_version, ..
            }
            | Self::AudioSetMute {
                protocol_version, ..
            }
            | Self::EqList { protocol_version }
            | Self::EqGet {
                protocol_version, ..
            }
            | Self::EqSave {
                protocol_version, ..
            }
            | Self::EqDelete {
                protocol_version, ..
            }
            | Self::EqApply {
                protocol_version, ..
            }
            | Self::EqBypass { protocol_version }
            | Self::MicDspGet {
                protocol_version, ..
            }
            | Self::MicDspSave {
                protocol_version, ..
            }
            | Self::MicDspLiveUpdate {
                protocol_version, ..
            }
            | Self::MicDspApply {
                protocol_version, ..
            }
            | Self::MicDspBypass {
                protocol_version, ..
            }
            | Self::MicVoiceEnrollStart {
                protocol_version, ..
            }
            | Self::MicVoiceEnrollCancel {
                protocol_version, ..
            }
            | Self::MicVoiceForget {
                protocol_version, ..
            }
            | Self::MicMonitorSet {
                protocol_version, ..
            }
            | Self::MicFirmwareGet {
                protocol_version, ..
            }
            | Self::MicFirmwareStage {
                protocol_version, ..
            }
            | Self::MicFirmwareValidate {
                protocol_version, ..
            }
            | Self::MicFirmwareBegin {
                protocol_version, ..
            }
            | Self::MicFirmwareStatus {
                protocol_version, ..
            }
            | Self::MicFirmwareForget {
                protocol_version, ..
            }
            | Self::OutputDspGet { protocol_version }
            | Self::OutputDspLiveUpdate {
                protocol_version, ..
            }
            | Self::OutputDspResetFactory {
                protocol_version, ..
            } => *protocol_version,
        }
    }

    /// Oldest ForgeHX IPC version that can deserialize and execute this command.
    pub fn minimum_protocol_version(&self) -> u32 {
        match self {
            Self::OutputDspGet { .. }
            | Self::OutputDspLiveUpdate { .. }
            | Self::OutputDspResetFactory { .. } => 12,
            Self::MicMonitorSet { .. } => 11,
            Self::SetLiftOffDistance { .. } => 10,
            Self::MicFirmwareGet { .. }
            | Self::MicFirmwareStage { .. }
            | Self::MicFirmwareValidate { .. }
            | Self::MicFirmwareBegin { .. }
            | Self::MicFirmwareStatus { .. }
            | Self::MicFirmwareForget { .. } => 6,
            Self::MicVoiceEnrollStart { .. }
            | Self::MicVoiceEnrollCancel { .. }
            | Self::MicVoiceForget { .. } => 9,
            Self::MicDspLiveUpdate { .. } => 8,
            Self::KeyboardCapabilities { .. } => 7,
            Self::MouseCapabilities { .. } => 5,
            Self::MicDspGet { .. }
            | Self::MicDspSave { .. }
            | Self::MicDspApply { .. }
            | Self::MicDspBypass { .. } => 4,
            Self::BackendStatus { .. }
            | Self::BackendRescan { .. }
            | Self::BackendEnsure { .. }
            | Self::BackendRestart { .. }
            | Self::ProfileSave { .. }
            | Self::LightingMetadata { .. }
            | Self::MouseState { .. }
            | Self::SetPollingRate { .. }
            | Self::SetMouseProfile { .. }
            | Self::SetButtonAssignment { .. }
            | Self::EqList { .. }
            | Self::EqGet { .. }
            | Self::EqSave { .. }
            | Self::EqDelete { .. }
            | Self::EqApply { .. }
            | Self::EqBypass { .. } => 3,
            _ => IPC_MIN_PROTOCOL_VERSION,
        }
    }

    pub fn with_protocol_version(mut self, version: u32) -> Self {
        match &mut self {
            Self::Ping { protocol_version }
            | Self::DaemonStatus { protocol_version }
            | Self::ListDevices { protocol_version }
            | Self::ListHyperxDevices { protocol_version }
            | Self::ListAllDevices { protocol_version }
            | Self::RefreshDevices { protocol_version }
            | Self::Doctor {
                protocol_version, ..
            }
            | Self::ListAudio { protocol_version }
            | Self::BackendStatus { protocol_version }
            | Self::BackendRescan { protocol_version }
            | Self::BackendEnsure { protocol_version }
            | Self::BackendRestart {
                protocol_version, ..
            }
            | Self::ProfileList { protocol_version }
            | Self::ProfileGet {
                protocol_version, ..
            }
            | Self::ProfileSave {
                protocol_version, ..
            }
            | Self::ProfileApply {
                protocol_version, ..
            }
            | Self::LightingMetadata {
                protocol_version, ..
            }
            | Self::SetLighting {
                protocol_version, ..
            }
            | Self::MouseState {
                protocol_version, ..
            }
            | Self::MouseCapabilities {
                protocol_version, ..
            }
            | Self::KeyboardCapabilities {
                protocol_version, ..
            }
            | Self::SetDpi {
                protocol_version, ..
            }
            | Self::SetPollingRate {
                protocol_version, ..
            }
            | Self::SetMouseProfile {
                protocol_version, ..
            }
            | Self::SetButtonAssignment {
                protocol_version, ..
            }
            | Self::SetLiftOffDistance {
                protocol_version, ..
            }
            | Self::AudioSetVolume {
                protocol_version, ..
            }
            | Self::AudioSetMute {
                protocol_version, ..
            }
            | Self::EqList { protocol_version }
            | Self::EqGet {
                protocol_version, ..
            }
            | Self::EqSave {
                protocol_version, ..
            }
            | Self::EqDelete {
                protocol_version, ..
            }
            | Self::EqApply {
                protocol_version, ..
            }
            | Self::EqBypass { protocol_version }
            | Self::MicDspGet {
                protocol_version, ..
            }
            | Self::MicDspSave {
                protocol_version, ..
            }
            | Self::MicDspLiveUpdate {
                protocol_version, ..
            }
            | Self::MicDspApply {
                protocol_version, ..
            }
            | Self::MicDspBypass {
                protocol_version, ..
            }
            | Self::MicVoiceEnrollStart {
                protocol_version, ..
            }
            | Self::MicVoiceEnrollCancel {
                protocol_version, ..
            }
            | Self::MicVoiceForget {
                protocol_version, ..
            }
            | Self::MicMonitorSet {
                protocol_version, ..
            }
            | Self::MicFirmwareGet {
                protocol_version, ..
            }
            | Self::MicFirmwareStage {
                protocol_version, ..
            }
            | Self::MicFirmwareValidate {
                protocol_version, ..
            }
            | Self::MicFirmwareBegin {
                protocol_version, ..
            }
            | Self::MicFirmwareStatus {
                protocol_version, ..
            }
            | Self::MicFirmwareForget {
                protocol_version, ..
            }
            | Self::OutputDspGet { protocol_version }
            | Self::OutputDspLiveUpdate {
                protocol_version, ..
            }
            | Self::OutputDspResetFactory {
                protocol_version, ..
            } => *protocol_version = version,
        }
        self
    }
}

fn default_ipc_min_protocol_version() -> u32 {
    IPC_MIN_PROTOCOL_VERSION
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "reply", rename_all = "snake_case")]
pub enum Reply {
    Pong {
        protocol_version: u32,
        #[serde(default = "default_ipc_min_protocol_version")]
        min_protocol_version: u32,
    },
    Status {
        protocol_version: u32,
        device_count: usize,
        all_device_count: usize,
        write_protected_count: usize,
    },
    Devices {
        devices: Vec<DeviceInfo>,
    },
    Doctor {
        reports: Vec<DiagnosticReport>,
    },
    AudioNodes {
        nodes: Vec<AudioNode>,
    },
    BackendStatuses {
        backends: Vec<BackendStatus>,
    },
    LightingController {
        metadata: Option<LightingControllerMetadata>,
    },
    MouseState {
        state: MouseDeviceState,
    },
    MouseCapabilities {
        model: Option<MouseModelInfo>,
    },
    KeyboardCapabilities {
        model: Option<KeyboardModelInfo>,
    },
    EqProfiles {
        names: Vec<String>,
    },
    EqProfile {
        config: EqConfig,
    },
    MicDspState {
        state: Box<MicrophoneDspState>,
    },
    OutputDspState {
        profile: OutputDspProfile,
        live: bool,
        generation: u64,
        target_device_id: String,
    },
    FirmwareIdentity {
        identity: FirmwareIdentity,
    },
    FirmwarePackage {
        package: FirmwarePackageInfo,
    },
    FirmwareTransaction {
        status: FirmwareTransactionStatus,
    },
    Profiles {
        names: Vec<String>,
    },
    Profile {
        profile: Profile,
    },
    Applied {
        skipped: Vec<String>,
    },
    Ok {
        message: String,
    },
    Error {
        code: String,
        message: String,
    },
}

impl Reply {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ForgeHxError {
    #[error("unsupported profile schema {0}")]
    UnsupportedSchema(u32),
    #[error("invalid profile: {0}")]
    InvalidProfile(String),
    #[error("unsupported capability: {0:?}")]
    Unsupported(Capability),
    #[error("device not found: {0}")]
    DeviceNotFound(DeviceId),
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("backend protocol error: {0}")]
    BackendProtocol(String),
    #[error("ambiguous backend device mapping: {0}")]
    AmbiguousDevice(String),
    #[error("hardware access denied: {0}")]
    Permission(String),
    #[error("audio integration unavailable: {0}")]
    AudioUnavailable(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("protocol version mismatch: expected {expected}, got {got}")]
    ProtocolVersion { expected: u32, got: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_owner_serializes_and_round_trips() {
        let owner = CapabilityOwner {
            capability: Capability::Lighting,
            backend: BackendKind::OpenRgb,
            backend_device_id: Some("3".into()),
            verified: true,
            writable: true,
            detail: Some("unique match".into()),
        };
        let json = serde_json::to_string(&owner).unwrap();
        assert_eq!(
            serde_json::from_str::<CapabilityOwner>(&json).unwrap(),
            owner
        );
    }

    #[test]
    fn v1_profile_migrates_to_v2_mouse_and_eq() {
        let old_json = r#"{
            "schema_version":1,"name":"Legacy","match_vendor_id":2385,
            "match_product_id":null,"match_serial":null,"lighting":null,
            "dpi":{"stages":[800,1600],"active_stage":1},
            "bindings":[{"key":"1","action":"left"}],
            "audio":{"volume":1.0,"muted":false,"eq":[{"hz":1000,"gain_db":2.5}]},
            "microphone":null
        }"#;
        let migrated = serde_json::from_str::<Profile>(old_json)
            .unwrap()
            .migrate()
            .unwrap();
        assert_eq!(migrated.schema_version, 2);
        assert_eq!(migrated.mouse.as_ref().unwrap().dpi.stages, vec![800, 1600]);
        assert_eq!(migrated.eq.as_ref().unwrap().filters.len(), 1);
        migrated.validate().unwrap();
    }

    #[test]
    fn eq_validation_rejects_unsafe_band() {
        let mut eq = EqConfig::default();
        eq.filters.push(ParametricEqBand {
            enabled: true,
            kind: EqFilterKind::Peaking,
            frequency_hz: 1000.0,
            gain_db: 40.0,
            q: 1.0,
        });
        assert!(eq.validate().is_err());
    }

    #[test]
    fn ipc_v3_commands_round_trip() {
        let values = [
            Command::BackendStatus {
                protocol_version: IPC_PROTOCOL_VERSION,
            },
            Command::BackendRescan {
                protocol_version: IPC_PROTOCOL_VERSION,
            },
            Command::SetPollingRate {
                protocol_version: IPC_PROTOCOL_VERSION,
                device_id: DeviceId("mouse".into()),
                hz: 1000,
            },
            Command::EqBypass {
                protocol_version: IPC_PROTOCOL_VERSION,
            },
        ];
        for command in values {
            let json = serde_json::to_string(&command).unwrap();
            let decoded: Command = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.protocol_version(), IPC_PROTOCOL_VERSION);
        }
    }
}

#[cfg(test)]
mod ipc_protocol_tests {
    use super::*;

    #[test]
    fn legacy_inventory_commands_remain_v2_compatible() {
        let command = Command::ListAllDevices {
            protocol_version: IPC_PROTOCOL_VERSION,
        };
        assert_eq!(command.minimum_protocol_version(), 2);
        assert_eq!(
            command.clone().with_protocol_version(2).protocol_version(),
            2
        );
    }

    #[test]
    fn backend_control_commands_require_v3() {
        let command = Command::BackendEnsure {
            protocol_version: IPC_PROTOCOL_VERSION,
        };
        assert_eq!(command.minimum_protocol_version(), 3);
    }

    #[test]
    fn profile_schema_v2_save_requires_v3() {
        let command = Command::ProfileSave {
            protocol_version: IPC_PROTOCOL_VERSION,
            profile: Profile::default(),
        };
        assert_eq!(command.minimum_protocol_version(), 3);
    }

    #[test]
    fn microphone_dsp_defaults_validate() {
        MicrophoneDspConfig::default().validate().unwrap();
    }

    #[test]
    fn legacy_microphone_profile_gets_safe_noise_monitor_defaults() {
        let legacy = r#"{
          "name":"Broadcast Full","enabled":true,"input_gain_db":0.0,
          "high_pass_hz":80.0,"output_gain_db":0.0
        }"#;
        let cfg: MicrophoneDspConfig = serde_json::from_str(legacy).unwrap();
        assert!(cfg.extraneous_noise_monitor.enabled);
        assert!(cfg.extraneous_noise_monitor.auto_adapt);
        assert_eq!(cfg.extraneous_noise_monitor.sensitivity_percent, 85.0);
        assert_eq!(
            cfg.extraneous_noise_monitor.max_adaptive_suppression_db,
            72.0
        );
        assert!(cfg.extraneous_noise_monitor.speaker_rejection_enabled);
    }

    #[test]
    fn noise_monitor_limits_are_validated() {
        let mut cfg = MicrophoneDspConfig::default();
        cfg.extraneous_noise_monitor.max_adaptive_suppression_db = 72.1;
        assert!(cfg.validate().is_err());
        cfg.extraneous_noise_monitor.max_adaptive_suppression_db = 72.0;
        cfg.extraneous_noise_monitor.sensitivity_percent = 100.0;
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn microphone_dsp_rejects_unsafe_values() {
        let mut config = MicrophoneDspConfig::default();
        config.limiter.ceiling_db = 3.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn microphone_dsp_commands_require_v4() {
        let command = Command::MicDspBypass {
            protocol_version: IPC_PROTOCOL_VERSION,
            device_id: DeviceId("mic".into()),
        };
        assert_eq!(command.minimum_protocol_version(), 4);
    }

    #[test]
    fn microphone_monitor_defaults_off_at_half_level() {
        let monitor = MicrophoneMonitorState::default();
        assert!(!monitor.enabled);
        assert!(!monitor.active);
        assert_eq!(monitor.level_percent, 50.0);
        assert!(monitor.sink_node_name.is_none());
    }

    #[test]
    fn microphone_monitor_command_requires_v11() {
        let command = Command::MicMonitorSet {
            protocol_version: IPC_PROTOCOL_VERSION,
            device_id: DeviceId("mic".into()),
            enabled: true,
            level_percent: 50.0,
        };
        assert_eq!(command.minimum_protocol_version(), 11);
        assert_eq!(
            command.clone().with_protocol_version(10).protocol_version(),
            10
        );
    }

    #[test]
    fn mouse_capabilities_command_requires_v5() {
        let command = Command::MouseCapabilities {
            protocol_version: IPC_PROTOCOL_VERSION,
            device_id: DeviceId("mouse".into()),
        };
        assert_eq!(command.minimum_protocol_version(), 5);
    }

    #[test]
    fn mouse_model_info_round_trips() {
        let model = MouseModelInfo {
            id: "pulsefire-haste-wireless".into(),
            name: "HyperX Pulsefire Haste Wireless".into(),
            protocol_family: MouseProtocolFamily::HasteV1,
            limits: MouseLimits {
                dpi_min: Some(200),
                dpi_max: Some(16000),
                dpi_step: Some(100),
                max_dpi_stages: Some(5),
                polling_rates_hz: vec![125, 250, 500, 1000],
                button_count: Some(6),
                onboard_profiles: Some(1),
                wireless: true,
                battery: true,
                lift_off_distances_mm: vec![1, 2],
            },
            exact_hardware_match: true,
            native_driver: Some("hyperx-pulsefire-haste-wireless-v1".into()),
            complete: true,
        };
        let json = serde_json::to_string(&model).unwrap();
        assert_eq!(
            serde_json::from_str::<MouseModelInfo>(&json).unwrap(),
            model
        );
    }

    #[test]
    fn lift_off_distance_command_requires_v10() {
        let command = Command::SetLiftOffDistance {
            protocol_version: IPC_PROTOCOL_VERSION,
            device_id: DeviceId("mouse".into()),
            mm: 1,
        };
        assert_eq!(command.minimum_protocol_version(), 10);
    }

    #[test]
    fn firmware_identity_round_trips() {
        let identity = FirmwareIdentity {
            device_id: DeviceId("mic-1".into()),
            model: "HyperX SoloCast".into(),
            hardware_revision: Some("rev-a".into()),
            firmware_version: Some("4.1.0".into()),
            firmware_version_source: Some("usb_descriptor".into()),
            bootloader_version: None,
            support_level: FirmwareSupportLevel::InventoryOnly,
        };
        let json = serde_json::to_string(&identity).unwrap();
        assert_eq!(
            serde_json::from_str::<FirmwareIdentity>(&json).unwrap(),
            identity
        );
    }

    #[test]
    fn inventory_only_firmware_support_never_implies_update_or_recovery() {
        let support = FirmwareSupportLevel::InventoryOnly;
        assert!(!support.can_update());
        assert!(!support.can_recover());
    }

    #[test]
    fn microphone_firmware_commands_require_v6() {
        let command = Command::MicFirmwareGet {
            protocol_version: IPC_PROTOCOL_VERSION,
            device_id: DeviceId("mic".into()),
        };
        assert_eq!(command.minimum_protocol_version(), 6);
        let json = serde_json::to_string(&command).unwrap();
        assert_eq!(
            serde_json::from_str::<Command>(&json)
                .unwrap()
                .protocol_version(),
            IPC_PROTOCOL_VERSION
        );
    }
}
