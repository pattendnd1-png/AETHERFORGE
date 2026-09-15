use forgehx_core::{PlaybackRejectionConfig, SpeakerLockConfig};
use std::collections::VecDeque;

pub const VOICEPRINT_LEN: usize = 12;
const FEATURE_BANDS: usize = 18;
const ROLLING_FRAMES: usize = 24;
const ENROLLMENT_TARGET_FRAMES: usize = 350;
const CONTINUOUS_LEARNING_RATE: f32 = 0.0025;
const CONTINUOUS_CHECKPOINT_FRAMES: usize = 500;
const CONTINUOUS_MATCH_MARGIN: f32 = 0.04;
const DOWNSAMPLED: usize = 120;

#[derive(Debug, Clone, Copy, Default)]
pub struct SpeakerDecision {
    pub enrolled: bool,
    pub enrollment_active: bool,
    pub enrollment_progress_percent: u8,
    pub score: f32,
    pub rejected: bool,
}

#[derive(Debug, Clone)]
struct EnrollmentState {
    sum: [f32; VOICEPRINT_LEN],
    frames: usize,
}

#[derive(Debug, Clone)]
pub struct SpeakerVerifier {
    enabled: bool,
    continuous_learning: bool,
    voiceprint: Vec<f32>,
    match_threshold: f32,
    min_speech_db: f32,
    rolling: VecDeque<[f32; VOICEPRINT_LEN]>,
    enrollment: Option<EnrollmentState>,
    completed_voiceprint: Option<Vec<f32>>,
    continuous_learning_checkpoint: usize,
}

impl SpeakerVerifier {
    pub fn new(config: &SpeakerLockConfig) -> Self {
        Self {
            enabled: config.enabled,
            continuous_learning: config.continuous_learning,
            voiceprint: config.voiceprint.clone(),
            match_threshold: config.match_threshold,
            min_speech_db: config.min_speech_db,
            rolling: VecDeque::with_capacity(ROLLING_FRAMES),
            enrollment: None,
            completed_voiceprint: None,
            continuous_learning_checkpoint: 0,
        }
    }

    pub fn update_config(&mut self, config: &SpeakerLockConfig) {
        self.enabled = config.enabled;
        self.continuous_learning = config.continuous_learning;
        self.match_threshold = config.match_threshold;
        self.min_speech_db = config.min_speech_db;
        if config.voiceprint != self.voiceprint && self.enrollment.is_none() {
            self.voiceprint = config.voiceprint.clone();
            self.rolling.clear();
        }
    }

    pub fn begin_enrollment(&mut self) {
        self.enrollment = Some(EnrollmentState {
            sum: [0.0; VOICEPRINT_LEN],
            frames: 0,
        });
        self.completed_voiceprint = None;
        self.continuous_learning_checkpoint = 0;
        self.rolling.clear();
    }

    pub fn cancel_enrollment(&mut self) {
        self.enrollment = None;
    }

    pub fn forget_voice(&mut self) {
        self.enrollment = None;
        self.completed_voiceprint = None;
        self.voiceprint.clear();
        self.continuous_learning_checkpoint = 0;
        self.rolling.clear();
    }

    pub fn take_completed_voiceprint(&mut self) -> Option<Vec<f32>> {
        self.completed_voiceprint.take()
    }

