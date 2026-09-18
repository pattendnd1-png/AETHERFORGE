//! App-private microphone DSP.
//!
//! This engine has no PipeWire ownership code and no AetherForge system-DSP
//! integration. It consumes mono `f32` samples supplied by `private_audio` and
//! applies the current BEACN Live Profile entirely inside this process.

use crate::hardware::{EqBandKind, NoiseStyle};
use crate::software_dsp::{MIC_EQ_BAND_COUNT, SoftwareDspState};
use std::f32::consts::PI;

pub const PRIVATE_DSP_SAMPLE_RATE_HZ: u32 = 48_000;
pub const PRIVATE_DSP_BLOCK_FRAMES: usize = 480;

#[derive(Debug, Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Default for Biquad {
    fn default() -> Self {
        Self::bypass()
    }
}

impl Biquad {
    const fn bypass() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    fn set_bypass(&mut self) {
        self.b0 = 1.0;
        self.b1 = 0.0;
        self.b2 = 0.0;
        self.a1 = 0.0;
        self.a2 = 0.0;
    }

    fn set_coefficients(&mut self, b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) {
        let denom = if a0.abs() < 1.0e-12 { 1.0 } else { a0 };
        self.b0 = b0 / denom;
        self.b1 = b1 / denom;
        self.b2 = b2 / denom;
        self.a1 = a1 / denom;
        self.a2 = a2 / denom;
    }

    fn configure(
        &mut self,
        kind: EqBandKind,
        gain_db: f32,
        frequency_hz: f32,
        q: f32,
        sample_rate_hz: f32,
    ) {
        if matches!(kind, EqBandKind::NotSet) {
            self.set_bypass();
            return;
        }

        let frequency_hz = frequency_hz.clamp(20.0, sample_rate_hz * 0.45);
        let q = q.clamp(0.1, 10.0);
        let w0 = 2.0 * PI * frequency_hz / sample_rate_hz;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        match kind {
            EqBandKind::NotSet => self.set_bypass(),
            EqBandKind::LowPass => {
                let b0 = (1.0 - cos_w0) * 0.5;
                let b1 = 1.0 - cos_w0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                self.set_coefficients(b0, b1, b2, a0, a1, a2);
            }
            EqBandKind::HighPass => {
                let b0 = (1.0 + cos_w0) * 0.5;
                let b1 = -(1.0 + cos_w0);
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                self.set_coefficients(b0, b1, b2, a0, a1, a2);
            }
            EqBandKind::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha;
                self.set_coefficients(b0, b1, b2, a0, a1, a2);
            }
            EqBandKind::Bell => {
                let a = 10.0_f32.powf(gain_db / 40.0);
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;
                self.set_coefficients(b0, b1, b2, a0, a1, a2);
            }
            EqBandKind::LowShelf => {
                let a = 10.0_f32.powf(gain_db / 40.0);
                let sqrt_a = a.sqrt();
                let shelf_alpha = sin_w0 * 0.5 * 2.0_f32.sqrt();
                let two_sqrt_a_alpha = 2.0 * sqrt_a * shelf_alpha;
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                self.set_coefficients(b0, b1, b2, a0, a1, a2);
            }
            EqBandKind::HighShelf => {
                let a = 10.0_f32.powf(gain_db / 40.0);
                let sqrt_a = a.sqrt();
                let shelf_alpha = sin_w0 * 0.5 * 2.0_f32.sqrt();
                let two_sqrt_a_alpha = 2.0 * sqrt_a * shelf_alpha;
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                self.set_coefficients(b0, b1, b2, a0, a1, a2);
            }
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1;
        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;
        output
    }
}

#[derive(Debug)]
pub struct PrivateDspEngine {
    sample_rate_hz: f32,
    eq: [Biquad; MIC_EQ_BAND_COUNT],
    de_esser_highpass: Biquad,
    exciter_highpass: Biquad,
    suppress_envelope: f32,
    suppress_gain: f32,
    expander_envelope: f32,
    expander_gain: f32,
    compressor_envelope: f32,
    compressor_gain: f32,
    de_esser_envelope: f32,
}

impl PrivateDspEngine {
    pub fn new(sample_rate_hz: f32) -> Self {
        Self {
            sample_rate_hz: sample_rate_hz.max(8_000.0),
            eq: [Biquad::bypass(); MIC_EQ_BAND_COUNT],
            de_esser_highpass: Biquad::bypass(),
            exciter_highpass: Biquad::bypass(),
            suppress_envelope: 0.0,
            suppress_gain: 1.0,
            expander_envelope: 0.0,
            expander_gain: 1.0,
            compressor_envelope: 0.0,
            compressor_gain: 1.0,
            de_esser_envelope: 0.0,
        }
    }

