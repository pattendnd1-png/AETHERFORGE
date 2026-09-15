use crate::adaptive_noise::AdaptiveNoiseController;
use crate::background_rejection::BackgroundRejector;
use crate::noise_monitor::ExtraneousNoiseMonitor;
use crate::speaker_lock::{PlaybackLeakGuard, SpeakerVerifier};
use crate::transient::TransientSuppressor;
use crate::voice_only::VoiceOnlyProcessor;
use crate::voicepilot::{AdaptiveTargets, VoicePilotAnalyzer};
use forgehx_core::{
    MicEqFilterKind, MicrophoneDspConfig, NoiseSceneTelemetry, VoiceIsolationTelemetry,
    VoicePilotMode, VoicePilotTelemetry,
};
use sonora::config::{
    EchoCanceller, GainController2, HighPassFilter, NoiseSuppression, NoiseSuppressionLevel,
};
use sonora::{AudioProcessing, Config as SonoraConfig, StreamConfig};
use std::f32::consts::PI;

pub const SAMPLE_RATE: f32 = 48_000.0;
pub const FRAME_SAMPLES: usize = 480;
const NOISE_RECONFIGURE_FRAMES: usize = 200;

#[derive(Debug)]
pub struct VoiceProcessingEngine {
    config: MicrophoneDspConfig,
    apm: AudioProcessing,
    voicepilot: VoicePilotAnalyzer,
    speaker_verifier: SpeakerVerifier,
    playback_guard: PlaybackLeakGuard,
    voice_only: VoiceOnlyProcessor,
    click_suppressor: TransientSuppressor,
    background_rejector: BackgroundRejector,
    noise_monitor: ExtraneousNoiseMonitor,
    adaptive_noise: AdaptiveNoiseController,
    noise_scene_telemetry: NoiseSceneTelemetry,
    isolation_telemetry: VoiceIsolationTelemetry,
    current_noise_level: NoiseSuppressionLevel,
    noise_reconfigure_frames: usize,
}

impl VoiceProcessingEngine {
    pub fn new(config: MicrophoneDspConfig) -> Result<Self, String> {
        config.validate().map_err(|e| e.to_string())?;
        let current_noise_level =
            noise_level_from_percent(config.noise_suppression.strength_percent);
        let mut apm = AudioProcessing::builder()
            .config(sonora_config(
                &config,
                config.noise_suppression.strength_percent,
            ))
            .capture_config(StreamConfig::new(48_000, 1))
            .render_config(StreamConfig::new(48_000, 1))
            .build();
        apm.set_capture_pre_gain(db_to_gain(config.input_gain_db));
        if aec_enabled(&config) {
            let _ = apm.set_stream_delay_ms(0);
        }
        Ok(Self {
            speaker_verifier: SpeakerVerifier::new(&config.speaker_lock),
            playback_guard: PlaybackLeakGuard::new(&config.playback_rejection),
            voice_only: VoiceOnlyProcessor::default(),
            click_suppressor: TransientSuppressor::new(&config.click_suppression),
            background_rejector: BackgroundRejector::default(),
            noise_monitor: ExtraneousNoiseMonitor::default(),
            adaptive_noise: AdaptiveNoiseController::default(),
            noise_scene_telemetry: NoiseSceneTelemetry::default(),
            isolation_telemetry: VoiceIsolationTelemetry::default(),
            config,
            apm,
            voicepilot: VoicePilotAnalyzer::default(),
            current_noise_level,
            noise_reconfigure_frames: 0,
        })
    }

