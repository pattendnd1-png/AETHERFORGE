use crate::{FRAME_SAMPLES, SAMPLE_RATE};
use forgehx_core::{
    ExtraneousNoiseMonitorConfig, NoiseAdaptationState, NoiseBandLevels, NoiseClassScores,
    NoiseSceneTelemetry,
};
use std::collections::VecDeque;

const PROBE_HZ: &[f32] = &[
    50.0, 60.0, 100.0, 120.0, 180.0, 240.0, 320.0, 450.0, 630.0, 900.0, 1250.0, 1800.0, 2500.0,
    3500.0, 5000.0, 6500.0, 8000.0, 10000.0, 12500.0, 15000.0, 18000.0,
];
const STABLE_LEARN_FRAMES: u32 = 30;
const FLOOR_DOWN_ALPHA: f32 = 0.18;
const FLOOR_UP_ALPHA: f32 = 0.025;
const BAND_STABILITY_DB: f32 = 2.0;
const SPEECH_HOLD_FRAMES: u32 = 12;
const RENDER_HISTORY_FRAMES: usize = 6;

#[derive(Debug, Clone)]
pub struct NoiseObservation {
    pub telemetry: NoiseSceneTelemetry,
    pub stable_non_speech: bool,
}

#[derive(Debug)]
pub struct ExtraneousNoiseMonitor {
    band_floor_dbfs: NoiseBandLevels,
    last_band_dbfs: NoiseBandLevels,
    stable_frames: u32,
    last_overall_dbfs: f32,
    last_probe_db: Vec<f32>,
    last_local_overall_dbfs: f32,
    last_local_probe_db: Vec<f32>,
    render_history: VecDeque<[f32; FRAME_SAMPLES]>,
    telemetry: NoiseSceneTelemetry,
    speech_votes: VecDeque<bool>,
    speech_hold_frames: u32,
}

impl Default for ExtraneousNoiseMonitor {
    fn default() -> Self {
        Self {
            band_floor_dbfs: NoiseBandLevels::default(),
            last_band_dbfs: NoiseBandLevels::default(),
            stable_frames: 0,
            last_overall_dbfs: -90.0,
            last_probe_db: vec![-90.0; PROBE_HZ.len()],
            last_local_overall_dbfs: -90.0,
            last_local_probe_db: vec![-90.0; PROBE_HZ.len()],
            render_history: VecDeque::with_capacity(RENDER_HISTORY_FRAMES),
            telemetry: NoiseSceneTelemetry::default(),
            speech_votes: VecDeque::with_capacity(3),
            speech_hold_frames: 0,
        }
    }
}

impl ExtraneousNoiseMonitor {
    pub fn telemetry(&self) -> NoiseSceneTelemetry {
        self.telemetry.clone()
    }