    pub fn evaluate(&mut self, frame: &[f32], learning_allowed: bool) -> SpeakerDecision {
        let level = rms_db(frame);
        let voiced = level >= self.min_speech_db;
        let features = voiced.then(|| voice_features(frame));

        // FORGEHX_CONTINUOUS_VOICE_LEARNING: normal microphone use is the training session.
        // No raw PCM is retained; only the normalized feature centroid/voiceprint is kept.
        if self.continuous_learning
            && self.voiceprint.is_empty()
            && self.enrollment.is_none()
            && voiced
            && learning_allowed
        {
            self.begin_enrollment();
        }

        if let (Some(state), Some(features)) = (&mut self.enrollment, features) {
            if learning_allowed {
                for (sum, value) in state.sum.iter_mut().zip(features) {
                    *sum += value;
                }
                state.frames += 1;
                if state.frames >= ENROLLMENT_TARGET_FRAMES {
                    let mut voiceprint =
                        state.sum.map(|value| value / state.frames as f32).to_vec();
                    normalize(&mut voiceprint);
                    self.voiceprint = voiceprint.clone();
                    self.completed_voiceprint = Some(voiceprint);
                    self.enrollment = None;
                    self.continuous_learning_checkpoint = 0;
                    self.rolling.clear();
                }
            }
        }

        if let Some(features) = features {
            if self.rolling.len() >= ROLLING_FRAMES {
                self.rolling.pop_front();
            }
            self.rolling.push_back(features);
        }

        let enrollment_active = self.enrollment.is_some();
        let progress = self
            .enrollment
            .as_ref()
            .map(|state| ((state.frames * 100 / ENROLLMENT_TARGET_FRAMES).min(100)) as u8)
            .unwrap_or(if self.voiceprint.is_empty() { 0 } else { 100 });
        let enrolled = self.voiceprint.len() == VOICEPRINT_LEN;

        if !enrolled || enrollment_active {
            return SpeakerDecision {
                enrolled,
                enrollment_active,
                enrollment_progress_percent: progress,
                score: 1.0,
                rejected: false,
            };
        }
        if !voiced {
            return SpeakerDecision {
                enrolled,
                enrollment_active,
                enrollment_progress_percent: progress,
                score: 0.0,
                rejected: self.enabled,
            };
        }

        let candidate = rolling_centroid(&self.rolling);
        let score = cosine(&candidate, &self.voiceprint)
            .clamp(-1.0, 1.0)
            .max(0.0);

        // Refine only from speech that already matches the enrolled speaker with extra margin.
        // Playback-correlated audio is passed in with learning_allowed=false by the engine.
        let refine_threshold = (self.match_threshold + CONTINUOUS_MATCH_MARGIN).min(0.98);
        if self.continuous_learning && learning_allowed && score >= refine_threshold {
            for (stored, observed) in self.voiceprint.iter_mut().zip(candidate.iter()) {
                *stored = *stored * (1.0 - CONTINUOUS_LEARNING_RATE)
                    + *observed * CONTINUOUS_LEARNING_RATE;
            }
            normalize(&mut self.voiceprint);
            self.continuous_learning_checkpoint =
                self.continuous_learning_checkpoint.saturating_add(1);
            if self.continuous_learning_checkpoint >= CONTINUOUS_CHECKPOINT_FRAMES {
                self.continuous_learning_checkpoint = 0;
                self.completed_voiceprint = Some(self.voiceprint.clone());
            }
        }

        SpeakerDecision {
            enrolled,
            enrollment_active,
            enrollment_progress_percent: progress,
            score,
            rejected: self.enabled && score < self.match_threshold,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackLeakGuard {
    enabled: bool,
    hard_block: bool,
    threshold: f32,
    history_frames: usize,
    history: VecDeque<[f32; DOWNSAMPLED]>,
}

impl PlaybackLeakGuard {
    pub fn new(config: &PlaybackRejectionConfig) -> Self {
        let mut value = Self {
            enabled: false,
            hard_block: true,
            threshold: 0.62,
            history_frames: 50,
            history: VecDeque::new(),
        };
        value.update_config(config);
        value
    }

    pub fn update_config(&mut self, config: &PlaybackRejectionConfig) {
        self.enabled = config.enabled;
        self.hard_block = config.hard_block;
        self.threshold = config.correlation_threshold;
        self.history_frames = (config.history_ms.max(2000) as usize / 10).clamp(10, 200);
        while self.history.len() > self.history_frames {
            self.history.pop_front();
        }
    }

    pub fn observe_render(&mut self, render: &[f32]) {
        // Keep a small in-memory playback fingerprint even when hard rejection is disabled.
        // Continuous voice learning uses it to avoid learning speaker/replay audio.
        if rms_db(render) < -72.0 {
            return;
        }
        if self.history.len() >= self.history_frames {
            self.history.pop_front();
        }
        self.history.push_back(downsample(render));
    }

    pub fn evaluate(&self, capture: &[f32]) -> (f32, bool) {
        if self.history.is_empty() || rms_db(capture) < -65.0 {
            return (0.0, false);
        }
        let candidate = downsample(capture);
        let mut score = 0.0f32;
        for reference in &self.history {
            score = score.max(max_shifted_correlation(&candidate, reference));
        }
        (
            score,
            self.enabled && self.hard_block && score >= self.threshold,
        )
    }
}

fn voice_features(frame: &[f32]) -> [f32; VOICEPRINT_LEN] {
    let min_hz = 120.0f32;
    let max_hz = 7600.0f32;
    let mut bands = [0.0f32; FEATURE_BANDS];
    for (index, band) in bands.iter_mut().enumerate() {
        let t = index as f32 / (FEATURE_BANDS - 1) as f32;
        let hz = min_hz * (max_hz / min_hz).powf(t);
        *band = goertzel_power(frame, hz).max(1.0e-12).ln();
    }
    let mean = bands.iter().sum::<f32>() / FEATURE_BANDS as f32;
    for band in &mut bands {
        *band -= mean;
    }

    let mut cep = [0.0f32; VOICEPRINT_LEN];
    for (k, out) in cep.iter_mut().enumerate() {
        let k = k + 1;
        for (n, value) in bands.iter().enumerate() {
            *out += *value
                * (std::f32::consts::PI * k as f32 * (n as f32 + 0.5) / FEATURE_BANDS as f32).cos();
        }
    }
    normalize_array(&mut cep);
    cep
}

fn goertzel_power(frame: &[f32], hz: f32) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * hz / 48_000.0;
    let coeff = 2.0 * omega.cos();
    let mut s1 = 0.0f32;
    let mut s2 = 0.0f32;
    for &sample in frame {
        let s0 = sample + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - coeff * s1 * s2) / frame.len().max(1) as f32
}

fn rolling_centroid(history: &VecDeque<[f32; VOICEPRINT_LEN]>) -> Vec<f32> {
    if history.is_empty() {
        return vec![0.0; VOICEPRINT_LEN];
    }
    let mut out = vec![0.0f32; VOICEPRINT_LEN];
    for frame in history {
        for (dst, value) in out.iter_mut().zip(frame) {
            *dst += *value;
        }
    }
    for value in &mut out {
        *value /= history.len() as f32;
    }
    normalize(&mut out);
    out
}

fn downsample(frame: &[f32]) -> [f32; DOWNSAMPLED] {
    let mut out = [0.0f32; DOWNSAMPLED];
    if frame.is_empty() {
        return out;
    }
    for (index, value) in out.iter_mut().enumerate() {
        let start = index * frame.len() / DOWNSAMPLED;
        let end = ((index + 1) * frame.len() / DOWNSAMPLED)
            .max(start + 1)
            .min(frame.len());
        *value = frame[start..end].iter().sum::<f32>() / (end - start) as f32;
    }
    let mean = out.iter().sum::<f32>() / DOWNSAMPLED as f32;
    for value in &mut out {
        *value -= mean;
    }
    out
}

fn max_shifted_correlation(a: &[f32; DOWNSAMPLED], b: &[f32; DOWNSAMPLED]) -> f32 {
    let mut best = 0.0f32;
    for shift in (-48i32..=48).step_by(4) {
        let mut dot = 0.0f32;
        let mut aa = 0.0f32;
        let mut bb = 0.0f32;
        for (i, &x) in a.iter().enumerate() {
            let j = i as i32 + shift;
            if !(0..DOWNSAMPLED as i32).contains(&j) {
                continue;
            }
            let y = b[j as usize];
            dot += x * y;
            aa += x * x;
            bb += y * y;
        }
        if aa > 1.0e-9 && bb > 1.0e-9 {
            best = best.max((dot / (aa * bb).sqrt()).abs());
        }
    }
    best.clamp(0.0, 1.0)
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut aa = 0.0f32;
    let mut bb = 0.0f32;
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        aa += x * x;
        bb += y * y;
    }
    if aa <= 1.0e-9 || bb <= 1.0e-9 {
        0.0
    } else {
        dot / (aa * bb).sqrt()
    }
}

fn normalize(values: &mut [f32]) {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 1.0e-9 {
        for value in values {
            *value /= norm;
        }
    }
}

fn normalize_array(values: &mut [f32; VOICEPRINT_LEN]) {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 1.0e-9 {
        for value in values {
            *value /= norm;
        }
    }
}

fn rms_db(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return -90.0;
    }
    let power = frame.iter().map(|sample| sample * sample).sum::<f32>() / frame.len() as f32;
    20.0 * power.sqrt().max(1.0e-6).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_guard_rejects_matching_render_audio() {
        let cfg = PlaybackRejectionConfig {
            correlation_threshold: 0.55,
            ..Default::default()
        };
        let mut guard = PlaybackLeakGuard::new(&cfg);
        let frame = (0..480)
            .map(|i| ((i as f32) * 0.071).sin() * 0.2)
            .collect::<Vec<_>>();
        guard.observe_render(&frame);
        let (score, rejected) = guard.evaluate(&frame);
        assert!(score > 0.9);
        assert!(rejected);
    }