    pub fn update_config(&mut self, config: MicrophoneDspConfig) -> Result<(), String> {
        config.validate().map_err(|e| e.to_string())?;
        let noise_level = noise_level_from_percent(config.noise_suppression.strength_percent);
        let old_cleanup = cleanup_signature(&self.config, self.current_noise_level);
        let new_cleanup = cleanup_signature(&config, noise_level);
        if old_cleanup != new_cleanup {
            self.apm.apply_config(sonora_config(
                &config,
                config.noise_suppression.strength_percent,
            ));
            self.current_noise_level = noise_level;
            self.noise_reconfigure_frames = 0;
        }
        self.apm
            .set_capture_pre_gain(db_to_gain(config.input_gain_db));
        if aec_enabled(&config) {
            let _ = self.apm.set_stream_delay_ms(0);
        }
        self.speaker_verifier.update_config(&config.speaker_lock);
        self.playback_guard
            .update_config(&config.playback_rejection);
        self.click_suppressor
            .update_config(&config.click_suppression);
        self.config = config;
        Ok(())
    }

    pub fn telemetry(&self) -> VoicePilotTelemetry {
        self.voicepilot.telemetry()
    }

    pub fn voice_isolation_telemetry(&self) -> VoiceIsolationTelemetry {
        self.isolation_telemetry.clone()
    }

    pub fn noise_scene_telemetry(&self) -> NoiseSceneTelemetry {
        self.noise_scene_telemetry.clone()
    }

    pub fn begin_speaker_enrollment(&mut self) {
        self.speaker_verifier.begin_enrollment();
    }

    pub fn cancel_speaker_enrollment(&mut self) {
        self.speaker_verifier.cancel_enrollment();
    }

    pub fn forget_speaker_voiceprint(&mut self) {
        self.speaker_verifier.forget_voice();
    }

    pub fn take_completed_voiceprint(&mut self) -> Option<Vec<f32>> {
        self.speaker_verifier.take_completed_voiceprint()
    }

