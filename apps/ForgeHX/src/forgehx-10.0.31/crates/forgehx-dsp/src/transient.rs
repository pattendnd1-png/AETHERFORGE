use forgehx_core::ClickSuppressionConfig;

const SAMPLE_RATE: f32 = 48_000.0;
const EPSILON: f32 = 1.0e-9;

/// Stateful acoustic de-clicker for short keyboard and mouse transients.
///
/// This intentionally does not inspect keyboard/mouse input events or key codes. It detects
/// impulsive microphone energy from the audio itself and attenuates only the short transient
/// window plus a bounded recovery tail, so normal sustained speech remains intact.
#[derive(Debug, Clone)]
pub struct TransientSuppressor {
    config: ClickSuppressionConfig,
    hard_remaining: usize,
    recovery_remaining: usize,
    recovery_total: usize,
}

impl TransientSuppressor {
    pub fn new(config: &ClickSuppressionConfig) -> Self {
        Self {
            config: config.clone(),
            hard_remaining: 0,
            recovery_remaining: 0,
            recovery_total: samples_from_ms(config.recovery_ms),
        }
    }

    pub fn update_config(&mut self, config: &ClickSuppressionConfig) {
        self.config = config.clone();
        self.recovery_total = samples_from_ms(config.recovery_ms);
        if !config.enabled {
            self.hard_remaining = 0;
            self.recovery_remaining = 0;
        }
    }

    pub fn process(&mut self, frame: &mut [f32]) -> bool {
        if !self.config.enabled || frame.is_empty() {
            self.hard_remaining = 0;
            self.recovery_remaining = 0;
            return false;
        }

        // Analyze the unmodified frame so an existing recovery tail cannot hide a new click.
        let detection = detect_transient(frame, self.config.sensitivity_percent);
        self.apply_pending_tail(frame);

        let Some(peak_index) = detection else {
            return false;
        };
        let hard_gain = db_reduction_to_gain(self.config.suppression_db);
        let max_click = samples_from_ms(self.config.max_click_ms).max(1);
        let pre_roll = samples_from_ms(0.75).min(peak_index);
        let start = peak_index.saturating_sub(pre_roll);
        let end = start.saturating_add(max_click);

        let local_end = end.min(frame.len());
        for sample in &mut frame[start..local_end] {
            *sample *= hard_gain;
        }

        if end > frame.len() {
            self.hard_remaining = self.hard_remaining.max(end - frame.len());
            self.recovery_remaining = self.recovery_remaining.max(self.recovery_total);
            return true;
        }

        self.apply_recovery_from(frame, end, hard_gain);
        true
    }

    fn apply_pending_tail(&mut self, frame: &mut [f32]) {
        let mut offset = 0usize;
        if self.hard_remaining > 0 {
            let gain = db_reduction_to_gain(self.config.suppression_db);
            let count = self.hard_remaining.min(frame.len());
            for sample in &mut frame[..count] {
                *sample *= gain;
            }
            self.hard_remaining -= count;
            offset = count;
            if count == frame.len() {
                return;
            }
        }

        if self.recovery_remaining == 0 || self.recovery_total == 0 {
            return;
        }
        let hard_gain = db_reduction_to_gain(self.config.suppression_db);
        for sample in frame.iter_mut().skip(offset) {
            if self.recovery_remaining == 0 {
                break;
            }
            let progressed = self.recovery_total.saturating_sub(self.recovery_remaining) as f32;
            let t = (progressed / self.recovery_total.max(1) as f32).clamp(0.0, 1.0);
            let gain = hard_gain + (1.0 - hard_gain) * smoothstep(t);
            *sample *= gain;
            self.recovery_remaining -= 1;
        }
    }

    fn apply_recovery_from(&mut self, frame: &mut [f32], start: usize, hard_gain: f32) {
        if self.recovery_total == 0 || start >= frame.len() {
            self.recovery_remaining = self.recovery_total;
            return;
        }
        let available = frame.len() - start;
        let local = available.min(self.recovery_total);
        for i in 0..local {
            let t = i as f32 / self.recovery_total.max(1) as f32;
            let gain = hard_gain + (1.0 - hard_gain) * smoothstep(t);
            frame[start + i] *= gain;
        }
        self.recovery_remaining = self.recovery_total.saturating_sub(local);
    }
}

