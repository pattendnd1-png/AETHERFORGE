use beacn_lib::EQ_HEADPHONES_VERSION;
use beacn_lib::MaybeFuture;
use beacn_lib::audio::BeacnAudioDevice;
use beacn_lib::audio::messages::Message;
use beacn_lib::audio::messages::compressor::{
    Compressor, CompressorMode, CompressorRatio, CompressorThreshold,
};
use beacn_lib::audio::messages::controls::{Balance, Controls};
use beacn_lib::audio::messages::eq_common::{EQBand, EQBandType, EQFrequency, EQGain, EQQ};
use beacn_lib::audio::messages::eq_headphones::{EQChannel, EQHeadphones};
use beacn_lib::audio::messages::eq_microphone::{EQMicrophone, EQMode};
use beacn_lib::audio::messages::expander::{
    Expander, ExpanderMode, ExpanderRatio, ExpanderThreshold,
};
use beacn_lib::audio::messages::headphones::{
    HPLevel, HPMicMonitorLevel, HPMicOutputGain, HeadphoneTypes, Headphones,
};
use beacn_lib::audio::messages::mic_setup::{MicGain, MicSetup};
use beacn_lib::audio::messages::suppressor::{
    Suppressor, SuppressorAdaptTime, SuppressorSensitivity, SuppressorStyle,
};
use beacn_lib::types::{MakeUpGain, Percent, TimeFrame};

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

const fn eq_defaults() -> [EqBandState; 9] {
    [
        EqBandState::new(80.0),
        EqBandState::new(160.0),
        EqBandState::new(320.0),
        EqBandState::new(640.0),
        EqBandState::new(1_250.0),
        EqBandState::new(2_500.0),
        EqBandState::new(5_000.0),
        EqBandState::new(10_000.0),
        EqBandState::new(16_000.0),
    ]
}

const EQ_BANDS: [EQBand; 9] = [
    EQBand::Band1,
    EQBand::Band2,
    EQBand::Band3,
    EQBand::Band4,
    EQBand::Band5,
    EQBand::Band6,
    EQBand::Band7,
    EQBand::Band8,
    EQBand::Band9,
];

const EQ_CHANNELS: [EQChannel; 2] = [EQChannel::Left, EQChannel::Right];