    pub fn process_10ms(
        &mut self,
        capture: &[f32; FRAME_SAMPLES],
        render: Option<&[f32; FRAME_SAMPLES]>,
    ) -> Result<[f32; FRAME_SAMPLES], String> {
        if let Some(render_frame) = render {
            self.playback_guard.observe_render(render_frame);
            self.voice_only.observe_render(render_frame);
        }

        let aligned_reference = self.voice_only.aligned_reference(capture);
        let aligned_render = aligned_reference.as_ref().map(|value| &value.frame);
        let reference_available = aligned_render.is_some();
        let alignment_confidence = aligned_reference
            .as_ref()
            .map(|value| value.confidence)
            .unwrap_or(0.0);

        let (playback_leak_score, playback_guard_rejected) = self.playback_guard.evaluate(capture);
        let playback_learning_blocked = playback_leak_score
            >= self.config.playback_rejection.correlation_threshold
            || alignment_confidence >= 0.12;

        if aec_enabled(&self.config) {
            if let Some(render_frame) = aligned_render {
                let mut render_dest = [0.0f32; FRAME_SAMPLES];
                self.apm
                    .process_render_f32(&[render_frame.as_slice()], &mut [&mut render_dest])
                    .map_err(|e| e.to_string())?;
            }
        }

        let mut cleaned = [0.0f32; FRAME_SAMPLES];
        self.apm
            .process_capture_f32(&[capture.as_slice()], &mut [&mut cleaned])
            .map_err(|e| e.to_string())?;

        // The reference has already been aligned to the acoustic capture time. Remove only the
        // component that is correlated with system playback; uncorrelated local speech remains.
        let voice_only_subtraction = if self
            .config
            .extraneous_noise_monitor
            .speaker_rejection_enabled
        {
            aligned_render
                .map(|reference| {
                    self.voice_only.subtract_correlated_playback(
                        &mut cleaned,
                        reference,
                        self.config.echo_cancellation.strength_percent,
                    )
                })
                .unwrap_or_default()
        } else {
            Default::default()
        };

        // Remove short acoustic keyboard/mouse switch transients before identity analysis and
        // the rest of the voice chain. This is audio-only and never records input key codes.
        self.click_suppressor.process(&mut cleaned);

        let observation = self.noise_monitor.analyze(
            capture,
            &cleaned,
            aligned_render,
            &self.config.extraneous_noise_monitor,
        );
        let adaptive = self
            .adaptive_noise
            .update(&observation.telemetry, &self.config);
        self.noise_scene_telemetry = observation.telemetry.clone();
        self.noise_scene_telemetry.adaptive_suppression_db = adaptive.rejection_cap_db;
        self.noise_scene_telemetry.sonora_noise_target_percent = adaptive.sonora_noise_percent;

        // Blanket residual attenuation is playback-only cleanup. During local speech/double-talk
        // it is zero; the aligned correlated subtractor above keeps removing playback without
        // turning the user's voice down with it.
        if adaptive.residual_speaker_suppression_db > 0.0 {
            let gain = 10.0f32.powf(-adaptive.residual_speaker_suppression_db / 20.0);
            for sample in &mut cleaned {
                *sample *= gain;
            }
        }

        // Sonora removes broadband noise first; this stateful stage then rejects the remaining
        // steady room/fan floor while preserving speech gaps via the configured grace window.
        self.background_rejector.process_adaptive(
            &mut cleaned,
            &self.config.noise_suppression,
            adaptive.background_margin_offset_db,
            adaptive.rejection_cap_db,
        );

        let speaker = self
            .speaker_verifier
            .evaluate(&cleaned, !playback_learning_blocked);
        let speaker_rejected =
            self.config.speaker_lock.enabled && speaker.enrolled && speaker.rejected;
        let confirmed_local_voice = speaker.enrolled && !speaker.rejected;
        let playback_evidence = playback_leak_score
            .max(voice_only_subtraction.correlation)
            .max(alignment_confidence);
        let playback_rejected = should_hard_reject_playback(
            self.config.playback_rejection.enabled,
            self.config.playback_rejection.hard_block,
            reference_available,
            playback_guard_rejected,
            voice_only_subtraction.playback_dominant,
            playback_evidence,
            confirmed_local_voice,
            observation.telemetry.speech_protected,
        );

        self.isolation_telemetry = VoiceIsolationTelemetry {
            enrolled: speaker.enrolled,
            enrollment_active: speaker.enrollment_active,
            enrollment_progress_percent: speaker.enrollment_progress_percent,
            speaker_match_score: speaker.score,
            playback_leak_score: playback_evidence,
            speaker_rejected,
            playback_rejected,
        };
        if playback_rejected || speaker_rejected {
            cleaned.fill(0.0);
        }

        let targets = if self.config.voicepilot.enabled {
            self.voicepilot
                .analyze(&cleaned, &self.config, reference_available)
        } else {
            AdaptiveTargets {
                bass_boost_db: self.config.tone.bass_boost_db,
                treble_boost_db: self.config.tone.treble_boost_db,
                noise_strength_percent: self.config.noise_suppression.strength_percent,
                ..Default::default()
            }
        };

        if self.config.noise_suppression.enabled
            && self.config.voicepilot.enabled
            && matches!(self.config.voicepilot.auto_noise, VoicePilotMode::Auto)
        {
            self.noise_reconfigure_frames = self.noise_reconfigure_frames.saturating_add(1);
            let desired_noise_percent = targets
                .noise_strength_percent
                .max(adaptive.sonora_noise_percent)
                .clamp(self.config.noise_suppression.strength_percent, 100.0);
            let desired_level = noise_level_from_percent(desired_noise_percent);
            if self.noise_reconfigure_frames >= NOISE_RECONFIGURE_FRAMES {
                self.noise_reconfigure_frames = 0;
                if desired_level != self.current_noise_level {
                    self.apm
                        .apply_config(sonora_config(&self.config, desired_noise_percent));
                    self.apm
                        .set_capture_pre_gain(db_to_gain(self.config.input_gain_db));
                    if aec_enabled(&self.config) {
                        let _ = self.apm.set_stream_delay_ms(0);
                    }
                    self.current_noise_level = desired_level;
                }
            }
        }

        let mut frame = cleaned;
        apply_gate(&mut frame, &self.config);
        apply_bass_treble(&mut frame, &self.config, targets);
        apply_multiband_eq(&mut frame, &self.config);
        apply_parametric_eq(&mut frame, &self.config, targets);
        apply_dynamic_eq(&mut frame, &self.config);
        apply_de_esser(&mut frame, &self.config, targets);
        apply_multiband_compressor(&mut frame, &self.config);
        apply_voice_enhancer(&mut frame, &self.config, targets);
        apply_compressor(&mut frame, &self.config, targets);
        apply_saturation(&mut frame, &self.config);
        apply_gain(
            &mut frame,
            self.config.output_gain_db + targets.output_trim_db,
        );
        apply_limiter(&mut frame, &self.config);
        Ok(frame)
    }
}

