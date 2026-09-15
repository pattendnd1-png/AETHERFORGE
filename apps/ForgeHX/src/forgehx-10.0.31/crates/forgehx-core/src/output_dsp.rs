use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum OutputDeviceClass {
    DesktopSpeaker,
    BookshelfSpeaker,
    StudioMonitor,
    DesktopMonitorSpeaker,
    TvSpeaker,
    Soundbar,
    Speaker2_1,
    Speaker5_1,
    Speaker7_1,
    OpenBackHeadphone,
    ClosedBackHeadphone,
    GamingHeadset,
    Iem,
    WiredEarbud,
    BluetoothEarbud,
    BluetoothHeadphone,
    VehicleAudio,
    ExternalAudio,
    Custom,
}

impl OutputDeviceClass {
    pub const ALL: [Self; 19] = [
        Self::DesktopSpeaker,
        Self::BookshelfSpeaker,
        Self::StudioMonitor,
        Self::DesktopMonitorSpeaker,
        Self::TvSpeaker,
        Self::Soundbar,
        Self::Speaker2_1,
        Self::Speaker5_1,
        Self::Speaker7_1,
        Self::OpenBackHeadphone,
        Self::ClosedBackHeadphone,
        Self::GamingHeadset,
        Self::Iem,
        Self::WiredEarbud,
        Self::BluetoothEarbud,
        Self::BluetoothHeadphone,
        Self::VehicleAudio,
        Self::ExternalAudio,
        Self::Custom,
    ];