    pub fn analyze(
        &mut self,
        raw_capture: &[f32; FRAME_SAMPLES],
        cleaned_capture: &[f32; FRAME_SAMPLES],
        render: Option<&[f32; FRAME_SAMPLES]>,
        config: &ExtraneousNoiseMonitorConfig,
    ) -> NoiseObservation {
        if !config.enabled {
            self.telemetry = NoiseSceneTelemetry::default();
            return NoiseObservation {
                telemetry: self.telemetry.clone(),
                stable_non_speech: false,
            };
        }

        if let Some(render_frame) = render {
            if self.render_history.len() >= RENDER_HISTORY_FRAMES {
                self.render_history.pop_front();
            }
            self.render_history.push_back(*render_frame);
        }

        let probe_db = probe_dbfs(raw_capture);
        let band_levels = band_levels_from_probes(&probe_db);
        let overall_dbfs = rms_dbfs(raw_capture);
        let cleaned_dbfs = rms_dbfs(cleaned_capture);
        let slope_db_per_oct = spectral_slope_db_per_octave(&probe_db);
        let spread_db = spectral_spread_db(&probe_db);
        let tonal = tonal_score(&probe_db);
        let transient = transient_score(raw_capture, overall_dbfs, self.last_overall_dbfs);
        let occupancy = spectral_occupancy(&probe_db);
        let broadband = (occupancy * (1.0 - tonal * 0.75)).clamp(0.0, 1.0);
        let white_like =
            (likeness(slope_db_per_oct, 0.0, 3.0) * broadband * spread_penalty(spread_db, 15.0))
                .clamp(0.0, 1.0);
        let pink_like =
            (likeness(slope_db_per_oct, -3.0, 3.0) * broadband * spread_penalty(spread_db, 18.0))
                .clamp(0.0, 1.0);
        let hum_rumble = hum_score(&probe_db);
        let narrowband_whine = whine_score(&probe_db);
        let speaker_leak = self.speaker_leak_score(cleaned_capture, render);

        let _scene_spectral_flux = spectral_flux(&probe_db, &self.last_probe_db);
        let level_delta = (overall_dbfs - self.last_overall_dbfs).abs();
        let local_probe_db = probe_dbfs(cleaned_capture);
        let local_spectral_flux = spectral_flux(&local_probe_db, &self.last_local_probe_db);
        let local_level_delta = (cleaned_dbfs - self.last_local_overall_dbfs).abs();
        let floor_reference = average_band_floor(&self.band_floor_dbfs).max(-90.0);
        let dynamic_local_energy = cleaned_dbfs >= floor_reference + 8.0
            && transient < 0.60
            && speaker_leak < 0.35
            && (local_spectral_flux >= 0.10 || local_level_delta >= 2.5);
        self.speech_votes.push_back(dynamic_local_energy);
        while self.speech_votes.len() > 3 {
            self.speech_votes.pop_front();
        }
        let vote_count = self.speech_votes.iter().filter(|&&value| value).count();
        if vote_count >= 2 {
            self.speech_hold_frames = SPEECH_HOLD_FRAMES;
        } else if self.speech_hold_frames > 0 {
            self.speech_hold_frames -= 1;
        }
        let speech_protected = self.speech_hold_frames > 0;
        let render_active = render.map(rms_dbfs).unwrap_or(-90.0) > -55.0;
        let double_talk =
            render.is_some() && render_active && speech_protected && speaker_leak > 0.20;

        let stable_level = level_delta <= BAND_STABILITY_DB;
        let stable_bands = bands_within(&band_levels, &self.last_band_dbfs, BAND_STABILITY_DB);
        let stable_non_speech =
            stable_level && stable_bands && !speech_protected && !double_talk && transient < 0.60;
        if stable_non_speech {
            self.stable_frames = self.stable_frames.saturating_add(1);
        } else {
            self.stable_frames = 0;
        }
        update_band_floors(
            &mut self.band_floor_dbfs,
            &band_levels,
            self.stable_frames >= STABLE_LEARN_FRAMES,
            speech_protected || double_talk || transient >= 0.60,
        );

        let dominant_bands = dominant_bands(&band_levels, &self.band_floor_dbfs);
        let adaptation_state = if speech_protected || double_talk {
            NoiseAdaptationState::SpeechProtected
        } else if render.is_none() && config.speaker_rejection_enabled {
            NoiseAdaptationState::ReferenceUnavailable
        } else if stable_non_speech && self.stable_frames >= STABLE_LEARN_FRAMES {
            NoiseAdaptationState::Suppressing
        } else if stable_non_speech {
            NoiseAdaptationState::Learning
        } else {
            NoiseAdaptationState::Holding
        };

        self.telemetry = NoiseSceneTelemetry {
            active: true,
            reference_available: render.is_some(),
            adaptation_state,
            overall_noise_dbfs: cleaned_dbfs.min(overall_dbfs),
            classes: NoiseClassScores {
                white_like,
                pink_like,
                broadband,
                hum_rumble,
                narrowband_whine,
                transient,
                speaker_leak,
            },
            band_floor_dbfs: self.band_floor_dbfs.clone(),
            dominant_bands,
            adaptive_suppression_db: self.telemetry.adaptive_suppression_db,
            sonora_noise_target_percent: self.telemetry.sonora_noise_target_percent,
            speech_protected,
            double_talk,
        };

        self.last_band_dbfs = band_levels;
        self.last_overall_dbfs = overall_dbfs;
        self.last_probe_db = probe_db;
        self.last_local_overall_dbfs = cleaned_dbfs;
        self.last_local_probe_db = local_probe_db;
        NoiseObservation {
            telemetry: self.telemetry.clone(),
            stable_non_speech,
        }
    }