fn aec_enabled(config: &MicrophoneDspConfig) -> bool {
    config.echo_cancellation.enabled
        && (!config.voicepilot.enabled
            || !matches!(config.voicepilot.auto_aec, VoicePilotMode::Bypassed))
}

fn agc_enabled(config: &MicrophoneDspConfig) -> bool {
    config.voicepilot.enabled && matches!(config.voicepilot.auto_loudness, VoicePilotMode::Auto)
}

fn cleanup_signature(
    config: &MicrophoneDspConfig,
    noise_level: NoiseSuppressionLevel,
) -> (bool, bool, bool, NoiseSuppressionLevel, bool) {
    (
        config.high_pass_hz.is_some(),
        aec_enabled(config),
        config.noise_suppression.enabled,
        noise_level,
        agc_enabled(config),
    )
}

fn noise_level_from_percent(percent: f32) -> NoiseSuppressionLevel {
    match percent.clamp(0.0, 100.0) {
        value if value < 25.0 => NoiseSuppressionLevel::Low,
        value if value < 50.0 => NoiseSuppressionLevel::Moderate,
        value if value < 75.0 => NoiseSuppressionLevel::High,
        _ => NoiseSuppressionLevel::VeryHigh,
    }
}

fn sonora_config(config: &MicrophoneDspConfig, noise_percent: f32) -> SonoraConfig {
    SonoraConfig {
        high_pass_filter: config.high_pass_hz.map(|_| HighPassFilter::default()),
        echo_canceller: aec_enabled(config).then(EchoCanceller::default),
        noise_suppression: config.noise_suppression.enabled.then(|| NoiseSuppression {
            level: noise_level_from_percent(noise_percent),
            analyze_linear_aec_output_when_available: aec_enabled(config),
        }),
        gain_controller2: agc_enabled(config).then(GainController2::default),
        ..Default::default()
    }
}

fn apply_gain(frame: &mut [f32], db: f32) {
    let gain = db_to_gain(db);
    for sample in frame {
        *sample *= gain;
    }
}

fn apply_gate(frame: &mut [f32], config: &MicrophoneDspConfig) {
    if !config.gate.enabled {
        return;
    }
    if rms_db(frame) < config.gate.close_threshold_db {
        for sample in frame {
            *sample *= 0.05;
        }
    }
}

fn apply_bass_treble(frame: &mut [f32], config: &MicrophoneDspConfig, targets: AdaptiveTargets) {
    let bass = if config.voicepilot.enabled
        && matches!(config.voicepilot.auto_tone, VoicePilotMode::Auto)
    {
        targets.bass_boost_db
    } else {
        config.tone.bass_boost_db
    };
    let treble = if config.voicepilot.enabled
        && matches!(config.voicepilot.auto_tone, VoicePilotMode::Auto)
    {
        targets.treble_boost_db
    } else {
        config.tone.treble_boost_db
    };
    apply_biquad(
        frame,
        MicEqFilterKind::LowShelf,
        config.tone.bass_frequency_hz,
        bass.clamp(0.0, 12.0),
        0.707,
    );
    apply_biquad(
        frame,
        MicEqFilterKind::HighShelf,
        config.tone.treble_frequency_hz,
        treble.clamp(0.0, 12.0),
        0.707,
    );
}