#[derive(Debug, Clone, PartialEq)]
pub struct HardwareState {
    pub mic_gain: u32,
    pub mic_eq_mode: ProcessorMode,
    pub mic_eq: [EqBandState; 9],
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
    pub headphone_eq_left: [EqBandState; 9],
    pub headphone_eq_right: [EqBandState; 9],
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
            suppressor_adapt_ms: 1000.0,
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

pub struct HardwareController {
    device: Option<Box<dyn BeacnAudioDevice>>,
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
            device: None,
            state: HardwareState::default(),
        }
    }

    pub fn connect() -> Result<Self, String> {
        Err(
            "Protected pass-through mode: direct USB DSP control is blocked to preserve ALSA/PipeWire microphone availability."
                .to_owned(),
        )
    }

    pub fn is_connected(&self) -> bool {
        self.device.is_some()
    }

    pub fn firmware_version(&self) -> Option<String> {
        self.device
            .as_ref()
            .map(|device| device.get_version().to_string())
    }

    pub fn headphone_eq_supported(&self) -> bool {
        self.device
            .as_ref()
            .is_some_and(|device| device.get_version() >= EQ_HEADPHONES_VERSION)
    }

    pub fn refresh_core(&mut self) -> Result<(), String> {
        self.fetch(Message::MicSetup(MicSetup::GetMicGain))?;
        self.refresh_mic_eq()?;

        self.fetch(Message::Compressor(Compressor::GetMode))?;
        self.refresh_compressor()?;

        self.fetch(Message::Expander(Expander::GetMode))?;
        self.refresh_expander()?;

        for message in [
            Suppressor::GetEnabled,
            Suppressor::GetAmount,
            Suppressor::GetStyle,
            Suppressor::GetSensitivity,
            Suppressor::GetAdaptTime,
        ] {
            self.fetch(Message::Suppressor(message))?;
        }

        self.refresh_headphones()?;
        Ok(())
    }

    pub fn set_mic_gain(&mut self, value: u32) -> Result<(), String> {
        let value = clamp_mic_gain(value);
        self.set(Message::MicSetup(MicSetup::MicGain(MicGain(value))))
    }

    pub fn set_mic_eq_mode(&mut self, mode: ProcessorMode) -> Result<(), String> {
        self.set(Message::EQMicrophone(EQMicrophone::Mode(eq_mode(mode))))?;
        self.refresh_mic_eq()
    }

    pub fn set_mic_eq_kind(&mut self, index: usize, kind: EqBandKind) -> Result<(), String> {
        let band = eq_band(index)?;
        let mode = eq_mode(self.state.mic_eq_mode);
        self.set(Message::EQMicrophone(EQMicrophone::Type(
            mode,
            band,
            beacn_eq_kind(kind),
        )))
    }

    pub fn set_mic_eq_gain(&mut self, index: usize, value: f32) -> Result<(), String> {
        let band = eq_band(index)?;
        let mode = eq_mode(self.state.mic_eq_mode);
        self.set(Message::EQMicrophone(EQMicrophone::Gain(
            mode,
            band,
            EQGain(clamp_eq_gain(value)),
        )))
    }

    pub fn set_mic_eq_frequency(&mut self, index: usize, value: f32) -> Result<(), String> {
        let band = eq_band(index)?;
        let mode = eq_mode(self.state.mic_eq_mode);
        self.set(Message::EQMicrophone(EQMicrophone::Frequency(
            mode,
            band,
            EQFrequency(clamp_eq_frequency(value)),
        )))
    }

    pub fn set_mic_eq_q(&mut self, index: usize, value: f32) -> Result<(), String> {
        let band = eq_band(index)?;
        let mode = eq_mode(self.state.mic_eq_mode);
        self.set(Message::EQMicrophone(EQMicrophone::Q(
            mode,
            band,
            EQQ(clamp_eq_q(value)),
        )))
    }

    pub fn set_mic_eq_enabled(&mut self, index: usize, enabled: bool) -> Result<(), String> {
        let band = eq_band(index)?;
        let mode = eq_mode(self.state.mic_eq_mode);
        if enabled && self.state.mic_eq[index].kind == EqBandKind::NotSet {
            self.set(Message::EQMicrophone(EQMicrophone::Type(
                mode,
                band,
                EQBandType::BellBand,
            )))?;
        }
        self.set(Message::EQMicrophone(EQMicrophone::Enabled(
            mode, band, enabled,
        )))
    }

    pub fn set_compressor_mode(&mut self, mode: ProcessorMode) -> Result<(), String> {
        self.set(Message::Compressor(Compressor::Mode(compressor_mode(mode))))?;
        self.refresh_compressor()
    }

    pub fn set_compressor_enabled(&mut self, enabled: bool) -> Result<(), String> {
        let mode = compressor_mode(self.state.compressor_mode);
        self.set(Message::Compressor(Compressor::Enabled(mode, enabled)))
    }

    pub fn set_compressor_threshold(&mut self, value: f32) -> Result<(), String> {
        let mode = compressor_mode(self.state.compressor_mode);
        let value = CompressorThreshold(clamp_compressor_threshold(value));
        self.set(Message::Compressor(Compressor::Threshold(mode, value)))
    }

    pub fn set_compressor_ratio(&mut self, value: f32) -> Result<(), String> {
        let mode = compressor_mode(self.state.compressor_mode);
        let value = CompressorRatio(value.clamp(1.0, 16.0));
        self.set(Message::Compressor(Compressor::Ratio(mode, value)))
    }

    pub fn set_compressor_attack(&mut self, value: f32) -> Result<(), String> {
        let mode = compressor_mode(self.state.compressor_mode);
        self.set(Message::Compressor(Compressor::Attack(
            mode,
            TimeFrame(value.clamp(1.0, 2000.0)),
        )))
    }

    pub fn set_compressor_release(&mut self, value: f32) -> Result<(), String> {
        let mode = compressor_mode(self.state.compressor_mode);
        self.set(Message::Compressor(Compressor::Release(
            mode,
            TimeFrame(value.clamp(1.0, 2000.0)),
        )))
    }

    pub fn set_compressor_makeup(&mut self, value: f32) -> Result<(), String> {
        let mode = compressor_mode(self.state.compressor_mode);
        self.set(Message::Compressor(Compressor::MakeupGain(
            mode,
            MakeUpGain(value.clamp(0.0, 12.0)),
        )))
    }

    pub fn set_expander_mode(&mut self, mode: ProcessorMode) -> Result<(), String> {
        self.set(Message::Expander(Expander::Mode(expander_mode(mode))))?;
        self.refresh_expander()
    }

    pub fn set_expander_enabled(&mut self, enabled: bool) -> Result<(), String> {
        let mode = expander_mode(self.state.expander_mode);
        self.set(Message::Expander(Expander::Enabled(mode, enabled)))
    }

    pub fn set_expander_threshold(&mut self, value: f32) -> Result<(), String> {
        let mode = expander_mode(self.state.expander_mode);
        let value = ExpanderThreshold(clamp_expander_threshold(value));
        self.set(Message::Expander(Expander::Threshold(mode, value)))
    }

    pub fn set_expander_ratio(&mut self, value: f32) -> Result<(), String> {
        let mode = expander_mode(self.state.expander_mode);
        let value = ExpanderRatio(value.clamp(1.0, 10.0));
        self.set(Message::Expander(Expander::Ratio(mode, value)))
    }

    pub fn set_expander_attack(&mut self, value: f32) -> Result<(), String> {
        let mode = expander_mode(self.state.expander_mode);
        self.set(Message::Expander(Expander::Attack(
            mode,
            TimeFrame(value.clamp(1.0, 2000.0)),
        )))
    }

    pub fn set_expander_release(&mut self, value: f32) -> Result<(), String> {
        let mode = expander_mode(self.state.expander_mode);
        self.set(Message::Expander(Expander::Release(
            mode,
            TimeFrame(value.clamp(1.0, 2000.0)),
        )))
    }

    pub fn set_suppressor_enabled(&mut self, enabled: bool) -> Result<(), String> {
        self.set(Message::Suppressor(Suppressor::Enabled(enabled)))
    }

    pub fn set_suppressor_amount(&mut self, value: f32) -> Result<(), String> {
        self.set(Message::Suppressor(Suppressor::Amount(Percent(
            clamp_suppressor_amount(value),
        ))))
    }

    pub fn set_suppressor_style(&mut self, style: NoiseStyle) -> Result<(), String> {
        self.set(Message::Suppressor(Suppressor::Style(noise_style(style))))
    }

    pub fn set_suppressor_sensitivity(&mut self, value: f32) -> Result<(), String> {
        self.set(Message::Suppressor(Suppressor::Sensitivity(
            SuppressorSensitivity(value.clamp(-120.0, -60.0)),
        )))
    }

    pub fn set_suppressor_adapt_ms(&mut self, value: f32) -> Result<(), String> {
        self.set(Message::Suppressor(Suppressor::AdaptTime(
            SuppressorAdaptTime(value.clamp(100.0, 5000.0)),
        )))
    }

    pub fn set_headphone_level(&mut self, value: f32) -> Result<(), String> {
        self.set(Message::Headphones(Headphones::HeadphoneLevel(HPLevel(
            clamp_headphone_level(value),
        ))))
    }

    pub fn set_mic_monitor(&mut self, value: f32) -> Result<(), String> {
        self.set(Message::Headphones(Headphones::MicMonitor(
            HPMicMonitorLevel(clamp_mic_monitor(value)),
        )))
    }

    pub fn set_mic_output_gain(&mut self, value: f32) -> Result<(), String> {
        self.set(Message::Headphones(Headphones::MicOutputGain(
            HPMicOutputGain(value.clamp(0.0, 12.0)),
        )))
    }

    pub fn set_headphone_power(&mut self, value: HeadphonePower) -> Result<(), String> {
        self.set(Message::Headphones(Headphones::HeadphoneType(
            beacn_headphone_power(value),
        )))
    }

    pub fn set_headphone_fx_enabled(&mut self, enabled: bool) -> Result<(), String> {
        self.set(Message::Headphones(Headphones::FXEnabled(enabled)))
    }

    pub fn set_headphone_mono(&mut self, enabled: bool) -> Result<(), String> {
        self.ensure_headphone_eq_supported()?;
        self.set(Message::Controls(Controls::Mono(enabled)))
    }

    pub fn set_headphone_balance(&mut self, value: i32) -> Result<(), String> {
        self.ensure_headphone_eq_supported()?;
        self.set(Message::Controls(Controls::Balance(Balance(
            clamp_headphone_balance(value),
        ))))
    }

    pub fn set_headphone_eq_linked(
        &mut self,
        linked: bool,
        source_channel: HeadphoneEqChannel,
    ) -> Result<(), String> {
        self.ensure_headphone_eq_supported()?;
        let source_bands = match source_channel {
            HeadphoneEqChannel::Left => self.state.headphone_eq_left,
            HeadphoneEqChannel::Right => self.state.headphone_eq_right,
        };
        self.set(Message::EQHeadphones(EQHeadphones::Linked(linked)))?;
        if linked {
            for (index, band) in source_bands.into_iter().enumerate() {
                self.set_headphone_eq_kind(source_channel, index, band.kind)?;
                self.set_headphone_eq_frequency(source_channel, index, band.frequency_hz)?;
                self.set_headphone_eq_gain(source_channel, index, band.gain_db)?;
                self.set_headphone_eq_q(source_channel, index, band.q)?;
                self.set_headphone_eq_enabled(source_channel, index, band.enabled)?;
            }
        }
        Ok(())
    }

    pub fn set_headphone_eq_kind(
        &mut self,
        channel: HeadphoneEqChannel,
        index: usize,
        kind: EqBandKind,
    ) -> Result<(), String> {
        self.ensure_headphone_eq_supported()?;
        let channel = eq_channel(channel);
        let band = eq_band(index)?;
        let kind = beacn_eq_kind(kind);
        self.set(Message::EQHeadphones(EQHeadphones::Type(
            channel, band, kind,
        )))?;
        if self.state.headphone_eq_linked {
            self.set(Message::EQHeadphones(EQHeadphones::Type(
                channel.other(),
                band,
                kind,
            )))?;
        }
        Ok(())
    }

    pub fn set_headphone_eq_gain(
        &mut self,
        channel: HeadphoneEqChannel,
        index: usize,
        value: f32,
    ) -> Result<(), String> {
        self.ensure_headphone_eq_supported()?;
        let channel = eq_channel(channel);
        let band = eq_band(index)?;
        let value = EQGain(clamp_eq_gain(value));
        self.set(Message::EQHeadphones(EQHeadphones::Gain(
            channel, band, value,
        )))?;
        if self.state.headphone_eq_linked {
            self.set(Message::EQHeadphones(EQHeadphones::Gain(
                channel.other(),
                band,
                value,
            )))?;
        }
        Ok(())
    }

    pub fn set_headphone_eq_frequency(
        &mut self,
        channel: HeadphoneEqChannel,
        index: usize,
        value: f32,
    ) -> Result<(), String> {
        self.ensure_headphone_eq_supported()?;
        let channel = eq_channel(channel);
        let band = eq_band(index)?;
        let value = EQFrequency(clamp_eq_frequency(value));
        self.set(Message::EQHeadphones(EQHeadphones::Frequency(
            channel, band, value,
        )))?;
        if self.state.headphone_eq_linked {
            self.set(Message::EQHeadphones(EQHeadphones::Frequency(
                channel.other(),
                band,
                value,
            )))?;
        }
        Ok(())
    }

    pub fn set_headphone_eq_q(
        &mut self,
        channel: HeadphoneEqChannel,
        index: usize,
        value: f32,
    ) -> Result<(), String> {
        self.ensure_headphone_eq_supported()?;
        let channel = eq_channel(channel);
        let band = eq_band(index)?;
        let value = EQQ(clamp_eq_q(value));
        self.set(Message::EQHeadphones(EQHeadphones::Q(channel, band, value)))?;
        if self.state.headphone_eq_linked {
            self.set(Message::EQHeadphones(EQHeadphones::Q(
                channel.other(),
                band,
                value,
            )))?;
        }
        Ok(())
    }

    pub fn set_headphone_eq_enabled(
        &mut self,
        channel: HeadphoneEqChannel,
        index: usize,
        enabled: bool,
    ) -> Result<(), String> {
        self.ensure_headphone_eq_supported()?;
        let band = eq_band(index)?;
        let state = match channel {
            HeadphoneEqChannel::Left => self.state.headphone_eq_left[index],
            HeadphoneEqChannel::Right => self.state.headphone_eq_right[index],
        };
        let beacn_channel = eq_channel(channel);
        if enabled && state.kind == EqBandKind::NotSet {
            self.set_headphone_eq_kind(channel, index, EqBandKind::Bell)?;
        }
        self.set(Message::EQHeadphones(EQHeadphones::Enabled(
            beacn_channel,
            band,
            enabled,
        )))?;
        if self.state.headphone_eq_linked {
            self.set(Message::EQHeadphones(EQHeadphones::Enabled(
                beacn_channel.other(),
                band,
                enabled,
            )))?;
        }
        Ok(())
    }

    fn refresh_mic_eq(&mut self) -> Result<(), String> {
        self.fetch(Message::EQMicrophone(EQMicrophone::GetMode))?;
        let mode = eq_mode(self.state.mic_eq_mode);
        for band in EQ_BANDS {
            for message in [
                EQMicrophone::GetType(mode, band),
                EQMicrophone::GetGain(mode, band),
                EQMicrophone::GetFrequency(mode, band),
                EQMicrophone::GetQ(mode, band),
                EQMicrophone::GetEnabled(mode, band),
            ] {
                self.fetch(Message::EQMicrophone(message))?;
            }
        }
        Ok(())
    }

    fn refresh_headphones(&mut self) -> Result<(), String> {
        for message in [
            Headphones::GetHeadphoneLevel,
            Headphones::GetMicMonitor,
            Headphones::GetMicOutputGain,
            Headphones::GetHeadphoneType,
            Headphones::GetFXEnabled,
        ] {
            self.fetch(Message::Headphones(message))?;
        }

        if !self.headphone_eq_supported() {
            return Ok(());
        }

        for message in [Controls::GetMono, Controls::GetBalance] {
            self.fetch(Message::Controls(message))?;
        }
        self.fetch(Message::EQHeadphones(EQHeadphones::GetLinked))?;
        let channel_count = if self.state.headphone_eq_linked { 1 } else { 2 };
        for channel in EQ_CHANNELS.into_iter().take(channel_count) {
            for band in EQ_BANDS {
                for message in [
                    EQHeadphones::GetType(channel, band),
                    EQHeadphones::GetGain(channel, band),
                    EQHeadphones::GetFrequency(channel, band),
                    EQHeadphones::GetQ(channel, band),
                    EQHeadphones::GetEnabled(channel, band),
                ] {
                    self.fetch(Message::EQHeadphones(message))?;
                }
            }
        }
        if self.state.headphone_eq_linked {
            self.state.headphone_eq_right = self.state.headphone_eq_left;
        }
        Ok(())
    }

    fn refresh_compressor(&mut self) -> Result<(), String> {
        let mode = compressor_mode(self.state.compressor_mode);
        for message in [
            Compressor::GetEnabled(mode),
            Compressor::GetThreshold(mode),
            Compressor::GetRatio(mode),
            Compressor::GetAttack(mode),
            Compressor::GetRelease(mode),
            Compressor::GetMakeupGain(mode),
        ] {
            self.fetch(Message::Compressor(message))?;
        }
        Ok(())
    }

    fn refresh_expander(&mut self) -> Result<(), String> {
        let mode = expander_mode(self.state.expander_mode);
        for message in [
            Expander::GetEnabled(mode),
            Expander::GetThreshold(mode),
            Expander::GetRatio(mode),
            Expander::GetAttack(mode),
            Expander::GetRelease(mode),
        ] {
            self.fetch(Message::Expander(message))?;
        }
        Ok(())
    }

    fn ensure_headphone_eq_supported(&self) -> Result<(), String> {
        if self.headphone_eq_supported() {
            Ok(())
        } else {
            Err("Headphone EQ/balance requires BEACN firmware 1.3.0 or newer".to_owned())
        }
    }

    fn fetch(&mut self, message: Message) -> Result<(), String> {
        let response = self.send(message)?;
        self.apply_response(response);
        Ok(())
    }

    fn set(&mut self, message: Message) -> Result<(), String> {
        let response = self.send(message)?;
        self.apply_response(response);
        Ok(())
    }

    fn send(&self, message: Message) -> Result<Message, String> {
        let device = self
            .device
            .as_ref()
            .ok_or_else(|| "Hardware DSP is not connected".to_owned())?;
        device
            .handle_message(message)
            .wait()
            .map_err(|error| format!("BEACN hardware command failed: {error}"))
    }

    fn apply_response(&mut self, message: Message) {
        match message {
            Message::MicSetup(MicSetup::MicGain(value)) => self.state.mic_gain = value.0,
            Message::EQMicrophone(EQMicrophone::Mode(mode)) => {
                self.state.mic_eq_mode = processor_mode_from_eq(mode);
            }
            Message::EQMicrophone(EQMicrophone::Type(_, band, value)) => {
                self.state.mic_eq[eq_band_index(band)].kind = eq_kind_from_beacn(value);
            }
            Message::EQMicrophone(EQMicrophone::Gain(_, band, value)) => {
                self.state.mic_eq[eq_band_index(band)].gain_db = value.0;
            }
            Message::EQMicrophone(EQMicrophone::Frequency(_, band, value)) => {
                self.state.mic_eq[eq_band_index(band)].frequency_hz = value.0;
            }
            Message::EQMicrophone(EQMicrophone::Q(_, band, value)) => {
                self.state.mic_eq[eq_band_index(band)].q = value.0;
            }
            Message::EQMicrophone(EQMicrophone::Enabled(_, band, value)) => {
                self.state.mic_eq[eq_band_index(band)].enabled = value;
            }
            Message::Compressor(Compressor::Mode(mode)) => {
                self.state.compressor_mode = processor_mode_from_compressor(mode);
            }
            Message::Compressor(Compressor::Enabled(_, value)) => {
                self.state.compressor_enabled = value;
            }
            Message::Compressor(Compressor::Threshold(_, value)) => {
                self.state.compressor_threshold = value.0;
            }
            Message::Compressor(Compressor::Ratio(_, value)) => {
                self.state.compressor_ratio = value.0;
            }
            Message::Compressor(Compressor::Attack(_, value)) => {
                self.state.compressor_attack_ms = value.0;
            }
            Message::Compressor(Compressor::Release(_, value)) => {
                self.state.compressor_release_ms = value.0;
            }
            Message::Compressor(Compressor::MakeupGain(_, value)) => {
                self.state.compressor_makeup_gain = value.0;
            }
            Message::Expander(Expander::Mode(mode)) => {
                self.state.expander_mode = processor_mode_from_expander(mode);
            }
            Message::Expander(Expander::Enabled(_, value)) => {
                self.state.expander_enabled = value;
            }
            Message::Expander(Expander::Threshold(_, value)) => {
                self.state.expander_threshold = value.0;
            }
            Message::Expander(Expander::Ratio(_, value)) => {
                self.state.expander_ratio = value.0;
            }
            Message::Expander(Expander::Attack(_, value)) => {
                self.state.expander_attack_ms = value.0;
            }
            Message::Expander(Expander::Release(_, value)) => {
                self.state.expander_release_ms = value.0;
            }
            Message::Suppressor(Suppressor::Enabled(value)) => {
                self.state.suppressor_enabled = value;
            }
            Message::Suppressor(Suppressor::Amount(value)) => {
                self.state.suppressor_amount = value.0;
            }
            Message::Suppressor(Suppressor::Style(value)) => {
                self.state.suppressor_style = noise_style_from_beacn(value);
            }
            Message::Suppressor(Suppressor::Sensitivity(value)) => {
                self.state.suppressor_sensitivity = value.0;
            }
            Message::Suppressor(Suppressor::AdaptTime(value)) => {
                self.state.suppressor_adapt_ms = value.0;
            }
            Message::Headphones(Headphones::HeadphoneLevel(value)) => {
                self.state.headphone_level_db = value.0;
            }
            Message::Headphones(Headphones::MicMonitor(value)) => {
                self.state.mic_monitor_db = value.0;
            }
            Message::Headphones(Headphones::MicOutputGain(value)) => {
                self.state.mic_output_gain_db = value.0;
            }
            Message::Headphones(Headphones::HeadphoneType(value)) => {
                self.state.headphone_power = headphone_power_from_beacn(value);
            }
            Message::Headphones(Headphones::FXEnabled(value)) => {
                self.state.headphone_fx_enabled = value;
            }
            Message::Controls(Controls::Mono(value)) => {
                self.state.headphone_mono = value;
            }
            Message::Controls(Controls::Balance(value)) => {
                self.state.headphone_balance = value.0;
            }
            Message::EQHeadphones(EQHeadphones::Linked(value)) => {
                self.state.headphone_eq_linked = value;
            }
            Message::EQHeadphones(EQHeadphones::Type(channel, band, value)) => {
                self.headphone_band_mut(channel, band).kind = eq_kind_from_beacn(value);
            }
            Message::EQHeadphones(EQHeadphones::Gain(channel, band, value)) => {
                self.headphone_band_mut(channel, band).gain_db = value.0;
            }
            Message::EQHeadphones(EQHeadphones::Frequency(channel, band, value)) => {
                self.headphone_band_mut(channel, band).frequency_hz = value.0;
            }
            Message::EQHeadphones(EQHeadphones::Q(channel, band, value)) => {
                self.headphone_band_mut(channel, band).q = value.0;
            }
            Message::EQHeadphones(EQHeadphones::Enabled(channel, band, value)) => {
                self.headphone_band_mut(channel, band).enabled = value;
            }
            _ => {}
        }
    }

    fn headphone_band_mut(&mut self, channel: EQChannel, band: EQBand) -> &mut EqBandState {
        let index = eq_band_index(band);
        match channel {
            EQChannel::Left => &mut self.state.headphone_eq_left[index],
            EQChannel::Right => &mut self.state.headphone_eq_right[index],
        }
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

fn eq_band(index: usize) -> Result<EQBand, String> {
    EQ_BANDS
        .get(index)
        .copied()
        .ok_or_else(|| format!("EQ band index {index} is outside 0..9"))
}

fn eq_band_index(band: EQBand) -> usize {
    match band {
        EQBand::Band1 => 0,
        EQBand::Band2 => 1,
        EQBand::Band3 => 2,
        EQBand::Band4 => 3,
        EQBand::Band5 => 4,
        EQBand::Band6 => 5,
        EQBand::Band7 => 6,
        EQBand::Band8 => 7,
        EQBand::Band9 => 8,
    }
}

fn eq_mode(mode: ProcessorMode) -> EQMode {
    match mode {
        ProcessorMode::Simple => EQMode::Simple,
        ProcessorMode::Advanced => EQMode::Advanced,
    }
}

fn processor_mode_from_eq(mode: EQMode) -> ProcessorMode {
    match mode {
        EQMode::Simple => ProcessorMode::Simple,
        EQMode::Advanced => ProcessorMode::Advanced,
    }
}

fn beacn_eq_kind(kind: EqBandKind) -> EQBandType {
    match kind {
        EqBandKind::NotSet => EQBandType::NotSet,
        EqBandKind::LowPass => EQBandType::LowPassFilter,
        EqBandKind::HighPass => EQBandType::HighPassFilter,
        EqBandKind::Notch => EQBandType::NotchFilter,
        EqBandKind::Bell => EQBandType::BellBand,
        EqBandKind::LowShelf => EQBandType::LowShelf,
        EqBandKind::HighShelf => EQBandType::HighShelf,
    }
}

fn eq_kind_from_beacn(kind: EQBandType) -> EqBandKind {
    match kind {
        EQBandType::NotSet => EqBandKind::NotSet,
        EQBandType::LowPassFilter => EqBandKind::LowPass,
        EQBandType::HighPassFilter => EqBandKind::HighPass,
        EQBandType::NotchFilter => EqBandKind::Notch,
        EQBandType::BellBand => EqBandKind::Bell,
        EQBandType::LowShelf => EqBandKind::LowShelf,
        EQBandType::HighShelf => EqBandKind::HighShelf,
    }
}

fn eq_channel(channel: HeadphoneEqChannel) -> EQChannel {
    match channel {
        HeadphoneEqChannel::Left => EQChannel::Left,
        HeadphoneEqChannel::Right => EQChannel::Right,
    }
}

fn compressor_mode(mode: ProcessorMode) -> CompressorMode {
    match mode {
        ProcessorMode::Simple => CompressorMode::Simple,
        ProcessorMode::Advanced => CompressorMode::Advanced,
    }
}

fn processor_mode_from_compressor(mode: CompressorMode) -> ProcessorMode {
    match mode {
        CompressorMode::Simple => ProcessorMode::Simple,
        CompressorMode::Advanced => ProcessorMode::Advanced,
    }
}

fn expander_mode(mode: ProcessorMode) -> ExpanderMode {
    match mode {
        ProcessorMode::Simple => ExpanderMode::Simple,
        ProcessorMode::Advanced => ExpanderMode::Advanced,
    }
}

fn processor_mode_from_expander(mode: ExpanderMode) -> ProcessorMode {
    match mode {
        ExpanderMode::Simple => ProcessorMode::Simple,
        ExpanderMode::Advanced => ProcessorMode::Advanced,
    }
}

fn noise_style(style: NoiseStyle) -> SuppressorStyle {
    match style {
        NoiseStyle::Instant => SuppressorStyle::Instant,
        NoiseStyle::Adaptive => SuppressorStyle::Adaptive,
        NoiseStyle::Snapshot => SuppressorStyle::Snapshot,
    }
}

fn noise_style_from_beacn(style: SuppressorStyle) -> NoiseStyle {
    match style {
        SuppressorStyle::Instant => NoiseStyle::Instant,
        SuppressorStyle::Adaptive => NoiseStyle::Adaptive,
        SuppressorStyle::Snapshot => NoiseStyle::Snapshot,
    }
}

fn beacn_headphone_power(value: HeadphonePower) -> HeadphoneTypes {
    match value {
        HeadphonePower::LineLevel => HeadphoneTypes::LineLevel,
        HeadphonePower::Normal => HeadphoneTypes::NormalPower,
        HeadphonePower::HighImpedance => HeadphoneTypes::HighImpedance,
        HeadphonePower::InEarMonitors => HeadphoneTypes::InEarMonitors,
    }
}

fn headphone_power_from_beacn(value: HeadphoneTypes) -> HeadphonePower {
    match value {
        HeadphoneTypes::LineLevel => HeadphonePower::LineLevel,
        HeadphoneTypes::NormalPower => HeadphonePower::Normal,
        HeadphoneTypes::HighImpedance => HeadphonePower::HighImpedance,
        HeadphoneTypes::InEarMonitors => HeadphonePower::InEarMonitors,
    }
}