    fn speaker_leak_score(
        &self,
        cleaned_capture: &[f32; FRAME_SAMPLES],
        render: Option<&[f32; FRAME_SAMPLES]>,
    ) -> f32 {
        let mut best = render
            .map(|frame| normalized_correlation(cleaned_capture, frame))
            .unwrap_or(0.0);
        for frame in &self.render_history {
            best = best.max(normalized_correlation(cleaned_capture, frame));
        }
        best.clamp(0.0, 1.0)
    }
}

fn goertzel_power(frame: &[f32; FRAME_SAMPLES], hz: f32) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * hz / SAMPLE_RATE;
    let coeff = 2.0 * omega.cos();
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for (index, &sample) in frame.iter().enumerate() {
        let window = 0.5
            - 0.5 * (2.0 * std::f32::consts::PI * index as f32 / (FRAME_SAMPLES - 1) as f32).cos();
        let x = sample * window;
        let s0 = x + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(1.0e-12)
}

fn probe_dbfs(frame: &[f32; FRAME_SAMPLES]) -> Vec<f32> {
    PROBE_HZ
        .iter()
        .map(|&hz| {
            let amplitude = 2.0 * goertzel_power(frame, hz).sqrt() / FRAME_SAMPLES as f32;
            (20.0 * amplitude.max(1.0e-6).log10()).clamp(-90.0, 0.0)
        })
        .collect()
}

fn rms_dbfs(frame: &[f32; FRAME_SAMPLES]) -> f32 {
    let mean = frame.iter().map(|sample| sample * sample).sum::<f32>() / FRAME_SAMPLES as f32;
    (20.0 * mean.sqrt().max(1.0e-6).log10()).clamp(-90.0, 0.0)
}

fn spectral_slope_db_per_octave(probe_db: &[f32]) -> f32 {
    // A 10 ms / 48 kHz frame cannot resolve enough cycles below 180 Hz for
    // a stable broadband spectral-slope estimate. Keep those probes for the
    // dedicated hum/rumble classifier, but exclude them from pink/white
    // regression so low-frequency window leakage cannot bias the slope.
    const BROADBAND_SLOPE_START: usize = 4;
    let values = &probe_db[BROADBAND_SLOPE_START.min(probe_db.len())..];
    let frequencies = &PROBE_HZ[BROADBAND_SLOPE_START.min(PROBE_HZ.len())..];
    if values.len() < 2 || values.len() != frequencies.len() {
        return 0.0;
    }

    let xs = frequencies.iter().map(|hz| hz.log2()).collect::<Vec<_>>();
    let mean_x = xs.iter().sum::<f32>() / xs.len() as f32;
    let mean_y = values.iter().sum::<f32>() / values.len() as f32;
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for (x, y) in xs.iter().zip(values) {
        numerator += (*x - mean_x) * (*y - mean_y);
        denominator += (*x - mean_x) * (*x - mean_x);
    }
    numerator / denominator.max(1.0e-6)
}

fn spectral_flux(now: &[f32], previous: &[f32]) -> f32 {
    if now.len() != previous.len() || now.is_empty() {
        return 0.0;
    }
    let sum = now
        .iter()
        .zip(previous)
        .map(|(a, b)| ((a - b).max(0.0) / 18.0).clamp(0.0, 1.0))
        .sum::<f32>();
    (sum / now.len() as f32).clamp(0.0, 1.0)
}

fn spectral_spread_db(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    (values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f32>()
        / values.len() as f32)
        .sqrt()
}

fn spectral_occupancy(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let max = values.iter().copied().fold(-90.0f32, f32::max);
    let count = values.iter().filter(|&&value| value >= max - 18.0).count();
    (count as f32 / values.len() as f32).clamp(0.0, 1.0)
}

fn tonal_score(values: &[f32]) -> f32 {
    if values.len() < 3 {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = sorted[sorted.len() / 2];
    let peak = values.iter().copied().fold(-90.0f32, f32::max);
    ((peak - median - 6.0) / 18.0).clamp(0.0, 1.0)
}

fn transient_score(frame: &[f32; FRAME_SAMPLES], level_db: f32, last_level_db: f32) -> f32 {
    let peak = frame
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0f32, f32::max);
    let rms = 10.0f32.powf(level_db / 20.0).max(1.0e-6);
    let crest = peak / rms;
    let crest_score = ((crest - 3.5) / 7.0).clamp(0.0, 1.0);
    let jump_score = ((level_db - last_level_db - 5.0) / 18.0).clamp(0.0, 1.0);
    crest_score.max(jump_score)
}