fn apply_parametric_eq(frame: &mut [f32], config: &MicrophoneDspConfig, targets: AdaptiveTargets) {
    for band in config.eq.iter().filter(|b| b.enabled) {
        apply_biquad(frame, band.kind, band.frequency_hz, band.gain_db, band.q);
    }
    if config.voicepilot.enabled
        && matches!(config.voicepilot.auto_eq, VoicePilotMode::Auto)
        && targets.presence_db.abs() > 0.01
    {
        apply_biquad(
            frame,
            MicEqFilterKind::Bell,
            3200.0,
            targets.presence_db,
            0.9,
        );
    }
}

fn apply_multiband_eq(frame: &mut [f32], config: &MicrophoneDspConfig) {
    for band in config
        .multiband_eq
        .iter()
        .filter(|b| b.enabled && b.gain_db.abs() > 0.01)
    {
        let center = (band.low_hz * band.high_hz).sqrt();
        let q = (center / (band.high_hz - band.low_hz)).clamp(0.2, 8.0);
        apply_biquad(frame, MicEqFilterKind::Bell, center, band.gain_db, q);
    }
}

fn apply_dynamic_eq(frame: &mut [f32], config: &MicrophoneDspConfig) {
    let level = rms_db(frame);
    for band in config.dynamic_eq.iter().filter(|b| b.enabled) {
        if level > band.threshold_db {
            let over = level - band.threshold_db;
            let scale = (over / 18.0).clamp(0.0, 1.0);
            apply_biquad(
                frame,
                MicEqFilterKind::Bell,
                band.frequency_hz,
                band.gain_db * scale,
                band.q,
            );
        }
    }
}

fn apply_multiband_compressor(frame: &mut [f32], config: &MicrophoneDspConfig) {
    if config.multiband_compressor.is_empty() {
        return;
    }
    let original = frame.to_vec();
    let mut previous_lp = vec![0.0f32; frame.len()];
    let mut sum = vec![0.0f32; frame.len()];
    for (index, band) in config.multiband_compressor.iter().enumerate() {
        if !band.enabled {
            continue;
        }
        let mut current = if index + 1 == config.multiband_compressor.len() {
            original
                .iter()
                .zip(&previous_lp)
                .map(|(x, p)| x - p)
                .collect::<Vec<_>>()
        } else {
            let lp = lowpass(&original, band.high_hz);
            let part = lp
                .iter()
                .zip(&previous_lp)
                .map(|(x, p)| x - p)
                .collect::<Vec<_>>();
            previous_lp = lp;
            part
        };
        compress(
            &mut current,
            band.threshold_db,
            band.ratio,
            band.makeup_db,
            100.0,
        );
        for (dst, sample) in sum.iter_mut().zip(current) {
            *dst += sample;
        }
    }
    frame.copy_from_slice(&sum);
}

fn apply_de_esser(frame: &mut [f32], config: &MicrophoneDspConfig, targets: AdaptiveTargets) {
    if !config.de_esser.enabled {
        return;
    }
    let low = lowpass(
        frame,
        (config.de_esser.frequency_hz * 0.75).clamp(1200.0, 9000.0),
    );
    let high = frame
        .iter()
        .zip(low)
        .map(|(x, l)| x - l)
        .collect::<Vec<_>>();
    let high_db = rms_db(&high);
    if high_db <= config.de_esser.threshold_db {
        return;
    }
    let over = high_db - config.de_esser.threshold_db;
    let computed = over * (1.0 - 1.0 / config.de_esser.ratio.max(1.0));
    let automatic = if config.voicepilot.enabled
        && matches!(config.voicepilot.auto_de_esser, VoicePilotMode::Auto)
    {
        targets.de_esser_reduction_db
    } else {
        computed
    };
    let reduction = automatic.min(12.0) * (config.de_esser.amount_percent / 100.0);
    apply_biquad(
        frame,
        MicEqFilterKind::HighShelf,
        config.de_esser.frequency_hz,
        -reduction,
        0.8,
    );
}