    #[test]
    fn speaker_enrollment_produces_twelve_value_voiceprint() {
        let mut verifier = SpeakerVerifier::new(&SpeakerLockConfig::default());
        verifier.begin_enrollment();
        let frame = (0..480)
            .map(|i| (((i as f32) * 0.038).sin() + 0.4 * ((i as f32) * 0.083).sin()) * 0.2)
            .collect::<Vec<_>>();
        for _ in 0..ENROLLMENT_TARGET_FRAMES {
            verifier.evaluate(&frame, true);
        }
        assert_eq!(
            verifier.take_completed_voiceprint().map(|v| v.len()),
            Some(VOICEPRINT_LEN)
        );
    }

    #[test]
    fn continuous_learning_starts_enrollment_without_record_button() {
        let mut verifier = SpeakerVerifier::new(&SpeakerLockConfig::default());
        let frame = (0..480)
            .map(|i| (((i as f32) * 0.038).sin() + 0.4 * ((i as f32) * 0.083).sin()) * 0.2)
            .collect::<Vec<_>>();
        let first = verifier.evaluate(&frame, true);
        assert!(first.enrollment_active);
        for _ in 1..ENROLLMENT_TARGET_FRAMES {
            verifier.evaluate(&frame, true);
        }
        assert_eq!(
            verifier.take_completed_voiceprint().map(|v| v.len()),
            Some(VOICEPRINT_LEN)
        );
    }

    #[test]
    fn continuous_learning_refines_high_confidence_voiceprint() {
        let frame = (0..480)
            .map(|i| (((i as f32) * 0.038).sin() + 0.4 * ((i as f32) * 0.083).sin()) * 0.2)
            .collect::<Vec<_>>();
        let mut seed = SpeakerVerifier::new(&SpeakerLockConfig::default());
        for _ in 0..ENROLLMENT_TARGET_FRAMES {
            seed.evaluate(&frame, true);
        }
        let voiceprint = seed.take_completed_voiceprint().unwrap();
        let mut cfg = SpeakerLockConfig {
            voiceprint,
            ..Default::default()
        };
        cfg.match_threshold = 0.80;
        let mut verifier = SpeakerVerifier::new(&cfg);
        for _ in 0..CONTINUOUS_CHECKPOINT_FRAMES {
            verifier.evaluate(&frame, true);
        }
        assert_eq!(
            verifier.take_completed_voiceprint().map(|v| v.len()),
            Some(VOICEPRINT_LEN)
        );
    }
}
