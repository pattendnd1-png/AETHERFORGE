//! BEACN control-surface data types and hard direct-USB safety boundary.
//!
//! v0.1.23 keeps hardware writes behind a hard fail-closed boundary. The
//! separate `on_device` module may query the mic vendor parameter interface
//! with getter messages only; it does not send setters or detach audio. Linux
//! snd_usb_audio, ALSA and PipeWire retain the physical audio device. Audible
//! AetherForge processing lives in the app-private DSP path instead.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessorMode {
    #[default]
    Simple,
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoiseStyle {
    Instant,
    #[default]
    Adaptive,
    Snapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EqBandKind {
    NotSet,
    LowPass,
    HighPass,
    Notch,
    #[default]
    Bell,
    LowShelf,
    HighShelf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HeadphoneEqChannel {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeadphonePower {
    LineLevel,
    #[default]
    Normal,
    HighImpedance,
    InEarMonitors,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EqBandState {
    pub kind: EqBandKind,
    pub gain_db: f32,
    pub frequency_hz: f32,
    pub q: f32,
    pub enabled: bool,
}

impl EqBandState {
    const fn new(frequency_hz: f32) -> Self {
        Self {
            kind: EqBandKind::Bell,
            gain_db: 0.0,
            frequency_hz,
            q: 1.0,
            enabled: true,
        }
    }
}

pub const HARDWARE_EQ_BAND_COUNT: usize = 10;

const fn eq_defaults() -> [EqBandState; HARDWARE_EQ_BAND_COUNT] {
    [
        EqBandState::new(31.0),
        EqBandState::new(63.0),
        EqBandState::new(125.0),
        EqBandState::new(250.0),
        EqBandState::new(500.0),
        EqBandState::new(1_000.0),
        EqBandState::new(2_000.0),
        EqBandState::new(4_000.0),
        EqBandState::new(8_000.0),
        EqBandState::new(16_000.0),
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareState {
    pub mic_gain: u32,
    pub mic_eq_mode: ProcessorMode,
    pub mic_eq: [EqBandState; HARDWARE_EQ_BAND_COUNT],
    pub compressor_mode: ProcessorMode,
    pub compressor_enabled: bool,
    pub compressor_threshold: f32,
    pub compressor_ratio: f32,
    pub compressor_attack_ms: f32,
    pub compressor_release_ms: f32,
    pub compressor_makeup_gain: f32,
    pub expander_mode: ProcessorMode,
    pub expander_enabled: bool,
    pub expander_threshold: f32,
    pub expander_ratio: f32,
    pub expander_attack_ms: f32,
    pub expander_release_ms: f32,
    pub suppressor_enabled: bool,
    pub suppressor_amount: f32,
    pub suppressor_style: NoiseStyle,
    pub suppressor_sensitivity: f32,
    pub suppressor_adapt_ms: f32,
    pub headphone_level_db: f32,
    pub mic_monitor_db: f32,
    pub mic_output_gain_db: f32,
    pub headphone_power: HeadphonePower,
    pub headphone_fx_enabled: bool,
    pub headphone_mono: bool,
    pub headphone_balance: i32,
    pub headphone_eq_linked: bool,
    pub headphone_eq_left: [EqBandState; HARDWARE_EQ_BAND_COUNT],
    pub headphone_eq_right: [EqBandState; HARDWARE_EQ_BAND_COUNT],
}

impl Default for HardwareState {
    fn default() -> Self {
        Self {
            mic_gain: 10,
            mic_eq_mode: ProcessorMode::Simple,
            mic_eq: eq_defaults(),
            compressor_mode: ProcessorMode::Simple,
            compressor_enabled: false,
            compressor_threshold: -18.0,
            compressor_ratio: 4.0,
            compressor_attack_ms: 10.0,
            compressor_release_ms: 180.0,
            compressor_makeup_gain: 0.0,
            expander_mode: ProcessorMode::Simple,
            expander_enabled: false,
            expander_threshold: -50.0,
            expander_ratio: 2.0,
            expander_attack_ms: 10.0,
            expander_release_ms: 180.0,
            suppressor_enabled: false,
            suppressor_amount: 50.0,
            suppressor_style: NoiseStyle::Adaptive,
            suppressor_sensitivity: -90.0,
            suppressor_adapt_ms: 1_000.0,
            headphone_level_db: -20.0,
            mic_monitor_db: -20.0,
            mic_output_gain_db: 0.0,
            headphone_power: HeadphonePower::Normal,
            headphone_fx_enabled: true,
            headphone_mono: false,
            headphone_balance: 0,
            headphone_eq_linked: true,
            headphone_eq_left: eq_defaults(),
            headphone_eq_right: eq_defaults(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectUsbControlPolicy {
    BlockedToPreserveSystemAudio,
}

pub const fn direct_usb_control_policy() -> DirectUsbControlPolicy {
    DirectUsbControlPolicy::BlockedToPreserveSystemAudio
}

pub const fn direct_usb_claims_allowed() -> bool {
    false
}

/// Compatibility shell retained so an accidental call fails closed.
pub struct HardwareController {
    pub state: HardwareState,
}

impl Default for HardwareController {
    fn default() -> Self {
        Self::disconnected()
    }
}

impl HardwareController {
    pub fn disconnected() -> Self {
        Self {
            state: HardwareState::default(),
        }
    }

    pub fn connect() -> Result<Self, String> {
        Err(
            "Protected pass-through mode: hardware DSP writes are blocked; mic-memory reads are handled separately without taking audio ownership."
                .to_owned(),
        )
    }

    pub const fn is_connected(&self) -> bool {
        false
    }

    pub const fn firmware_version(&self) -> Option<String> {
        None
    }

    pub const fn headphone_eq_supported(&self) -> bool {
        false
    }
}

pub fn clamp_mic_gain(value: u32) -> u32 {
    value.clamp(3, 20)
}

pub fn clamp_compressor_threshold(value: f32) -> f32 {
    value.clamp(-40.0, 0.0)
}

pub fn clamp_expander_threshold(value: f32) -> f32 {
    value.clamp(-90.0, 0.0)
}

pub fn clamp_suppressor_amount(value: f32) -> f32 {
    value.clamp(0.0, 100.0)
}

pub fn clamp_eq_gain(value: f32) -> f32 {
    value.clamp(-12.0, 12.0)
}

pub fn clamp_eq_frequency(value: f32) -> f32 {
    value.clamp(20.0, 20_000.0)
}

pub fn clamp_eq_q(value: f32) -> f32 {
    value.clamp(0.1, 10.0)
}

pub fn clamp_headphone_level(value: f32) -> f32 {
    value.clamp(-70.0, 0.0)
}

pub fn clamp_mic_monitor(value: f32) -> f32 {
    value.clamp(-100.0, 6.0)
}

pub fn clamp_headphone_balance(value: i32) -> i32 {
    value.clamp(-100, 100)
}