fn apply_voice_enhancer(frame: &mut [f32], config: &MicrophoneDspConfig, targets: AdaptiveTargets) {
    let v = &config.voice_enhancer;
    let warmth = (v.warmth - 50.0) / 50.0 * 2.0;
    let body = (v.body - 50.0) / 50.0 * 2.5;
    let clarity = (v.clarity - 50.0) / 50.0 * 2.2;
    let presence = (v.presence - 50.0) / 50.0 * 2.0 + targets.presence_db * 0.35;
    let air = (v.air - 50.0) / 50.0 * 2.0;
    let depth = (v.depth - 50.0) / 50.0 * 1.5;
    apply_biquad(frame, MicEqFilterKind::LowShelf, 140.0, warmth + depth, 0.7);
    apply_biquad(frame, MicEqFilterKind::Bell, 220.0, body, 1.0);
    apply_biquad(frame, MicEqFilterKind::Bell, 1600.0, clarity, 0.9);
    apply_biquad(frame, MicEqFilterKind::Bell, 3500.0, presence, 0.9);
    apply_biquad(frame, MicEqFilterKind::HighShelf, 11000.0, air, 0.7);
}

fn apply_compressor(frame: &mut [f32], config: &MicrophoneDspConfig, targets: AdaptiveTargets) {
    if !config.compressor.enabled {
        return;
    }
    let mut threshold = config.compressor.threshold_db;
    if config.voicepilot.enabled && matches!(config.voicepilot.auto_dynamics, VoicePilotMode::Auto)
    {
        threshold -= targets.compressor_reduction_db * 0.25;
    }
    compress(
        frame,
        threshold,
        config.compressor.ratio,
        config.compressor.makeup_db,
        config.compressor.mix_percent,
    );
}

fn apply_saturation(frame: &mut [f32], config: &MicrophoneDspConfig) {
    if !config.saturation.enabled || config.saturation.mix_percent <= 0.0 {
        return;
    }
    let drive = db_to_gain(config.saturation.drive_db);
    let mix = (config.saturation.mix_percent / 100.0).clamp(0.0, 1.0);
    let norm = drive.tanh().max(1.0e-4);
    for sample in frame {
        let wet = (*sample * drive).tanh() / norm;
        *sample = *sample * (1.0 - mix) + wet * mix;
    }
}

fn apply_limiter(frame: &mut [f32], config: &MicrophoneDspConfig) {
    if !config.limiter.enabled {
        return;
    }
    let ceiling = db_to_gain(config.limiter.ceiling_db);
    let peak = frame.iter().fold(0.0f32, |p, x| p.max(x.abs()));
    if peak > ceiling {
        let gain = ceiling / peak.max(1.0e-6);
        for sample in frame {
            *sample *= gain;
        }
    }
}

fn compress(frame: &mut [f32], threshold_db: f32, ratio: f32, makeup_db: f32, mix_percent: f32) {
    let level = rms_db(frame);
    if level <= threshold_db {
        apply_gain(frame, makeup_db);
        return;
    }
    let over = level - threshold_db;
    let reduction_db = over - over / ratio.max(1.0);
    let wet_gain = db_to_gain(makeup_db - reduction_db);
    let mix = (mix_percent / 100.0).clamp(0.0, 1.0);
    for sample in frame {
        let dry = *sample;
        let wet = dry * wet_gain;
        *sample = dry * (1.0 - mix) + wet * mix;
    }
}

fn lowpass(input: &[f32], cutoff_hz: f32) -> Vec<f32> {
    let alpha = 1.0 - (-2.0 * PI * cutoff_hz.clamp(20.0, 20_000.0) / SAMPLE_RATE).exp();
    let mut state = 0.0f32;
    input
        .iter()
        .map(|&sample| {
            state += alpha * (sample - state);
            state
        })
        .collect()
}

fn apply_biquad(frame: &mut [f32], kind: MicEqFilterKind, frequency_hz: f32, gain_db: f32, q: f32) {
    let mut filter = Biquad::new(kind, frequency_hz, gain_db, q);
    for sample in frame {
        *sample = filter.process(*sample);
    }
}

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

