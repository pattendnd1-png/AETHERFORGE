use forgehx_core::NoiseSuppressionConfig;

const FRAME_MS: u32 = 10;
const INITIAL_NOISE_FLOOR_DB: f32 = -72.0;
const MIN_NOISE_FLOOR_DB: f32 = -90.0;
const MAX_NOISE_FLOOR_DB: f32 = -12.0;
const LOUD_BACKGROUND_LEARN_LIMIT_DB: f32 = -16.0;
const STABLE_LEVEL_DELTA_DB: f32 = 1.5;
const STABLE_LEARN_FRAMES: usize = 25;
const MAX_REJECTION_DB: f32 = -72.0;

#[derive(Debug, Clone)]
pub struct BackgroundRejector {
    noise_floor_db: f32,
    hangover_frames: usize,
    current_gain: f32,
    initialized: bool,
    last_level_db: f32,
    stable_frames: usize,
}

impl Default for BackgroundRejector {
    fn default() -> Self {
        Self {
            noise_floor_db: INITIAL_NOISE_FLOOR_DB,
            hangover_frames: 0,
            current_gain: 1.0,
            initialized: false,
            last_level_db: INITIAL_NOISE_FLOOR_DB,
            stable_frames: 0,
        }
    }
}

impl BackgroundRejector {
    pub fn process(&mut self, frame: &mut [f32], config: &NoiseSuppressionConfig) {
        self.process_adaptive(frame, config, 0.0, 72.0);
    }

    pub fn process_adaptive(
        &mut self,
        frame: &mut [f32],
        config: &NoiseSuppressionConfig,
        margin_offset_db: f32,
        rejection_cap_db: f32,
    ) {
        if !config.enabled || frame.is_empty() {
            self.current_gain = 1.0;
            self.hangover_frames = 0;
            self.stable_frames = 0;
            return;
        }

        let level_db = rms_db(frame);
        self.update_noise_floor(level_db);

        let margin_db = decision_margin_db(config.strength_percent, config.vad_threshold_percent);
        let open_limit_db = voice_open_limit_db(config.strength_percent);
        let open_threshold_db =
            (self.noise_floor_db + margin_db + margin_offset_db.clamp(0.0, 8.0)).min(open_limit_db);
        // The configured grace window already supplies speech-release hysteresis. Reusing a
        // lower close threshold here can latch a loud steady fan as speech forever after the
        // floor learner catches up, so every new frame must still clear the true open threshold.
        let speech_detected = level_db >= open_threshold_db;

        if speech_detected {
            self.hangover_frames = grace_to_frames(config.grace_ms);
        } else if self.hangover_frames > 0 {
            self.hangover_frames -= 1;
        }

        let pass_voice = speech_detected || self.hangover_frames > 0;
        let target_gain = if pass_voice {
            1.0
        } else {
            let attenuation_db = (-rejection_gain_db(config.strength_percent))
                .min(rejection_cap_db.clamp(18.0, 72.0));
            db_to_gain(-attenuation_db)
        };

        // Speech opens nearly immediately. Rejection closes decisively enough that a learned
        // fan/HVAC/room floor does not hover audibly between phrases, while still avoiding a
        // discontinuity at the exact VAD boundary.
        let smoothing = if target_gain > self.current_gain {
            0.92
        } else {
            0.82
        };
        self.current_gain += (target_gain - self.current_gain) * smoothing;
        for sample in frame {
            *sample *= self.current_gain;
        }
    }

    fn update_noise_floor(&mut self, level_db: f32) {
        let bounded = level_db.clamp(MIN_NOISE_FLOOR_DB, MAX_NOISE_FLOOR_DB);
        if !self.initialized {
            // Never let a user speaking on the first frame become the learned room floor.
            self.noise_floor_db = bounded.min(-45.0);
            self.last_level_db = bounded;
            self.initialized = true;
            return;
        }

        if (bounded - self.last_level_db).abs() <= STABLE_LEVEL_DELTA_DB {
            self.stable_frames = self.stable_frames.saturating_add(1);
        } else {
            self.stable_frames = 0;
        }
        self.last_level_db = bounded;

        let alpha = if bounded <= self.noise_floor_db {
            // Follow a falling room floor quickly.
            0.18
        } else if self.stable_frames >= STABLE_LEARN_FRAMES
            && bounded < LOUD_BACKGROUND_LEARN_LIMIT_DB
        {
            // A real room can have a fan/HVAC/computer floor louder than -28 dBFS. The old
            // learner refused to follow it, so it was permanently mistaken for speech. Only
            // stable energy is allowed to raise the floor; normal speech dynamics continually
            // reset stable_frames and therefore are not learned as background.
            0.035
        } else {
            0.0
        };

        if alpha > 0.0 {
            self.noise_floor_db += (bounded - self.noise_floor_db) * alpha;
            self.noise_floor_db = self
                .noise_floor_db
                .clamp(MIN_NOISE_FLOOR_DB, MAX_NOISE_FLOOR_DB);
        }
    }
}