fn detect_transient(frame: &[f32], sensitivity_percent: f32) -> Option<usize> {
    let mut energy = 0.0f32;
    let mut diff_energy = 0.0f32;
    let mut peak = 0.0f32;
    let mut peak_index = 0usize;
    let mut previous = frame[0];

    for (index, &sample) in frame.iter().enumerate() {
        let abs = sample.abs();
        energy += sample * sample;
        if abs > peak {
            peak = abs;
            peak_index = index;
        }
        if index > 0 {
            let delta = sample - previous;
            diff_energy += delta * delta;
        }
        previous = sample;
    }

    let rms = (energy / frame.len() as f32).sqrt();
    if rms <= EPSILON || peak <= EPSILON {
        return None;
    }
    let diff_rms = (diff_energy / frame.len().max(2) as f32).sqrt();
    let crest = peak / (rms + EPSILON);
    let high_frequency_ratio = diff_rms / (rms + EPSILON);

    let sensitivity = (sensitivity_percent / 100.0).clamp(0.0, 1.0);
    // Higher sensitivity lowers all three thresholds. The combination of crest factor and
    // derivative energy rejects sustained voiced vowels and most consonants while still
    // catching the short broadband onset typical of mechanical keys and mouse switches.
    let crest_threshold = lerp(5.5, 2.75, sensitivity);
    let hf_threshold = lerp(0.58, 0.20, sensitivity);
    let min_peak_db = lerp(-28.0, -50.0, sensitivity);
    let min_peak = db_to_gain(min_peak_db);

    let broadband_impulse =
        peak >= min_peak && crest >= crest_threshold && high_frequency_ratio >= hf_threshold;
    let extreme_impulse = peak >= min_peak
        && crest >= crest_threshold * 1.45
        && high_frequency_ratio >= hf_threshold * 0.65;

    (broadband_impulse || extreme_impulse).then_some(peak_index)
}

fn samples_from_ms(ms: f32) -> usize {
    (ms.max(0.0) * SAMPLE_RATE / 1000.0).round() as usize
}

fn db_to_gain(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}
fn db_reduction_to_gain(reduction_db: f32) -> f32 {
    db_to_gain(-reduction_db.clamp(0.0, 72.0))
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn isolated_impulse_is_strongly_reduced() {
        let mut processor = TransientSuppressor::new(&ClickSuppressionConfig::default());
        let mut frame = [0.0f32; 480];
        frame[235] = 0.10;
        frame[236] = -0.80;
        frame[237] = 0.55;
        frame[238] = -0.22;
        assert!(processor.process(&mut frame));
        let peak = frame.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak < 0.01, "click peak remained audible: {peak}");
    }

    #[test]
    fn damped_switch_click_is_reduced() {
        let mut processor = TransientSuppressor::new(&ClickSuppressionConfig::default());
        let mut frame = [0.0f32; 480];
        for n in 0..144usize {
            let envelope = (-(n as f32) / 35.0).exp();
            frame[180 + n] = 0.35 * envelope * (2.0 * PI * 4200.0 * n as f32 / SAMPLE_RATE).sin();
        }
        let before = frame.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(processor.process(&mut frame));
        let after = frame.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(
            after < before * 0.05,
            "switch click was not strongly reduced: {before} -> {after}"
        );
    }

    #[test]
    fn steady_voice_like_wave_is_preserved() {
        let mut processor = TransientSuppressor::new(&ClickSuppressionConfig::default());
        let mut frame = [0.0f32; 480];
        let original = std::array::from_fn::<_, 480, _>(|i| {
            0.12 * (2.0 * PI * 220.0 * i as f32 / SAMPLE_RATE).sin()
        });
        frame.copy_from_slice(&original);
        assert!(!processor.process(&mut frame));
        let error: f32 = frame.iter().zip(original).map(|(a, b)| (a - b).abs()).sum();
        assert!(
            error < 1.0e-6,
            "steady voice-like waveform was modified: {error}"
        );
    }
}