impl Biquad {
    fn new(kind: MicEqFilterKind, frequency_hz: f32, gain_db: f32, q: f32) -> Self {
        let f = frequency_hz.clamp(20.0, 20_000.0);
        let q = q.clamp(0.1, 18.0);
        let w0 = 2.0 * PI * f / SAMPLE_RATE;
        let cosw = w0.cos();
        let sinw = w0.sin();
        let alpha = sinw / (2.0 * q);
        let a = 10.0f32.powf(gain_db / 40.0);
        let (b0, b1, b2, a0, a1, a2) = match kind {
            MicEqFilterKind::Bell => (
                1.0 + alpha * a,
                -2.0 * cosw,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cosw,
                1.0 - alpha / a,
            ),
            MicEqFilterKind::LowPass => (
                (1.0 - cosw) / 2.0,
                1.0 - cosw,
                (1.0 - cosw) / 2.0,
                1.0 + alpha,
                -2.0 * cosw,
                1.0 - alpha,
            ),
            MicEqFilterKind::HighPass => (
                (1.0 + cosw) / 2.0,
                -(1.0 + cosw),
                (1.0 + cosw) / 2.0,
                1.0 + alpha,
                -2.0 * cosw,
                1.0 - alpha,
            ),
            MicEqFilterKind::Notch => {
                (1.0, -2.0 * cosw, 1.0, 1.0 + alpha, -2.0 * cosw, 1.0 - alpha)
            }
            MicEqFilterKind::LowShelf => {
                let sqrt_a = a.sqrt();
                let two = 2.0 * sqrt_a * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cosw + two),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cosw),
                    a * ((a + 1.0) - (a - 1.0) * cosw - two),
                    (a + 1.0) + (a - 1.0) * cosw + two,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cosw),
                    (a + 1.0) + (a - 1.0) * cosw - two,
                )
            }
            MicEqFilterKind::HighShelf => {
                let sqrt_a = a.sqrt();
                let two = 2.0 * sqrt_a * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cosw + two),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cosw),
                    a * ((a + 1.0) + (a - 1.0) * cosw - two),
                    (a + 1.0) - (a - 1.0) * cosw + two,
                    2.0 * ((a - 1.0) - (a + 1.0) * cosw),
                    (a + 1.0) - (a - 1.0) * cosw - two,
                )
            }
        };
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

fn rms_db(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return -90.0;
    }
    let p = frame.iter().map(|x| x * x).sum::<f32>() / frame.len() as f32;
    20.0 * p.sqrt().max(1.0e-5).log10()
}

fn should_hard_reject_playback(
    enabled: bool,
    hard_block: bool,
    reference_available: bool,
    playback_guard_rejected: bool,
    playback_dominant: bool,
    playback_evidence: f32,
    confirmed_local_voice: bool,
    speech_protected: bool,
) -> bool {
    if !enabled || !hard_block || !reference_available {
        return false;
    }

    let playback_triggered =
        playback_guard_rejected || playback_dominant || playback_evidence >= 0.14;
    if !playback_triggered {
        return false;
    }

    // `speech_protected` is a spectral hint, not proof of local speech. When the dedicated
    // playback guard or correlated subtractor has already proven system playback, that hint
    // must not veto hard rejection. A confirmed enrolled-speaker match remains authoritative
    // and preserves the user's voice during real double-talk.
    let confirmed_playback = playback_guard_rejected || playback_dominant;
    let advisory_local_speech = speech_protected && !confirmed_playback;
    let local_voice_present = confirmed_local_voice || advisory_local_speech;

    !local_voice_present
}