fn grace_to_frames(grace_ms: u32) -> usize {
    grace_ms.div_ceil(FRAME_MS) as usize
}

fn decision_margin_db(strength_percent: f32, vad_threshold_percent: f32) -> f32 {
    let strength = (strength_percent / 100.0).clamp(0.0, 1.0);
    let vad = (vad_threshold_percent / 100.0).clamp(0.0, 1.0);
    4.0 + 10.0 * strength + 6.0 * vad
}

fn voice_open_limit_db(strength_percent: f32) -> f32 {
    let strength = (strength_percent / 100.0).clamp(0.0, 1.0);
    -36.0 + 16.0 * strength
}

fn rejection_gain_db(strength_percent: f32) -> f32 {
    let strength = (strength_percent / 100.0).clamp(0.0, 1.0);
    (-18.0 - 56.0 * strength).clamp(MAX_REJECTION_DB, -18.0)
}

fn rms_db(frame: &[f32]) -> f32 {
    let power = frame.iter().map(|sample| sample * sample).sum::<f32>() / frame.len() as f32;
    20.0 * power.sqrt().max(1.0e-5).log10()
}

fn db_to_gain(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_strength_rejects_steady_background() {
        let mut rejector = BackgroundRejector::default();
        let config = NoiseSuppressionConfig::default();
        let mut last = [0.02f32; 480];
        for _ in 0..100 {
            let mut frame = [0.02f32; 480];
            rejector.process(&mut frame, &config);
            last = frame;
        }
        let peak = last
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak < 0.0001, "steady background peak was {peak}");
    }

    #[test]
    fn loud_steady_background_is_learned_and_rejected() {
        let mut rejector = BackgroundRejector::default();
        let config = NoiseSuppressionConfig::default();
        // About -24 dBFS: deliberately louder than the old -28 dB learner ceiling.
        let mut last = [0.063f32; 480];
        for _ in 0..180 {
            let mut frame = [0.063f32; 480];
            rejector.process(&mut frame, &config);
            last = frame;
        }
        let peak = last
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(
            peak < 0.0002,
            "loud stable room floor was not rejected: {peak}"
        );
    }

    #[test]
    fn varying_voice_is_not_learned_as_background() {
        let mut rejector = BackgroundRejector::default();
        let config = NoiseSuppressionConfig::default();
        for _ in 0..160 {
            let mut background = [0.025f32; 480];
            rejector.process(&mut background, &config);
        }

        let mut passed_peak = 0.0f32;
        for index in 0..80 {
            let amplitude = if index % 3 == 0 {
                0.16
            } else if index % 3 == 1 {
                0.09
            } else {
                0.12
            };
            let mut voice = [amplitude; 480];
            rejector.process(&mut voice, &config);
            passed_peak = passed_peak.max(voice[0].abs());
        }
        assert!(
            passed_peak > 0.07,
            "varying speech was learned as background: {passed_peak}"
        );
    }

    #[test]
    fn speech_hangover_preserves_short_gap() {
        let mut rejector = BackgroundRejector::default();
        let config = NoiseSuppressionConfig::default();
        for _ in 0..100 {
            let mut background = [0.005f32; 480];
            rejector.process(&mut background, &config);
        }

        let mut speech = [0.08f32; 480];
        rejector.process(&mut speech, &config);
        assert!(speech[0] > 0.04);

        let mut short_gap = [0.005f32; 480];
        for _ in 0..5 {
            short_gap.fill(0.005);
            rejector.process(&mut short_gap, &config);
        }
        assert!(
            short_gap[0] > 0.004,
            "hangover attenuated a short speech gap: {}",
            short_gap[0]
        );
    }
}