    #[must_use]
    pub const fn for_display_speakers(is_television: bool) -> Self {
        if is_television {
            Self::TvSpeaker
        } else {
            Self::DesktopMonitorSpeaker
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DesktopSpeaker => "Desktop speakers",
            Self::BookshelfSpeaker => "Bookshelf speakers",
            Self::StudioMonitor => "Studio monitors",
            Self::DesktopMonitorSpeaker => "Desktop monitor speakers",
            Self::TvSpeaker => "TV speakers",
            Self::Soundbar => "Soundbar",
            Self::Speaker2_1 => "2.1 speakers",
            Self::Speaker5_1 => "5.1 speakers",
            Self::Speaker7_1 => "7.1 speakers",
            Self::OpenBackHeadphone => "Open-back headphones",
            Self::ClosedBackHeadphone => "Closed-back headphones",
            Self::GamingHeadset => "Gaming headset",
            Self::Iem => "IEM",
            Self::WiredEarbud => "Wired earbuds",
            Self::BluetoothEarbud => "Bluetooth earbuds",
            Self::BluetoothHeadphone => "Bluetooth headphones",
            Self::VehicleAudio => "Vehicle audio",
            Self::ExternalAudio => "External audio",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OutputFilterKind {
    HighPass,
    LowPass,
    BandPass,
    Peak,
    Notch,
    LowShelf,
    HighShelf,
}

impl OutputFilterKind {
    pub const ALL: [Self; 7] = [
        Self::HighPass,
        Self::BandPass,
        Self::LowPass,
        Self::Peak,
        Self::Notch,
        Self::LowShelf,
        Self::HighShelf,
    ];

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::HighPass => "High-pass",
            Self::LowPass => "Low-pass",
            Self::BandPass => "Mid / band-pass",
            Self::Peak => "Parametric peak",
            Self::Notch => "Notch",
            Self::LowShelf => "Low shelf",
            Self::HighShelf => "High shelf",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputFilter {
    pub enabled: bool,
    pub kind: OutputFilterKind,
    pub frequency_hz: f32,
    pub gain_db: f32,
    pub q: f32,
    pub slope_db_per_octave: u8,
}

impl OutputFilter {
    #[must_use]
    pub fn new(kind: OutputFilterKind, frequency_hz: f32) -> Self {
        Self {
            enabled: true,
            kind,
            frequency_hz,
            gain_db: 0.0,
            q: 0.707_106_77,
            slope_db_per_octave: 12,
        }
    }

    #[must_use]
    pub fn high_pass(frequency_hz: f32, slope_db_per_octave: u8) -> Self {
        let mut filter = Self::new(OutputFilterKind::HighPass, frequency_hz);
        filter.slope_db_per_octave = slope_db_per_octave;
        filter
    }

    #[must_use]
    pub fn low_pass(frequency_hz: f32, slope_db_per_octave: u8) -> Self {
        let mut filter = Self::new(OutputFilterKind::LowPass, frequency_hz);
        filter.slope_db_per_octave = slope_db_per_octave;
        filter
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputEnhancement {
    pub enabled: bool,
    pub amount_db: f32,
    pub frequency_hz: f32,
    pub q: f32,
}

impl OutputEnhancement {
    #[must_use]
    pub const fn disabled(frequency_hz: f32, q: f32) -> Self {
        Self {
            enabled: false,
            amount_db: 0.0,
            frequency_hz,
            q,
        }
    }

    #[must_use]
    pub const fn enabled(amount_db: f32, frequency_hz: f32, q: f32) -> Self {
        Self {
            enabled: true,
            amount_db,
            frequency_hz,
            q,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputLimiter {
    pub enabled: bool,
    pub ceiling_dbfs: f32,
}

impl Default for OutputLimiter {
    fn default() -> Self {
        Self {
            enabled: true,
            ceiling_dbfs: -1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputGamingMode {
    pub enabled: bool,
    pub impact_db: f32,
    pub impact_frequency_hz: f32,
    pub mud_cut_db: f32,
    pub mud_frequency_hz: f32,
    pub detail_db: f32,
    pub detail_frequency_hz: f32,
}

impl Default for OutputGamingMode {
    fn default() -> Self {
        Self {
            enabled: false,
            impact_db: 1.5,
            impact_frequency_hz: 110.0,
            mud_cut_db: -1.5,
            mud_frequency_hz: 300.0,
            detail_db: 2.0,
            detail_frequency_hz: 2_800.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputDspProfile {
    pub schema_version: u16,
    pub name: String,
    pub device_class: OutputDeviceClass,
    pub bypassed: bool,
    pub preamp_db: f32,
    pub filters: Vec<OutputFilter>,
    pub bass: OutputEnhancement,
    pub clarity: OutputEnhancement,
    #[serde(default)]
    pub gaming: OutputGamingMode,
    pub output_gain_db: f32,
    pub limiter: OutputLimiter,
}

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum OutputDspProfileError {
    #[error("sample rate must be finite and between 8 kHz and 384 kHz")]
    InvalidSampleRate,
    #[error("channel count must be between 1 and 32")]
    InvalidChannelCount,
    #[error("gain values must be finite and between -24 dB and +24 dB")]
    InvalidGain,
    #[error("filter or enhancement frequency is outside the usable audio band")]
    InvalidFrequency,
    #[error("Q must be finite and between 0.1 and 18")]
    InvalidQ,
    #[error("filter slope must be 6, 12, 18, 24, 36, or 48 dB/octave")]
    InvalidSlope,
    #[error("limiter ceiling must be finite and between -12 dBFS and 0 dBFS")]
    InvalidLimiterCeiling,
}

impl OutputDspProfile {
    #[must_use]
    pub fn factory(device_class: OutputDeviceClass) -> Self {
        let (preamp_db, high_pass_hz, high_pass_slope, bass_db, bass_hz, clarity_db, clarity_hz) =
            match device_class {
                OutputDeviceClass::DesktopSpeaker => (-4.0, 60.0, 24, 2.5, 95.0, 1.5, 3_000.0),
                OutputDeviceClass::BookshelfSpeaker => (-3.0, 45.0, 24, 2.0, 85.0, 1.0, 3_200.0),
                OutputDeviceClass::StudioMonitor => (-1.5, 28.0, 12, 0.0, 80.0, 0.0, 3_000.0),
                OutputDeviceClass::DesktopMonitorSpeaker => {
                    (-5.0, 100.0, 24, 1.0, 125.0, 2.0, 3_000.0)
                }
                OutputDeviceClass::TvSpeaker => (-5.5, 85.0, 24, 1.5, 110.0, 2.5, 2_700.0),
                OutputDeviceClass::Soundbar => (-4.0, 70.0, 24, 2.0, 105.0, 2.0, 2_700.0),
                OutputDeviceClass::Speaker2_1 => (-4.0, 28.0, 24, 2.5, 75.0, 1.0, 3_000.0),
                OutputDeviceClass::Speaker5_1 => (-3.0, 28.0, 24, 1.5, 80.0, 1.0, 3_000.0),
                OutputDeviceClass::Speaker7_1 => (-3.0, 28.0, 24, 1.5, 80.0, 1.0, 3_000.0),
                OutputDeviceClass::OpenBackHeadphone => (-3.0, 20.0, 12, 2.0, 75.0, 1.0, 3_200.0),
                OutputDeviceClass::ClosedBackHeadphone => (-3.0, 22.0, 12, 1.5, 80.0, 1.5, 3_000.0),
                OutputDeviceClass::GamingHeadset => (-5.0, 30.0, 18, 2.5, 90.0, 2.5, 2_800.0),
                OutputDeviceClass::Iem => (-3.5, 25.0, 12, 1.5, 85.0, 1.0, 3_200.0),
                OutputDeviceClass::WiredEarbud => (-4.0, 35.0, 18, 2.5, 95.0, 1.5, 3_000.0),
                OutputDeviceClass::BluetoothEarbud => (-4.5, 45.0, 18, 2.5, 100.0, 1.5, 2_900.0),
                OutputDeviceClass::BluetoothHeadphone => (-4.0, 30.0, 18, 2.0, 90.0, 1.0, 3_000.0),
                OutputDeviceClass::VehicleAudio => (-5.0, 28.0, 24, 3.0, 80.0, 1.0, 2_800.0),
                OutputDeviceClass::ExternalAudio => (-2.0, 20.0, 12, 0.0, 80.0, 0.0, 3_000.0),
                OutputDeviceClass::Custom => (0.0, 20.0, 12, 0.0, 80.0, 0.0, 3_000.0),
            };

        let mut filters = Vec::new();
        if device_class != OutputDeviceClass::Custom {
            filters.push(OutputFilter::high_pass(high_pass_hz, high_pass_slope));
        }

        Self {
            schema_version: 2,
            name: format!("{} — Aether Reference", device_class.label()),
            device_class,
            bypassed: false,
            preamp_db,
            filters,
            bass: if bass_db > 0.0 {
                OutputEnhancement::enabled(bass_db, bass_hz, 0.707_106_77)
            } else {
                OutputEnhancement::disabled(bass_hz, 0.707_106_77)
            },
            clarity: if clarity_db > 0.0 {
                OutputEnhancement::enabled(clarity_db, clarity_hz, 0.9)
            } else {
                OutputEnhancement::disabled(clarity_hz, 0.9)
            },
            gaming: OutputGamingMode::default(),
            output_gain_db: 0.0,
            limiter: OutputLimiter::default(),
        }
    }

    pub fn validate(
        &self,
        sample_rate_hz: f32,
        channels: usize,
    ) -> Result<(), OutputDspProfileError> {
        if !sample_rate_hz.is_finite() || !(8_000.0..=384_000.0).contains(&sample_rate_hz) {
            return Err(OutputDspProfileError::InvalidSampleRate);
        }
        if !(1..=32).contains(&channels) {
            return Err(OutputDspProfileError::InvalidChannelCount);
        }

        for gain in [self.preamp_db, self.output_gain_db] {
            validate_gain(gain)?;
        }
        for filter in &self.filters {
            validate_frequency(filter.frequency_hz, sample_rate_hz)?;
            validate_q(filter.q)?;
            validate_gain(filter.gain_db)?;
            if !matches!(filter.slope_db_per_octave, 6 | 12 | 18 | 24 | 36 | 48) {
                return Err(OutputDspProfileError::InvalidSlope);
            }
        }
        for enhancement in [&self.bass, &self.clarity] {
            validate_frequency(enhancement.frequency_hz, sample_rate_hz)?;
            validate_q(enhancement.q)?;
            validate_gain(enhancement.amount_db)?;
        }
        for gain in [
            self.gaming.impact_db,
            self.gaming.mud_cut_db,
            self.gaming.detail_db,
        ] {
            validate_gain(gain)?;
        }
        for frequency in [
            self.gaming.impact_frequency_hz,
            self.gaming.mud_frequency_hz,
            self.gaming.detail_frequency_hz,
        ] {
            validate_frequency(frequency, sample_rate_hz)?;
        }
        if !self.limiter.ceiling_dbfs.is_finite()
            || !(-12.0..=0.0).contains(&self.limiter.ceiling_dbfs)
        {
            return Err(OutputDspProfileError::InvalidLimiterCeiling);
        }
        Ok(())
    }
}

fn validate_gain(value: f32) -> Result<(), OutputDspProfileError> {
    if value.is_finite() && (-24.0..=24.0).contains(&value) {
        Ok(())
    } else {
        Err(OutputDspProfileError::InvalidGain)
    }
}

fn validate_q(value: f32) -> Result<(), OutputDspProfileError> {
    if value.is_finite() && (0.1..=18.0).contains(&value) {
        Ok(())
    } else {
        Err(OutputDspProfileError::InvalidQ)
    }
}

fn validate_frequency(value: f32, sample_rate_hz: f32) -> Result<(), OutputDspProfileError> {
    let upper = sample_rate_hz * 0.49;
    if value.is_finite() && (10.0..=upper).contains(&value) {
        Ok(())
    } else {
        Err(OutputDspProfileError::InvalidFrequency)
    }
}