    pub fn reset(&mut self) {
        for filter in &mut self.eq {
            filter.reset();
        }
        self.de_esser_highpass.reset();
        self.exciter_highpass.reset();
        self.suppress_envelope = 0.0;
        self.suppress_gain = 1.0;
        self.expander_envelope = 0.0;
        self.expander_gain = 1.0;
        self.compressor_envelope = 0.0;
        self.compressor_gain = 1.0;
        self.de_esser_envelope = 0.0;
    }

    pub fn process_mono_block(&mut self, samples: &mut [f32], profile: &SoftwareDspState) {
        let mut profile = profile.clone();
        profile.sanitize();
        self.configure_filters(&profile);

        let input_gain = db_to_gain(profile.mic_gain_db);
        let output_gain = db_to_gain(profile.mic_output_gain_db);
        let suppress_threshold = db_to_gain(profile.suppressor.sensitivity_db);
        let suppress_amount = (profile.suppressor.amount / 100.0).clamp(0.0, 1.0);
        let de_esser_amount = (profile.de_esser.amount / 100.0).clamp(0.0, 1.0);
        let exciter_amount = (profile.exciter.amount / 100.0).clamp(0.0, 1.0);
        let exciter_drive = 1.0 + (profile.exciter.tone / 100.0).clamp(0.0, 1.0) * 4.0;

        let compressor_attack = smoothing(profile.compressor.attack_ms, self.sample_rate_hz);
        let compressor_release = smoothing(profile.compressor.release_ms, self.sample_rate_hz);
        let expander_attack = smoothing(profile.expander.attack_ms, self.sample_rate_hz);
        let expander_release = smoothing(profile.expander.release_ms, self.sample_rate_hz);
        let suppress_speed = match profile.suppressor.style {
            NoiseStyle::Instant => 0.40,
            NoiseStyle::Adaptive => {
                let frames = (profile.suppressor.adapt_ms * 0.001 * self.sample_rate_hz).max(1.0);
                (1.0 / frames).clamp(0.0001, 0.05)
            }
            NoiseStyle::Snapshot => 0.02,
        };

        for sample in samples {
            let mut value = finite_or_zero(*sample) * input_gain;

            for (filter, band) in self.eq.iter_mut().zip(profile.mic_eq.iter()) {
                if band.enabled && !matches!(band.kind, EqBandKind::NotSet) {
                    value = filter.process(value);
                }
            }

            if profile.suppressor.enabled {
                self.suppress_envelope = envelope_follow(
                    self.suppress_envelope,
                    value.abs(),
                    0.005,
                    0.120,
                    self.sample_rate_hz,
                );
                let floor = 1.0 - suppress_amount * 0.97;
                let target = if self.suppress_envelope < suppress_threshold {
                    floor
                } else {
                    1.0
                };
                self.suppress_gain += (target - self.suppress_gain) * suppress_speed;
                value *= self.suppress_gain.clamp(0.0, 1.0);
            }

            if profile.expander.enabled {
                self.expander_envelope = envelope_follow_with_coefficients(
                    self.expander_envelope,
                    value.abs(),
                    expander_attack,
                    expander_release,
                );
                let level_db = gain_to_db(self.expander_envelope);
                let target_gain = if level_db < profile.expander.threshold_db {
                    let reduction_db = ((profile.expander.threshold_db - level_db)
                        * (profile.expander.ratio - 1.0))
                        .clamp(0.0, 60.0);
                    db_to_gain(-reduction_db)
                } else {
                    1.0
                };
                let coeff = if target_gain < self.expander_gain {
                    expander_attack
                } else {
                    expander_release
                };
                self.expander_gain = coeff * self.expander_gain + (1.0 - coeff) * target_gain;
                value *= self.expander_gain.clamp(0.0, 1.0);
            }

            if profile.compressor.enabled {
                self.compressor_envelope = envelope_follow_with_coefficients(
                    self.compressor_envelope,
                    value.abs(),
                    compressor_attack,
                    compressor_release,
                );
                let level_db = gain_to_db(self.compressor_envelope);
                let target_gain = if level_db > profile.compressor.threshold_db {
                    let compressed_db = profile.compressor.threshold_db
                        + (level_db - profile.compressor.threshold_db)
                            / profile.compressor.ratio.max(1.0);
                    db_to_gain(compressed_db - level_db + profile.compressor.makeup_gain_db)
                } else {
                    db_to_gain(profile.compressor.makeup_gain_db)
                };
                let coeff = if target_gain < self.compressor_gain {
                    compressor_attack
                } else {
                    compressor_release
                };
                self.compressor_gain = coeff * self.compressor_gain + (1.0 - coeff) * target_gain;
                value *= self.compressor_gain.clamp(0.0, 4.0);
            }

            if profile.de_esser.enabled {
                let high = self.de_esser_highpass.process(value);
                self.de_esser_envelope = envelope_follow(
                    self.de_esser_envelope,
                    high.abs(),
                    0.002,
                    0.060,
                    self.sample_rate_hz,
                );
                let activity = (self.de_esser_envelope * 8.0).clamp(0.0, 1.0);
                value -= high * de_esser_amount * activity * 0.75;
            }

            if profile.exciter.enabled {
                let high = self.exciter_highpass.process(value);
                let harmonics = (high * exciter_drive).tanh() - high;
                value += harmonics * exciter_amount * 0.5;
            }

            value *= output_gain;
            *sample = safety_limit(value);
        }
    }

