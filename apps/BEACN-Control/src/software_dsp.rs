//! Safe, profile-owned BEACN-style DSP state.
//!
//! This module deliberately contains no USB or PipeWire ownership code. It is the
//! editable control/profile model used by the Windows-parity UI while ALSA/PipeWire
//! remain authoritative for the live BEACN audio device.

use crate::hardware::{EqBandKind, EqBandState, HeadphonePower, NoiseStyle, ProcessorMode};

pub const MIC_EQ_BAND_COUNT: usize = 9;
pub const HEADPHONE_EQ_BAND_COUNT: usize = 10;

const fn band(frequency_hz: f32) -> EqBandState {
    EqBandState {
        kind: EqBandKind::Bell,
        gain_db: 0.0,
        frequency_hz,
        q: 1.0,
        enabled: true,
    }
}

pub const fn mic_eq_defaults() -> [EqBandState; MIC_EQ_BAND_COUNT] {
    [
        band(80.0),
        band(160.0),
        band(320.0),
        band(640.0),
        band(1_250.0),
        band(2_500.0),
        band(5_000.0),
        band(10_000.0),
        band(16_000.0),
    ]
}

pub const fn headphone_eq_defaults() -> [EqBandState; HEADPHONE_EQ_BAND_COUNT] {
    [
        band(31.0),
        band(63.0),
        band(125.0),
        band(250.0),
        band(500.0),
        band(1_000.0),
        band(2_000.0),
        band(4_000.0),
        band(8_000.0),
        band(16_000.0),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DspBackendState {
    #[default]
    Unavailable,
    Ready,
    Error,
}

impl DspBackendState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unavailable => "DSP BACKEND UNAVAILABLE",
            Self::Ready => "DSP BACKEND READY",
            Self::Error => "DSP BACKEND ERROR",
        }
    }

    pub const fn can_dispatch(self) -> bool {
        matches!(self, Self::Ready)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressorState {
    pub mode: ProcessorMode,
    pub enabled: bool,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_gain_db: f32,
}

impl Default for CompressorState {
    fn default() -> Self {
        Self {
            mode: ProcessorMode::Simple,
            enabled: false,
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 180.0,
            makeup_gain_db: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpanderState {
    pub mode: ProcessorMode,
    pub enabled: bool,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}

impl Default for ExpanderState {
    fn default() -> Self {
        Self {
            mode: ProcessorMode::Simple,
            enabled: false,
            threshold_db: -50.0,
            ratio: 2.0,
            attack_ms: 10.0,
            release_ms: 180.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoiseSuppressionState {
    pub enabled: bool,
    pub amount: f32,
    pub style: NoiseStyle,
    pub sensitivity_db: f32,
    pub adapt_ms: f32,
}

impl Default for NoiseSuppressionState {
    fn default() -> Self {
        Self {
            enabled: false,
            amount: 50.0,
            style: NoiseStyle::Adaptive,
            sensitivity_db: -90.0,
            adapt_ms: 1_000.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeEsserState {
    pub enabled: bool,
    pub amount: f32,
    pub frequency_hz: f32,
}

impl Default for DeEsserState {
    fn default() -> Self {
        Self {
            enabled: false,
            amount: 35.0,
            frequency_hz: 6_500.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExciterState {
    pub enabled: bool,
    pub amount: f32,
    pub tone: f32,
}

impl Default for ExciterState {
    fn default() -> Self {
        Self {
            enabled: false,
            amount: 20.0,
            tone: 50.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadphoneState {
    pub level_db: f32,
    pub mic_monitor_db: f32,
    pub power: HeadphonePower,
    pub fx_enabled: bool,
    pub mono: bool,
    pub balance: i32,
    pub eq_linked: bool,
    pub eq_left: [EqBandState; HEADPHONE_EQ_BAND_COUNT],
    pub eq_right: [EqBandState; HEADPHONE_EQ_BAND_COUNT],
    pub preset_name: String,
    pub binaural_personalization: bool,
}

impl Default for HeadphoneState {
    fn default() -> Self {
        Self {
            level_db: -20.0,
            mic_monitor_db: -20.0,
            power: HeadphonePower::Normal,
            fx_enabled: true,
            mono: false,
            balance: 0,
            eq_linked: true,
            eq_left: headphone_eq_defaults(),
            eq_right: headphone_eq_defaults(),
            preset_name: "Flat".to_owned(),
            binaural_personalization: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoftwareDspState {
    pub mic_gain_db: f32,
    pub mic_eq_mode: ProcessorMode,
    pub mic_eq: [EqBandState; MIC_EQ_BAND_COUNT],
    pub compressor: CompressorState,
    pub expander: ExpanderState,
    pub suppressor: NoiseSuppressionState,
    pub de_esser: DeEsserState,
    pub exciter: ExciterState,
    pub mic_output_gain_db: f32,
    pub headphones: HeadphoneState,
}

impl Default for SoftwareDspState {
    fn default() -> Self {
        Self {
            mic_gain_db: 0.0,
            mic_eq_mode: ProcessorMode::Simple,
            mic_eq: mic_eq_defaults(),
            compressor: CompressorState::default(),
            expander: ExpanderState::default(),
            suppressor: NoiseSuppressionState::default(),
            de_esser: DeEsserState::default(),
            exciter: ExciterState::default(),
            mic_output_gain_db: 0.0,
            headphones: HeadphoneState::default(),
        }
    }
}

impl SoftwareDspState {
    pub fn sanitize(&mut self) {
        self.mic_gain_db = self.mic_gain_db.clamp(-24.0, 24.0);
        self.mic_output_gain_db = self.mic_output_gain_db.clamp(-24.0, 12.0);
        for band in &mut self.mic_eq {
            sanitize_eq_band(band);
        }
        self.compressor.threshold_db = self.compressor.threshold_db.clamp(-40.0, 0.0);
        self.compressor.ratio = self.compressor.ratio.clamp(1.0, 16.0);
        self.compressor.attack_ms = self.compressor.attack_ms.clamp(1.0, 2_000.0);
        self.compressor.release_ms = self.compressor.release_ms.clamp(1.0, 2_000.0);
        self.compressor.makeup_gain_db = self.compressor.makeup_gain_db.clamp(0.0, 12.0);
        self.expander.threshold_db = self.expander.threshold_db.clamp(-90.0, 0.0);
        self.expander.ratio = self.expander.ratio.clamp(1.0, 10.0);
        self.expander.attack_ms = self.expander.attack_ms.clamp(1.0, 2_000.0);
        self.expander.release_ms = self.expander.release_ms.clamp(1.0, 2_000.0);
        self.suppressor.amount = self.suppressor.amount.clamp(0.0, 100.0);
        self.suppressor.sensitivity_db = self.suppressor.sensitivity_db.clamp(-120.0, -60.0);
        self.suppressor.adapt_ms = self.suppressor.adapt_ms.clamp(100.0, 5_000.0);
        self.de_esser.amount = self.de_esser.amount.clamp(0.0, 100.0);
        self.de_esser.frequency_hz = self.de_esser.frequency_hz.clamp(2_000.0, 12_000.0);
        self.exciter.amount = self.exciter.amount.clamp(0.0, 100.0);
        self.exciter.tone = self.exciter.tone.clamp(0.0, 100.0);
        self.headphones.level_db = self.headphones.level_db.clamp(-70.0, 0.0);
        self.headphones.mic_monitor_db = self.headphones.mic_monitor_db.clamp(-100.0, 6.0);
        self.headphones.balance = self.headphones.balance.clamp(-100, 100);
        for band in &mut self.headphones.eq_left {
            sanitize_eq_band(band);
        }
        for band in &mut self.headphones.eq_right {
            sanitize_eq_band(band);
        }
    }

    pub fn set_headphone_band(
        &mut self,
        left: bool,
        index: usize,
        band: EqBandState,
    ) -> Result<(), String> {
        if index >= HEADPHONE_EQ_BAND_COUNT {
            return Err(format!("invalid headphone EQ band {index}"));
        }
        let mut band = band;
        sanitize_eq_band(&mut band);
        if left {
            self.headphones.eq_left[index] = band;
            if self.headphones.eq_linked {
                self.headphones.eq_right[index] = band;
            }
        } else {
            self.headphones.eq_right[index] = band;
            if self.headphones.eq_linked {
                self.headphones.eq_left[index] = band;
            }
        }
        Ok(())
    }

    pub fn set_headphones_linked(&mut self, linked: bool, source_left: bool) {
        self.headphones.eq_linked = linked;
        if linked {
            if source_left {
                self.headphones.eq_right = self.headphones.eq_left;
            } else {
                self.headphones.eq_left = self.headphones.eq_right;
            }
        }
    }
}

pub fn sanitize_eq_band(band: &mut EqBandState) {
    band.gain_db = band.gain_db.clamp(-12.0, 12.0);
    band.frequency_hz = band.frequency_hz.clamp(20.0, 20_000.0);
    band.q = band.q.clamp(0.1, 10.0);
}