fn hum_score(values: &[f32]) -> f32 {
    if values.len() < 6 {
        return 0.0;
    }
    let hum_peak = [0usize, 1, 2, 3, 4, 5]
        .into_iter()
        .map(|index| values[index])
        .fold(-90.0f32, f32::max);
    let mid_mean = values[8..15].iter().sum::<f32>() / 7.0;
    ((hum_peak - mid_mean - 4.0) / 18.0).clamp(0.0, 1.0)
}

fn whine_score(values: &[f32]) -> f32 {
    if values.len() < 15 {
        return 0.0;
    }
    let high = &values[12..];
    let peak = high.iter().copied().fold(-90.0f32, f32::max);
    let mean = high.iter().sum::<f32>() / high.len() as f32;
    ((peak - mean - 5.0) / 16.0).clamp(0.0, 1.0)
}

fn likeness(value: f32, center: f32, width: f32) -> f32 {
    (1.0 - (value - center).abs() / width.max(0.1)).clamp(0.0, 1.0)
}

fn spread_penalty(spread_db: f32, allowed: f32) -> f32 {
    (1.0 - ((spread_db - allowed).max(0.0) / allowed.max(1.0))).clamp(0.0, 1.0)
}

fn band_levels_from_probes(values: &[f32]) -> NoiseBandLevels {
    NoiseBandLevels {
        sub_rumble_dbfs: average(&values[0..4]),
        low_dbfs: average(&values[4..7]),
        low_mid_dbfs: average(&values[7..10]),
        mid_dbfs: average(&values[10..13]),
        presence_dbfs: average(&values[13..16]),
        high_dbfs: average(&values[16..19]),
        air_dbfs: average(&values[19..21]),
    }
}

