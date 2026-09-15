use forgehx_core::{MicrophoneDspConfig, NoiseSceneTelemetry, VoicePilotMode};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveNoiseTargets {
    pub sonora_noise_percent: f32,
    pub background_margin_offset_db: f32,
    pub rejection_cap_db: f32,
    pub residual_speaker_suppression_db: f32,
}

impl Default for AdaptiveNoiseTargets {
    fn default() -> Self {
        Self {
            sonora_noise_percent: 0.0,
            background_margin_offset_db: 0.0,
            rejection_cap_db: 72.0,
            residual_speaker_suppression_db: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdaptiveNoiseController {
    current: AdaptiveNoiseTargets,
    initialized: bool,
}

impl Default for AdaptiveNoiseController {
    fn default() -> Self {
        Self {
            current: AdaptiveNoiseTargets::default(),
            initialized: false,
        }
    }
}

impl AdaptiveNoiseController {
    pub fn update(
        &mut self,
        telemetry: &NoiseSceneTelemetry,
        config: &MicrophoneDspConfig,
    ) -> AdaptiveNoiseTargets {
        let base_noise = config.noise_suppression.strength_percent.clamp(0.0, 100.0);
        let max_rejection = config
            .extraneous_noise_monitor
            .max_adaptive_suppression_db
            .clamp(18.0, 72.0);
        let neutral = AdaptiveNoiseTargets {
            sonora_noise_percent: base_noise,
            background_margin_offset_db: 0.0,
            rejection_cap_db: max_rejection,
            residual_speaker_suppression_db: 0.0,
        };

        if !config.extraneous_noise_monitor.enabled
            || !config.extraneous_noise_monitor.auto_adapt
            || !telemetry.active
        {
            self.current = neutral;
            self.initialized = true;
            return self.current;
        }

        let noise_auto = config.voicepilot.enabled
            && matches!(config.voicepilot.auto_noise, VoicePilotMode::Auto);
        let aec_auto =
            config.voicepilot.enabled && matches!(config.voicepilot.auto_aec, VoicePilotMode::Auto);
        let sensitivity =
            (config.extraneous_noise_monitor.sensitivity_percent / 100.0).clamp(0.0, 1.0);
        let broad = telemetry
            .classes
            .white_like
            .max(telemetry.classes.pink_like)
            .max(telemetry.classes.broadband)
            * sensitivity;
        let hum = telemetry.classes.hum_rumble * sensitivity;
        let whine = telemetry.classes.narrowband_whine * sensitivity;
        let persistent_scene = broad.max(hum * 0.65).max(whine * 0.55).clamp(0.0, 1.0);
        let persistent_scene = if telemetry.classes.transient >= 0.60 {
            0.0
        } else {
            persistent_scene
        };

        let mut desired = neutral;
        if noise_auto {
            desired.sonora_noise_percent =
                (base_noise + broad * 24.0 + hum * 8.0 + whine * 6.0).clamp(base_noise, 100.0);
            desired.background_margin_offset_db =
                (broad * 8.0 + hum * 2.0 + whine * 1.5).clamp(0.0, 8.0);
            let base_cap = 18.0 + 54.0 * (base_noise / 100.0);
            let adaptive_cap = 18.0 + 54.0 * persistent_scene;
            desired.rejection_cap_db = base_cap
                .max(adaptive_cap)
                .min(max_rejection)
                .clamp(18.0, 72.0);
        }

        if config.extraneous_noise_monitor.speaker_rejection_enabled
            && telemetry.reference_available
            && aec_auto
        {
            desired.residual_speaker_suppression_db =
                (telemetry.classes.speaker_leak * 48.0).clamp(0.0, 48.0);
            if telemetry.double_talk || telemetry.speech_protected {
                // Voice-only mode removes correlated playback with the aligned reference.
                // Never blanket-attenuate the user's local voice during double-talk.
                desired.residual_speaker_suppression_db = 0.0;
            }
        }

        if !self.initialized {
            self.current = neutral;
            self.initialized = true;
        }
        self.current = rate_limit(self.current, desired);
        self.current
    }
}

fn rate_limit(
    current: AdaptiveNoiseTargets,
    desired: AdaptiveNoiseTargets,
) -> AdaptiveNoiseTargets {
    AdaptiveNoiseTargets {
        sonora_noise_percent: approach(
            current.sonora_noise_percent,
            desired.sonora_noise_percent,
            2.0,
        ),
        background_margin_offset_db: approach(
            current.background_margin_offset_db,
            desired.background_margin_offset_db,
            1.5,
        ),
        rejection_cap_db: approach(current.rejection_cap_db, desired.rejection_cap_db, 1.5),
        residual_speaker_suppression_db: approach(
            current.residual_speaker_suppression_db,
            desired.residual_speaker_suppression_db,
            1.5,
        ),
    }
}

fn approach(current: f32, desired: f32, max_step: f32) -> f32 {
    current + (desired - current).clamp(-max_step, max_step)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::{NoiseAdaptationState, NoiseClassScores, NoiseSceneTelemetry};

    fn active_scene() -> NoiseSceneTelemetry {
        NoiseSceneTelemetry {
            active: true,
            reference_available: true,
            adaptation_state: NoiseAdaptationState::Suppressing,
            classes: NoiseClassScores {
                white_like: 0.85,
                pink_like: 0.20,
                broadband: 0.75,
                hum_rumble: 0.10,
                narrowband_whine: 0.05,
                transient: 0.0,
                speaker_leak: 0.80,
            },
            ..Default::default()
        }
    }

    fn settle(
        controller: &mut AdaptiveNoiseController,
        scene: &NoiseSceneTelemetry,
        cfg: &MicrophoneDspConfig,
    ) -> AdaptiveNoiseTargets {
        let mut targets = AdaptiveNoiseTargets::default();
        for _ in 0..80 {
            targets = controller.update(scene, cfg);
        }
        targets
    }

    #[test]
    fn auto_noise_can_raise_cleanup_but_never_past_user_and_product_bounds() {
        let mut controller = AdaptiveNoiseController::default();
        let mut cfg = MicrophoneDspConfig::default();
        cfg.noise_suppression.strength_percent = 60.0;
        cfg.extraneous_noise_monitor.max_adaptive_suppression_db = 64.0;
        let targets = settle(&mut controller, &active_scene(), &cfg);
        assert!(targets.sonora_noise_percent >= 60.0 && targets.sonora_noise_percent <= 100.0);
        assert!(targets.rejection_cap_db <= 64.0);
    }

    #[test]
    fn manual_noise_domain_returns_user_noise_strength_unchanged() {
        let mut controller = AdaptiveNoiseController::default();
        let mut cfg = MicrophoneDspConfig::default();
        cfg.voicepilot.auto_noise = VoicePilotMode::Manual;
        cfg.noise_suppression.strength_percent = 73.0;
        let targets = settle(&mut controller, &active_scene(), &cfg);
        assert_eq!(targets.sonora_noise_percent, 73.0);
        assert_eq!(targets.background_margin_offset_db, 0.0);
    }

    #[test]
    fn locked_noise_domain_returns_user_noise_strength_unchanged() {
        let mut controller = AdaptiveNoiseController::default();
        let mut cfg = MicrophoneDspConfig::default();
        cfg.voicepilot.auto_noise = VoicePilotMode::Locked;
        cfg.noise_suppression.strength_percent = 81.0;
        let targets = settle(&mut controller, &active_scene(), &cfg);
        assert_eq!(targets.sonora_noise_percent, 81.0);
        assert_eq!(targets.background_margin_offset_db, 0.0);
    }

    #[test]
    fn bypassed_noise_domain_never_enables_adaptive_noise_suppression() {
        let mut controller = AdaptiveNoiseController::default();
        let mut cfg = MicrophoneDspConfig::default();
        cfg.voicepilot.auto_noise = VoicePilotMode::Bypassed;
        let targets = settle(&mut controller, &active_scene(), &cfg);
        assert_eq!(
            targets.sonora_noise_percent,
            cfg.noise_suppression.strength_percent
        );
        assert_eq!(targets.background_margin_offset_db, 0.0);
    }

    #[test]
    fn auto_adapt_off_observes_but_returns_neutral_targets() {
        let mut controller = AdaptiveNoiseController::default();
        let mut cfg = MicrophoneDspConfig::default();
        cfg.extraneous_noise_monitor.auto_adapt = false;
        let targets = controller.update(&active_scene(), &cfg);
        assert_eq!(
            targets.sonora_noise_percent,
            cfg.noise_suppression.strength_percent
        );
        assert_eq!(targets.residual_speaker_suppression_db, 0.0);
    }

    #[test]
    fn double_talk_does_not_blanket_attenuate_local_voice() {
        let mut controller = AdaptiveNoiseController::default();
        let cfg = MicrophoneDspConfig::default();
        let mut scene = active_scene();
        scene.double_talk = true;
        scene.speech_protected = true;
        let targets = settle(&mut controller, &scene, &cfg);
        assert_eq!(targets.residual_speaker_suppression_db, 0.0);
    }

    #[test]
    fn playback_only_may_use_up_to_forty_eight_db_residual_suppression() {
        let mut controller = AdaptiveNoiseController::default();
        let cfg = MicrophoneDspConfig::default();
        let targets = settle(&mut controller, &active_scene(), &cfg);
        assert!(targets.residual_speaker_suppression_db > 6.0);
        assert!(targets.residual_speaker_suppression_db <= 48.0);
    }

    #[test]
    fn target_changes_are_rate_limited_between_frames() {
        let mut controller = AdaptiveNoiseController::default();
        let cfg = MicrophoneDspConfig::default();
        let first = controller.update(&active_scene(), &cfg);
        let second = controller.update(&active_scene(), &cfg);
        assert!(
            (second.sonora_noise_percent - first.sonora_noise_percent).abs() <= 2.0 + f32::EPSILON
        );
        assert!(
            (second.residual_speaker_suppression_db - first.residual_speaker_suppression_db).abs()
                <= 1.5 + f32::EPSILON
        );
    }
}