    fn configure_filters(&mut self, profile: &SoftwareDspState) {
        for (filter, band) in self.eq.iter_mut().zip(profile.mic_eq.iter()) {
            if band.enabled {
                filter.configure(
                    band.kind,
                    band.gain_db,
                    band.frequency_hz,
                    band.q,
                    self.sample_rate_hz,
                );
            } else {
                filter.set_bypass();
            }
        }
        self.de_esser_highpass.configure(
            EqBandKind::HighPass,
            0.0,
            profile.de_esser.frequency_hz,
            0.707,
            self.sample_rate_hz,
        );
        self.exciter_highpass.configure(
            EqBandKind::HighPass,
            0.0,
            2_500.0,
            0.707,
            self.sample_rate_hz,
        );
    }
}

fn smoothing(time_ms: f32, sample_rate_hz: f32) -> f32 {
    let seconds = (time_ms.max(0.1) * 0.001).max(1.0 / sample_rate_hz);
    (-1.0 / (seconds * sample_rate_hz)).exp()
}

fn envelope_follow(
    current: f32,
    input: f32,
    attack_seconds: f32,
    release_seconds: f32,
    sample_rate_hz: f32,
) -> f32 {
    let attack = (-1.0 / (attack_seconds.max(0.0001) * sample_rate_hz)).exp();
    let release = (-1.0 / (release_seconds.max(0.0001) * sample_rate_hz)).exp();
    envelope_follow_with_coefficients(current, input, attack, release)
}

fn envelope_follow_with_coefficients(current: f32, input: f32, attack: f32, release: f32) -> f32 {
    let coefficient = if input > current { attack } else { release };
    coefficient * current + (1.0 - coefficient) * input
}

pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

pub fn gain_to_db(gain: f32) -> f32 {
    20.0 * gain.max(1.0e-9).log10()
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn safety_limit(value: f32) -> f32 {
    finite_or_zero(value).clamp(-8.0, 8.0).tanh()
}

pub fn private_dsp_self_test() -> Result<(), String> {
    let mut engine = PrivateDspEngine::new(PRIVATE_DSP_SAMPLE_RATE_HZ as f32);
    let mut profile = SoftwareDspState {
        mic_gain_db: 6.0,
        mic_output_gain_db: 6.0,
        ..SoftwareDspState::default()
    };
    profile.compressor.enabled = true;
    profile.expander.enabled = true;
    profile.suppressor.enabled = true;
    profile.de_esser.enabled = true;
    profile.exciter.enabled = true;

    let mut block = [0.0_f32; PRIVATE_DSP_BLOCK_FRAMES];
    for (index, sample) in block.iter_mut().enumerate() {
        let phase = 2.0 * PI * 1_000.0 * index as f32 / PRIVATE_DSP_SAMPLE_RATE_HZ as f32;
        *sample = phase.sin() * 0.8;
    }
    engine.process_mono_block(&mut block, &profile);
    if block.iter().any(|sample| !sample.is_finite()) {
        return Err("private DSP produced a non-finite sample".to_owned());
    }
    if block.iter().any(|sample| sample.abs() > 1.0) {
        return Err("private DSP safety limiter exceeded unity".to_owned());
    }
    if block.iter().all(|sample| sample.abs() < 1.0e-7) {
        return Err("private DSP unexpectedly produced silence".to_owned());
    }
    Ok(())
}