fn db_to_gain(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_stays_finite() {
        let mut engine = VoiceProcessingEngine::new(MicrophoneDspConfig::default()).unwrap();
        let out = engine
            .process_10ms(&[0.0; FRAME_SAMPLES], Some(&[0.0; FRAME_SAMPLES]))
            .unwrap();
        assert!(out.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn full_engine_rejects_learned_room_noise() {
        let mut config = MicrophoneDspConfig::default();
        config.high_pass_hz = None;
        config.echo_cancellation.enabled = false;
        let mut engine = VoiceProcessingEngine::new(config).unwrap();
        let mut output = [0.0f32; FRAME_SAMPLES];
        for _ in 0..220 {
            let input = [0.063f32; FRAME_SAMPLES];
            output = engine.process_10ms(&input, None).unwrap();
        }
        let peak = output
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(
            peak < 0.001,
            "full DSP path left loud learned room noise at {peak}"
        );
    }

    #[test]
    fn tone_and_limiter_keep_output_bounded() {
        let mut cfg = MicrophoneDspConfig::default();
        cfg.tone.bass_boost_db = 6.0;
        cfg.tone.treble_boost_db = 6.0;
        let mut engine = VoiceProcessingEngine::new(cfg).unwrap();
        let input = [0.8; FRAME_SAMPLES];
        let out = engine.process_10ms(&input, None).unwrap();
        assert!(out.iter().all(|x| x.abs() <= 1.0));
    }

    #[test]
    fn monitor_disabled_keeps_10_0_27_processing_behavior() {
        let mut cfg = MicrophoneDspConfig::default();
        cfg.extraneous_noise_monitor.enabled = false;
        cfg.echo_cancellation.enabled = false;
        let mut engine = VoiceProcessingEngine::new(cfg).unwrap();
        let input = [0.03; FRAME_SAMPLES];
        let _ = engine.process_10ms(&input, None).unwrap();
        assert!(!engine.noise_scene_telemetry().active);
    }

    #[test]
    fn reference_missing_does_not_fail_capture_processing() {
        let mut engine = VoiceProcessingEngine::new(MicrophoneDspConfig::default()).unwrap();
        let input = [0.02; FRAME_SAMPLES];
        let output = engine.process_10ms(&input, None).unwrap();
        assert!(output.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn playback_only_is_hard_rejected_after_alignment() {
        let mut cfg = MicrophoneDspConfig::default();
        cfg.high_pass_hz = None;
        cfg.gate.enabled = false;
        cfg.noise_suppression.enabled = false;
        cfg.speaker_lock.enabled = false;
        let mut engine = VoiceProcessingEngine::new(cfg).unwrap();
        let delay_frames = 90usize;
        let mut history = Vec::new();
        let mut output = [0.0f32; FRAME_SAMPLES];
        for index in 0..360usize {
            let mut render = [0.0f32; FRAME_SAMPLES];
            let amplitude = 0.10 + 0.06 * (index as f32 * 0.113).sin();
            for (i, sample) in render.iter_mut().enumerate() {
                *sample = amplitude * (i as f32 * (0.047 + (index % 5) as f32 * 0.0019)).sin();
            }
            history.push(render);
            let capture = if index >= delay_frames {
                history[index - delay_frames].map(|sample| sample * 0.55)
            } else {
                [0.0; FRAME_SAMPLES]
            };
            output = engine.process_10ms(&capture, Some(&render)).unwrap();
        }
        let peak = output
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(
            peak < 0.001,
            "playback-only frame leaked through at peak {peak}"
        );
    }

    #[test]
    fn false_speech_protection_cannot_override_confirmed_playback() {
        assert!(should_hard_reject_playback(
            true, true, true, true, false, 0.6202, false, true,
        ));
    }

    #[test]
    fn confirmed_enrolled_voice_overrides_playback_hard_block() {
        assert!(!should_hard_reject_playback(
            true, true, true, true, true, 0.91, true, true,
        ));
    }

    #[test]
    fn weak_playback_evidence_keeps_speech_protection() {
        assert!(!should_hard_reject_playback(
            true, true, true, false, false, 0.18, false, true,
        ));
    }

    #[test]
    fn adaptive_targets_cannot_exceed_product_bounds() {
        let mut engine = VoiceProcessingEngine::new(MicrophoneDspConfig::default()).unwrap();
        let input = [0.03; FRAME_SAMPLES];
        for _ in 0..160 {
            let _ = engine.process_10ms(&input, None).unwrap();
        }
        let telemetry = engine.noise_scene_telemetry();
        assert!(telemetry.adaptive_suppression_db <= 72.0);
        assert!(telemetry.sonora_noise_target_percent <= 100.0);
    }
}