fn average(values: &[f32]) -> f32 {
    if values.is_empty() {
        -90.0
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

fn average_band_floor(levels: &NoiseBandLevels) -> f32 {
    average(&[
        levels.sub_rumble_dbfs,
        levels.low_dbfs,
        levels.low_mid_dbfs,
        levels.mid_dbfs,
        levels.presence_dbfs,
        levels.high_dbfs,
        levels.air_dbfs,
    ])
}

fn bands_within(a: &NoiseBandLevels, b: &NoiseBandLevels, tolerance: f32) -> bool {
    [
        (a.sub_rumble_dbfs, b.sub_rumble_dbfs),
        (a.low_dbfs, b.low_dbfs),
        (a.low_mid_dbfs, b.low_mid_dbfs),
        (a.mid_dbfs, b.mid_dbfs),
        (a.presence_dbfs, b.presence_dbfs),
        (a.high_dbfs, b.high_dbfs),
        (a.air_dbfs, b.air_dbfs),
    ]
    .into_iter()
    .all(|(left, right)| (left - right).abs() <= tolerance)
}

fn update_band_floors(
    floor: &mut NoiseBandLevels,
    current: &NoiseBandLevels,
    stable_long_enough: bool,
    freeze_upward: bool,
) {
    update_floor(
        &mut floor.sub_rumble_dbfs,
        current.sub_rumble_dbfs,
        stable_long_enough,
        freeze_upward,
    );
    update_floor(
        &mut floor.low_dbfs,
        current.low_dbfs,
        stable_long_enough,
        freeze_upward,
    );
    update_floor(
        &mut floor.low_mid_dbfs,
        current.low_mid_dbfs,
        stable_long_enough,
        freeze_upward,
    );
    update_floor(
        &mut floor.mid_dbfs,
        current.mid_dbfs,
        stable_long_enough,
        freeze_upward,
    );
    update_floor(
        &mut floor.presence_dbfs,
        current.presence_dbfs,
        stable_long_enough,
        freeze_upward,
    );
    update_floor(
        &mut floor.high_dbfs,
        current.high_dbfs,
        stable_long_enough,
        freeze_upward,
    );
    update_floor(
        &mut floor.air_dbfs,
        current.air_dbfs,
        stable_long_enough,
        freeze_upward,
    );
}

fn update_floor(floor: &mut f32, current: f32, stable_long_enough: bool, freeze_upward: bool) {
    let current = current.clamp(-90.0, -12.0);
    if current < *floor {
        *floor += (current - *floor) * FLOOR_DOWN_ALPHA;
    } else if stable_long_enough && !freeze_upward {
        *floor += (current - *floor) * FLOOR_UP_ALPHA;
    }
    *floor = (*floor).clamp(-90.0, -12.0);
}

fn dominant_bands(current: &NoiseBandLevels, floor: &NoiseBandLevels) -> Vec<String> {
    let candidates = [
        ("sub-rumble", current.sub_rumble_dbfs, floor.sub_rumble_dbfs),
        ("low", current.low_dbfs, floor.low_dbfs),
        ("low-mid", current.low_mid_dbfs, floor.low_mid_dbfs),
        ("mid", current.mid_dbfs, floor.mid_dbfs),
        ("presence", current.presence_dbfs, floor.presence_dbfs),
        ("high", current.high_dbfs, floor.high_dbfs),
        ("air", current.air_dbfs, floor.air_dbfs),
    ];
    let mut ranked = candidates
        .into_iter()
        .map(|(name, level, learned)| (name, level.max(learned)))
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranked
        .into_iter()
        .take(3)
        .map(|(name, _)| name.to_owned())
        .collect()
}

fn normalized_correlation(a: &[f32; FRAME_SAMPLES], b: &[f32; FRAME_SAMPLES]) -> f32 {
    let mut dot = 0.0;
    let mut aa = 0.0;
    let mut bb = 0.0;
    for i in 0..FRAME_SAMPLES {
        dot += a[i] * b[i];
        aa += a[i] * a[i];
        bb += b[i] * b[i];
    }
    (dot.abs() / (aa.sqrt() * bb.sqrt()).max(1.0e-9)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xorshift(seed: &mut u32) -> f32 {
        let mut x = *seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *seed = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    fn white_noise(mut seed: u32, amplitude: f32) -> [f32; FRAME_SAMPLES] {
        let mut out = [0.0; FRAME_SAMPLES];
        for sample in &mut out {
            *sample = xorshift(&mut seed) * amplitude;
        }
        out
    }

    fn shaped_noise(mut seed: u32, amplitude: f32, pink: bool) -> [f32; FRAME_SAMPLES] {
        let white = white_noise(seed, amplitude);
        let mut out = [0.0; FRAME_SAMPLES];
        if pink {
            let mut state = 0.0;
            for (dst, src) in out.iter_mut().zip(white) {
                state = state * 0.92 + src * 0.08;
                *dst = (src * 0.35 + state * 1.8).clamp(-1.0, 1.0);
            }
        } else {
            out = white;
        }
        seed ^= 0x9E37_79B9;
        let _ = seed;
        out
    }

    fn sine(hz: f32, amplitude: f32) -> [f32; FRAME_SAMPLES] {
        let mut out = [0.0; FRAME_SAMPLES];
        for (index, sample) in out.iter_mut().enumerate() {
            *sample =
                (2.0 * std::f32::consts::PI * hz * index as f32 / SAMPLE_RATE).sin() * amplitude;
        }
        out
    }

    fn mixed_tones(tones: &[(f32, f32)]) -> [f32; FRAME_SAMPLES] {
        let mut out = [0.0; FRAME_SAMPLES];
        for &(hz, amplitude) in tones {
            let tone = sine(hz, amplitude);
            for (dst, src) in out.iter_mut().zip(tone) {
                *dst += src;
            }
        }
        out
    }

    fn scaled_plus_noise(
        render: &[f32; FRAME_SAMPLES],
        scale: f32,
        noise_amp: f32,
    ) -> [f32; FRAME_SAMPLES] {
        let noise = white_noise(0xA5A5_1234, noise_amp);
        let mut out = [0.0; FRAME_SAMPLES];
        for i in 0..FRAME_SAMPLES {
            out[i] = render[i] * scale + noise[i];
        }
        out
    }

    fn run_stable(
        monitor: &mut ExtraneousNoiseMonitor,
        frame: &[f32; FRAME_SAMPLES],
        render: Option<&[f32; FRAME_SAMPLES]>,
        cfg: &ExtraneousNoiseMonitorConfig,
        frames: usize,
    ) -> NoiseSceneTelemetry {
        let mut telemetry = NoiseSceneTelemetry::default();
        for _ in 0..frames {
            telemetry = monitor.analyze(frame, frame, render, cfg).telemetry;
        }
        telemetry
    }

    #[test]
    fn white_fixture_scores_more_white_than_pink() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let frame = shaped_noise(0x1234_5678, 0.03, false);
        let t = run_stable(&mut monitor, &frame, None, &cfg, 120);
        assert!(
            t.classes.white_like > t.classes.pink_like,
            "{:#?}",
            t.classes
        );
    }

    #[test]
    fn pink_fixture_scores_more_pink_than_white() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let frame = shaped_noise(0x1234_5678, 0.03, true);
        let t = run_stable(&mut monitor, &frame, None, &cfg, 120);
        assert!(
            t.classes.pink_like > t.classes.white_like,
            "{:#?}",
            t.classes
        );
    }

    #[test]
    fn sixty_hz_hum_is_detected_without_becoming_broadband() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let frame = sine(60.0, 0.08);
        let t = run_stable(&mut monitor, &frame, None, &cfg, 80);
        assert!(t.classes.hum_rumble > 0.60, "{:#?}", t.classes);
        assert!(t.classes.hum_rumble > t.classes.broadband);
    }

    #[test]
    fn persistent_6khz_whine_is_detected() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let frame = sine(6500.0, 0.06);
        let t = run_stable(&mut monitor, &frame, None, &cfg, 80);
        assert!(t.classes.narrowband_whine > 0.55, "{:#?}", t.classes);
    }

    #[test]
    fn click_is_transient_and_does_not_raise_floor_after_one_frame() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let quiet = [0.0; FRAME_SAMPLES];
        let before = run_stable(&mut monitor, &quiet, None, &cfg, 40).band_floor_dbfs;
        let mut click = quiet;
        click[120] = 0.9;
        let after = monitor.analyze(&click, &click, None, &cfg).telemetry;
        assert!(after.classes.transient > 0.60, "{:#?}", after.classes);
        assert!((after.band_floor_dbfs.mid_dbfs - before.mid_dbfs).abs() < 0.5);
    }

    #[test]
    fn steady_shaped_fan_noise_becomes_broadband_scene() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let frame = shaped_noise(0xCAFE_BABE, 0.025, true);
        let t = run_stable(&mut monitor, &frame, None, &cfg, 120);
        assert!(t.classes.broadband > 0.35, "{:#?}", t.classes);
    }

    #[test]
    fn correlated_playback_scores_as_speaker_leak() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let render = mixed_tones(&[(440.0, 0.03), (1300.0, 0.02), (4200.0, 0.01)]);
        let capture = scaled_plus_noise(&render, 0.65, 0.002);
        let t = run_stable(&mut monitor, &capture, Some(&render), &cfg, 80);
        assert!(t.classes.speaker_leak > 0.70, "{:#?}", t.classes);
        assert!(!t.double_talk);
    }

    #[test]
    fn local_speech_over_playback_enters_double_talk_protection() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let render = mixed_tones(&[(440.0, 0.025), (1300.0, 0.02)]);
        let mut telemetry = NoiseSceneTelemetry::default();
        for frame_index in 0..20 {
            let mut capture = scaled_plus_noise(&render, 0.55, 0.001);
            let speech = sine(
                220.0 + (frame_index % 5) as f32 * 35.0,
                0.05 + (frame_index % 3) as f32 * 0.01,
            );
            for i in 0..FRAME_SAMPLES {
                capture[i] += speech[i];
            }
            telemetry = monitor
                .analyze(&capture, &capture, Some(&render), &cfg)
                .telemetry;
        }
        assert!(
            telemetry.speech_protected || telemetry.double_talk,
            "{telemetry:#?}"
        );
    }

    #[test]
    fn uncorrelated_speech_is_not_classified_as_speaker_leak() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let render = sine(900.0, 0.04);
        let capture = sine(233.0, 0.06);
        let t = run_stable(&mut monitor, &capture, Some(&render), &cfg, 20);
        assert!(t.classes.speaker_leak < 0.35, "{:#?}", t.classes);
    }

    #[test]
    fn reference_loss_keeps_environment_monitoring_active() {
        let mut monitor = ExtraneousNoiseMonitor::default();
        let cfg = ExtraneousNoiseMonitorConfig::default();
        let frame = shaped_noise(0x1111_2222, 0.02, false);
        let t = run_stable(&mut monitor, &frame, None, &cfg, 40);
        assert!(t.active);
        assert!(!t.reference_available);
    }
}
