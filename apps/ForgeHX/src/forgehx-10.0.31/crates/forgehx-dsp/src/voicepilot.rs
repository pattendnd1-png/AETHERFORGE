use forgehx_core::{MicrophoneDspConfig, VoicePilotMode, VoicePilotTarget, VoicePilotTelemetry};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveTargets {
    pub bass_boost_db: f32,
    pub treble_boost_db: f32,
    pub presence_db: f32,
    pub noise_strength_percent: f32,
    pub de_esser_reduction_db: f32,
    pub compressor_reduction_db: f32,
    pub output_trim_db: f32,
}

impl Default for AdaptiveTargets {
    fn default() -> Self {
        Self {
            bass_boost_db: 0.0,
            treble_boost_db: 0.0,
            presence_db: 0.0,
            noise_strength_percent: 45.0,
            de_esser_reduction_db: 0.0,
            compressor_reduction_db: 0.0,
            output_trim_db: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VoicePilotAnalyzer {
    targets: AdaptiveTargets,
    noise_floor_db: f32,
    telemetry: VoicePilotTelemetry,
}

impl Default for VoicePilotAnalyzer {
    fn default() -> Self {
        Self {
            targets: AdaptiveTargets::default(),
            noise_floor_db: -72.0,
            telemetry: VoicePilotTelemetry::default(),
        }
    }
}

impl VoicePilotAnalyzer {
    pub fn analyze(
        &mut self,
        frame: &[f32],
        config: &MicrophoneDspConfig,
        reference_available: bool,
    ) -> AdaptiveTargets {
        let input_rms_db = rms_db(frame);
        let (low_db, mid_db, high_db) = spectral_proxy(frame);

        if input_rms_db < -42.0 {
            self.noise_floor_db = lerp(self.noise_floor_db, input_rms_db, 0.025);
        }

        let (base_bass, base_treble, base_presence) = target_bias(config.voicepilot.target);
        let adapt = (config.voicepilot.adaptation_strength / 100.0).clamp(0.0, 1.0);
        let smoothing = 0.025 + 0.16 * adapt;

        let desired_bass =
            (base_bass + ((mid_db - low_db - 3.0) * 0.12).clamp(-1.0, 3.0)).clamp(0.0, 6.0);
        let desired_treble =
            (base_treble + ((mid_db - high_db - 6.0) * 0.10).clamp(-0.8, 2.5)).clamp(0.0, 5.0);
        let desired_presence =
            (base_presence + ((low_db - mid_db) * 0.08).clamp(-1.5, 1.5)).clamp(-2.0, 4.0);
        let desired_noise = ((self.noise_floor_db + 72.0) * 2.0 + 30.0).clamp(20.0, 85.0);
        let desired_deesser = ((high_db - mid_db + 8.0) * 0.55).clamp(0.0, 8.0);
        let desired_comp = ((input_rms_db + 18.0) * 0.45).clamp(0.0, 10.0);
        let desired_output = (-16.0 - input_rms_db).clamp(-3.0, 6.0) * 0.25;

        self.targets.bass_boost_db = automate(
            config.voicepilot.auto_tone,
            self.targets.bass_boost_db,
            desired_bass,
            config.tone.bass_boost_db,
            smoothing,
        );
        self.targets.treble_boost_db = automate(
            config.voicepilot.auto_tone,
            self.targets.treble_boost_db,
            desired_treble,
            config.tone.treble_boost_db,
            smoothing,
        );
        self.targets.presence_db = automate(
            config.voicepilot.auto_eq,
            self.targets.presence_db,
            desired_presence,
            0.0,
            smoothing,
        );
        self.targets.noise_strength_percent = automate(
            config.voicepilot.auto_noise,
            self.targets.noise_strength_percent,
            desired_noise,
            config.noise_suppression.strength_percent,
            smoothing,
        );
        self.targets.de_esser_reduction_db = automate(
            config.voicepilot.auto_de_esser,
            self.targets.de_esser_reduction_db,
            desired_deesser,
            0.0,
            smoothing,
        );
        self.targets.compressor_reduction_db = automate(
            config.voicepilot.auto_dynamics,
            self.targets.compressor_reduction_db,
            desired_comp,
            0.0,
            smoothing,
        );
        self.targets.output_trim_db = automate(
            config.voicepilot.auto_loudness,
            self.targets.output_trim_db,
            desired_output,
            0.0,
            smoothing,
        );

        self.telemetry = VoicePilotTelemetry {
            active: config.voicepilot.enabled,
            reference_available,
            input_rms_db,
            output_rms_db: input_rms_db + self.targets.output_trim_db,
            noise_floor_db: self.noise_floor_db,
            bass_target_db: self.targets.bass_boost_db,
            treble_target_db: self.targets.treble_boost_db,
            presence_target_db: self.targets.presence_db,
            de_esser_reduction_db: self.targets.de_esser_reduction_db,
            compressor_reduction_db: self.targets.compressor_reduction_db,
        };
        self.targets
    }

    pub fn telemetry(&self) -> VoicePilotTelemetry {
        self.telemetry.clone()
    }
}

fn automate(mode: VoicePilotMode, current: f32, desired: f32, manual: f32, smoothing: f32) -> f32 {
    match mode {
        VoicePilotMode::Auto => lerp(current, desired, smoothing),
        VoicePilotMode::Manual | VoicePilotMode::Locked => manual,
        VoicePilotMode::Bypassed => 0.0,
    }
}

fn target_bias(target: VoicePilotTarget) -> (f32, f32, f32) {
    match target {
        VoicePilotTarget::Natural => (0.5, 0.5, 0.3),
        VoicePilotTarget::Clear => (0.3, 1.8, 2.2),
        VoicePilotTarget::Warm => (2.4, 0.4, 0.4),
        VoicePilotTarget::Full => (2.8, 0.8, 0.8),
        VoicePilotTarget::Broadcast => (1.5, 1.2, 1.6),
        VoicePilotTarget::BroadcastFull => (2.4, 1.3, 1.5),
        VoicePilotTarget::DeepAndFull => (3.6, 0.6, 0.4),
        VoicePilotTarget::CrispStream => (0.8, 2.2, 2.2),
        VoicePilotTarget::VoiceChat => (0.4, 1.0, 2.4),
        VoicePilotTarget::Custom => (1.0, 1.0, 1.0),
    }
}

fn rms_db(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return -90.0;
    }
    let power = frame.iter().map(|v| v * v).sum::<f32>() / frame.len() as f32;
    20.0 * power.sqrt().max(1.0e-5).log10()
}

fn spectral_proxy(frame: &[f32]) -> (f32, f32, f32) {
    let mut slow = 0.0f32;
    let mut fast = 0.0f32;
    let mut low_power = 0.0f32;
    let mut mid_power = 0.0f32;
    let mut high_power = 0.0f32;
    for &sample in frame {
        slow += 0.035 * (sample - slow);
        fast += 0.22 * (sample - fast);
        let low = slow;
        let mid = fast - slow;
        let high = sample - fast;
        low_power += low * low;
        mid_power += mid * mid;
        high_power += high * high;
    }
    let n = frame.len().max(1) as f32;
    let db = |p: f32| 10.0 * (p / n).max(1.0e-10).log10();
    (db(low_power), db(mid_power), db(high_power))
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_targets_stay_bounded() {
        let mut analyzer = VoicePilotAnalyzer::default();
        let cfg = MicrophoneDspConfig::default();
        let hot = [0.95f32; 480];
        for _ in 0..100 {
            analyzer.analyze(&hot, &cfg, true);
        }
        let t = analyzer.targets;
        assert!((0.0..=6.0).contains(&t.bass_boost_db));
        assert!((0.0..=5.0).contains(&t.treble_boost_db));
        assert!((-2.0..=4.0).contains(&t.presence_db));
        assert!((20.0..=85.0).contains(&t.noise_strength_percent));
    }
}
