use crate::{theme, widgets};
use eframe::egui::{self, RichText};
use forgehx_core::{
    Capability, DeviceInfo, MicEqBand, MicEqFilterKind, MicrophoneDspConfig, MicrophoneDspState,
    NoiseAdaptationState, VoicePilotMode, VoicePilotTarget,
};

#[derive(Debug, Clone)]
pub enum MicControl {
    Save(MicrophoneDspConfig),
    LiveUpdate(MicrophoneDspConfig),
    Apply(MicrophoneDspConfig),
    ForgetVoice,
    Monitor { enabled: bool, level_percent: f32 },
}

pub fn show(
    ui: &mut egui::Ui,
    device: &DeviceInfo,
    config: &mut MicrophoneDspConfig,
    state: Option<&MicrophoneDspState>,
    live_edits: bool,
    monitor_enabled: &mut bool,
    monitor_level_percent: &mut f32,
) -> Option<MicControl> {
    config.enabled = true;
    let before = config.clone();
    let mut action = None;

    ui.heading("Input DSP");
    ui.label(
        RichText::new(
            "HyperX capture processing only. Playback is sampled only as the AEC reference.",
        )
        .color(theme::text_secondary()),
    );
    if let Some(owner) = device.selected_owner(Capability::MicDsp) {
        widgets::info_row(ui, "Processing backend", owner.backend);
    }
    widgets::info_row(
        ui,
        "Processed source",
        state
            .map(|value| value.processed_source_node_name.as_str())
            .unwrap_or("aetherstream.system.microphone"),
    );
    if let Some(state) = state {
        widgets::info_row(
            ui,
            "Graph",
            if state.applied {
                "Permanent / live"
            } else {
                "Permanent / reconnecting"
            },
        );
        if let Some(raw) = &state.raw_source_node_name {
            widgets::info_row(ui, "Raw source", raw);
        }
        let t = &state.voicepilot_telemetry;
        if t.active {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("VoicePilot • input {:.1} dBFS", t.input_rms_db));
                ui.separator();
                ui.label(format!("noise floor {:.1} dBFS", t.noise_floor_db));
                ui.separator();
                ui.label(if t.reference_available {
                    "AEC reference active"
                } else {
                    "AEC reference unavailable"
                });
            });
        }
        for item in &state.unavailable_processors {
            ui.colored_label(theme::status_warn(), item);
        }
    }

    ui.separator();
    ui.group(|ui| {
        ui.strong("Live Headphone Monitor");
        let before_enabled = *monitor_enabled;
        let before_level = *monitor_level_percent;
        ui.checkbox(monitor_enabled, "Monitor ForgeHX Mic through wired headphones");
        ui.add_enabled(*monitor_enabled, egui::Slider::new(monitor_level_percent, 0.0..=100.0).text("Monitor level (%)"));
        if let Some(state) = state {
            if state.monitor.active {
                if let Some(sink) = &state.monitor.sink_node_name {
                    ui.colored_label(theme::status_ok(), format!("LIVE → {sink}"));
                } else {
                    ui.colored_label(theme::status_ok(), "LIVE → wired analog/headphone output");
                }
            } else if state.monitor.enabled {
                if let Some(error) = &state.monitor.error {
                    ui.colored_label(theme::status_warn(), error);
                } else {
                    ui.label(RichText::new("Monitor is reconnecting to the wired headphone output.").color(theme::text_secondary()));
                }
            }
        }
        ui.label(RichText::new("Post-DSP monitor only: this is the same ForgeHX Mic signal used by the stream. Bluetooth is never used for mic monitoring.").small().color(theme::text_secondary()));
        if before_enabled != *monitor_enabled || (before_level - *monitor_level_percent).abs() > f32::EPSILON {
            action = Some(MicControl::Monitor { enabled: *monitor_enabled, level_percent: *monitor_level_percent });
        }
    });

    ui.separator();
    ui.heading("Basic voice controls");
    ui.colored_label(theme::status_ok(), "DSP permanently enabled");
    ui.label(RichText::new("ForgeHX keeps this processing path attached across microphone, Bluetooth/speaker, PipeWire, daemon, and GUI reconnects.").small().color(theme::text_secondary()));
    ui.checkbox(
        &mut config.voicepilot.enabled,
        "Automatic Voice Processing (VoicePilot)",
    );
    ui.horizontal(|ui| {
        ui.label("Voice target");
        egui::ComboBox::from_id_salt("voicepilot_target")
            .selected_text(target_label(config.voicepilot.target))
            .show_ui(ui, |ui| {
                for target in [
                    VoicePilotTarget::Natural,
                    VoicePilotTarget::Clear,
                    VoicePilotTarget::Warm,
                    VoicePilotTarget::Full,
                    VoicePilotTarget::Broadcast,
                    VoicePilotTarget::BroadcastFull,
                    VoicePilotTarget::DeepAndFull,
                    VoicePilotTarget::CrispStream,
                    VoicePilotTarget::VoiceChat,
                    VoicePilotTarget::Custom,
                ] {
                    ui.selectable_value(
                        &mut config.voicepilot.target,
                        target,
                        target_label(target),
                    );
                }
            });
    });
    ui.add(
        egui::Slider::new(&mut config.voicepilot.adaptation_strength, 0.0..=100.0)
            .text("Adaptation strength (%)"),
    );

    ui.group(|ui| {
        ui.strong("Cleanup");
        ui.checkbox(&mut config.echo_cancellation.enabled, "Echo Cancellation (AEC3)");
        ui.add(egui::Slider::new(&mut config.echo_cancellation.strength_percent, 0.0..=100.0).text("Echo suppression (%)"));
        ui.checkbox(&mut config.echo_cancellation.automatic_reference, "Automatic playback reference");
        ui.checkbox(&mut config.echo_cancellation.residual_suppression, "Residual echo suppression");
        ui.checkbox(&mut config.noise_suppression.enabled, "Noise Reduction + Background Rejection");
        ui.add(egui::Slider::new(&mut config.noise_suppression.strength_percent, 0.0..=100.0).text("Noise Reduction / Background Rejection strength (%)"));
        ui.separator();
        ui.strong("Keyboard / Mouse Click Suppression");
        ui.checkbox(&mut config.click_suppression.enabled, "Suppress keyboard and mouse clicks");
        ui.add(egui::Slider::new(&mut config.click_suppression.sensitivity_percent, 0.0..=100.0).text("Click suppression sensitivity (%)"));
        ui.add(egui::Slider::new(&mut config.click_suppression.suppression_db, 0.0..=72.0).text("Click reduction (dB)"));
        ui.add(egui::Slider::new(&mut config.click_suppression.max_click_ms, 1.0..=30.0).text("Maximum click window (ms)"));
        ui.add(egui::Slider::new(&mut config.click_suppression.recovery_ms, 0.0..=100.0).text("Click recovery (ms)"));
        ui.label(RichText::new("Acoustic transient detection only: ForgeHX does not record key presses or mouse buttons.").small().color(theme::text_secondary()));
    });

    ui.group(|ui| {
        ui.strong("Noise Environment");
        ui.checkbox(&mut config.extraneous_noise_monitor.enabled, "Extraneous Noise Monitor");
        ui.checkbox(&mut config.extraneous_noise_monitor.auto_adapt, "Auto Noise Adapt");
        ui.add(egui::Slider::new(
            &mut config.extraneous_noise_monitor.sensitivity_percent,
            0.0..=100.0,
        ).text("Noise monitor sensitivity (%)"));
        ui.add(egui::Slider::new(
            &mut config.extraneous_noise_monitor.max_adaptive_suppression_db,
            0.0..=72.0,
        ).text("Maximum adaptive rejection (dB)"));
        ui.checkbox(
            &mut config.extraneous_noise_monitor.speaker_rejection_enabled,
            "Reject speaker/system playback leakage",
        );

        if let Some(state) = state {
            let noise = &state.noise_scene_telemetry;
            let status_color = match noise.adaptation_state {
                NoiseAdaptationState::Suppressing => theme::status_ok(),
                NoiseAdaptationState::SpeechProtected | NoiseAdaptationState::ReferenceUnavailable => theme::status_warn(),
                NoiseAdaptationState::Learning | NoiseAdaptationState::Holding => theme::text_secondary(),
            };
            ui.colored_label(status_color, format!("State: {}", noise_state_label(noise.adaptation_state)));
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("environment {:.1} dBFS", noise.overall_noise_dbfs));
                ui.separator();
                ui.label(if noise.reference_available { "speaker reference ACTIVE" } else { "speaker reference UNAVAILABLE" });
                ui.separator();
                ui.label(format!("adaptive {:.1} dB", noise.adaptive_suppression_db));
                ui.separator();
                ui.label(format!("Sonora target {:.0}%", noise.sonora_noise_target_percent));
            });
            let score = |value: f32| (value.clamp(0.0, 1.0) * 100.0).round();
            ui.label(format!(
                "Pink {:.0}%  •  White {:.0}%  •  Broadband/Fan {:.0}%  •  Hum/Rumble {:.0}%",
                score(noise.classes.pink_like),
                score(noise.classes.white_like),
                score(noise.classes.broadband),
                score(noise.classes.hum_rumble),
            ));
            ui.label(format!(
                "Whine {:.0}%  •  Transient {:.0}%  •  Speaker leak {:.0}%",
                score(noise.classes.narrowband_whine),
                score(noise.classes.transient),
                score(noise.classes.speaker_leak),
            ));
            ui.label(format!(
                "Band floors dBFS: sub {:.1} • low {:.1} • low-mid {:.1} • mid {:.1} • presence {:.1} • high {:.1} • air {:.1}",
                noise.band_floor_dbfs.sub_rumble_dbfs, noise.band_floor_dbfs.low_dbfs,
                noise.band_floor_dbfs.low_mid_dbfs, noise.band_floor_dbfs.mid_dbfs,
                noise.band_floor_dbfs.presence_dbfs, noise.band_floor_dbfs.high_dbfs,
                noise.band_floor_dbfs.air_dbfs,
            ));
            if !noise.dominant_bands.is_empty() {
                ui.label(format!("Dominant noise bands: {}", noise.dominant_bands.join(", ")));
            }
            if noise.speech_protected { ui.colored_label(theme::status_ok(), "LOCAL SPEECH PROTECTED"); }
            if noise.double_talk { ui.colored_label(theme::status_warn(), "DOUBLE-TALK: speaker rejection bounded to preserve your voice"); }
        }
        ui.label(RichText::new("ForgeHX analyzes environmental noise and the full system playback reference only to reject leakage/noise. System playback is never routed into the microphone output.").small().color(theme::text_secondary()));
    });

    ui.group(|ui| {
        ui.strong("Voice Isolation / Anti-Loop");
        ui.checkbox(&mut config.speaker_lock.enabled, "Only pass my enrolled voice");
        ui.checkbox(&mut config.speaker_lock.continuous_learning, "Continuous voice learning");
        ui.add(egui::Slider::new(&mut config.speaker_lock.match_threshold, 0.50..=0.99).text("Voice match strictness"));
        ui.add(egui::Slider::new(&mut config.speaker_lock.min_speech_db, -80.0..=-20.0).text("Minimum speech level (dBFS)"));
        ui.checkbox(&mut config.playback_rejection.enabled, "Reject audio reproduced by speakers");
        ui.checkbox(&mut config.playback_rejection.hard_block, "Hard-block playback matches / loops");
        ui.add(egui::Slider::new(&mut config.playback_rejection.correlation_threshold, 0.20..=0.99).text("Playback match threshold"));
        ui.add(egui::Slider::new(&mut config.playback_rejection.history_ms, 100..=2000).text("Playback history (ms)"));

        if let Some(state) = state {
            let isolation = &state.voice_isolation_telemetry;
            ui.horizontal_wrapped(|ui| {
                ui.label(if isolation.enrolled { "Voice profile: ENROLLED" } else { "Voice profile: NOT ENROLLED" });
                ui.separator();
                ui.label(format!("match {:.0}%", isolation.speaker_match_score * 100.0));
                ui.separator();
                ui.label(format!("playback leak {:.0}%", isolation.playback_leak_score * 100.0));
            });
            if isolation.speaker_rejected { ui.colored_label(theme::status_warn(), "Audio rejected: speaker identity mismatch"); }
            if isolation.playback_rejected { ui.colored_label(theme::status_warn(), "Audio rejected: speaker/playback loop match"); }
            if config.speaker_lock.continuous_learning {
                if isolation.enrollment_active {
                    ui.add(egui::ProgressBar::new(isolation.enrollment_progress_percent as f32 / 100.0).text(format!("Learning your voice automatically: {}%", isolation.enrollment_progress_percent)));
                } else if isolation.enrolled {
                    ui.label(RichText::new("Continuous voice learning: ACTIVE — your profile refines from high-confidence speech.").color(theme::status_ok()));
                } else if state.applied {
                    ui.label(RichText::new("Continuous voice learning: waiting for your voice — just speak normally.").color(theme::text_secondary()));
                } else {
                    ui.label(RichText::new("Continuous voice learning starts automatically with the permanent ForgeHX Mic.").color(theme::text_secondary()));
                }
            } else if state.applied {
                ui.label(RichText::new("Continuous voice learning is disabled.").color(theme::text_secondary()));
            }
            if isolation.enrolled && ui.button("Reset Learned Voice").clicked() { action = Some(MicControl::ForgetVoice); }
        }
        ui.label(RichText::new("Continuous learning stores only the derived voiceprint, never raw microphone recordings. Playback rejection always wins over speaker identity, so speaker audio cannot train or unlock the voice profile.").small().color(theme::text_secondary()));
    });

    ui.group(|ui| {
        ui.strong("Tone");
        ui.add(
            egui::Slider::new(&mut config.tone.bass_boost_db, 0.0..=12.0).text("Bass Boost (dB)"),
        );
        ui.add(
            egui::Slider::new(&mut config.tone.treble_boost_db, 0.0..=12.0)
                .text("Treble Boost (dB)"),
        );
        ui.add(egui::Slider::new(&mut config.voice_enhancer.warmth, 0.0..=100.0).text("Warmth"));
        ui.add(
            egui::Slider::new(&mut config.voice_enhancer.body, 0.0..=100.0).text("Body / Fullness"),
        );
        ui.add(egui::Slider::new(&mut config.voice_enhancer.clarity, 0.0..=100.0).text("Clarity"));
        ui.add(
            egui::Slider::new(&mut config.voice_enhancer.presence, 0.0..=100.0).text("Presence"),
        );
        ui.add(egui::Slider::new(&mut config.voice_enhancer.air, 0.0..=100.0).text("Air"));
    });

    ui.add(
        egui::Slider::new(&mut config.compressor.mix_percent, 0.0..=100.0)
            .text("Compression / Parallel mix (%)"),
    );
    ui.add(egui::Slider::new(&mut config.output_gain_db, -30.0..=24.0).text("Output level (dB)"));

    ui.separator();
    ui.collapsing("Advanced DSP", |ui| {
        ui.horizontal(|ui| {
            ui.label("Preset");
            ui.text_edit_singleline(&mut config.name);
            ui.label("Broadcast Full is the VoicePilot default");
        });
        ui.add(egui::Slider::new(&mut config.input_gain_db, -30.0..=24.0).text("Input trim (dB)"));

        ui.collapsing("VoicePilot automation domains", |ui| {
            automation_mode(ui, "Noise", &mut config.voicepilot.auto_noise);
            automation_mode(ui, "AEC", &mut config.voicepilot.auto_aec);
            automation_mode(ui, "EQ", &mut config.voicepilot.auto_eq);
            automation_mode(ui, "Bass / Treble", &mut config.voicepilot.auto_tone);
            automation_mode(ui, "Dynamics", &mut config.voicepilot.auto_dynamics);
            automation_mode(ui, "De-Esser", &mut config.voicepilot.auto_de_esser);
            automation_mode(ui, "Loudness", &mut config.voicepilot.auto_loudness);
            ui.small("Modes: AUTO • MANUAL • LOCKED • BYPASSED");
        });

        let mut hpf_enabled = config.high_pass_hz.is_some();
        if ui.checkbox(&mut hpf_enabled, "High-pass filter").changed() {
            config.high_pass_hz = hpf_enabled.then_some(config.high_pass_hz.unwrap_or(80.0));
        }
        if let Some(hz) = &mut config.high_pass_hz { ui.add(egui::Slider::new(hz, 20.0..=500.0).text("High-pass (Hz)")); }

        ui.collapsing("Bass / Treble detail", |ui| {
            ui.add(egui::Slider::new(&mut config.tone.bass_frequency_hz, 40.0..=300.0).text("Bass shelf frequency (Hz)"));
            ui.add(egui::Slider::new(&mut config.tone.treble_frequency_hz, 2000.0..=16000.0).logarithmic(true).text("Treble shelf frequency (Hz)"));
        });

        ui.collapsing("Parametric EQ", |ui| {
            let mut remove = None;
            for (index, band) in config.eq.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut band.enabled, format!("Band {}", index + 1));
                        eq_kind(ui, index, &mut band.kind);
                        if ui.small_button("Remove").clicked() { remove = Some(index); }
                    });
                    ui.add(egui::Slider::new(&mut band.frequency_hz, 20.0..=20000.0).logarithmic(true).text("Frequency (Hz)"));
                    ui.add(egui::Slider::new(&mut band.gain_db, -24.0..=24.0).text("Gain (dB)"));
                    ui.add(egui::Slider::new(&mut band.q, 0.1..=18.0).text("Q"));
                });
            }
            if let Some(index) = remove { config.eq.remove(index); }
            if config.eq.len() < 12 && ui.button("Add EQ band").clicked() {
                config.eq.push(MicEqBand { enabled: true, kind: MicEqFilterKind::Bell, frequency_hz: 1000.0, gain_db: 0.0, q: 1.0 });
            }
        });

        ui.collapsing("Multiband EQ", |ui| {
            for (index, band) in config.multiband_eq.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.checkbox(&mut band.enabled, format!("Band {}", index + 1));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut band.low_hz).range(20.0..=20000.0).suffix(" Hz low"));
                        ui.add(egui::DragValue::new(&mut band.high_hz).range(20.0..=20000.0).suffix(" Hz high"));
                    });
                    ui.add(egui::Slider::new(&mut band.gain_db, -24.0..=24.0).text("Band gain (dB)"));
                });
            }
        });

        ui.collapsing("Dynamic EQ", |ui| {
            for (index, band) in config.dynamic_eq.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.checkbox(&mut band.enabled, format!("Dynamic band {}", index + 1));
                    ui.add(egui::Slider::new(&mut band.frequency_hz, 20.0..=20000.0).logarithmic(true).text("Frequency (Hz)"));
                    ui.add(egui::Slider::new(&mut band.gain_db, -24.0..=24.0).text("Maximum cut / boost (dB)"));
                    ui.add(egui::Slider::new(&mut band.q, 0.1..=18.0).text("Q"));
                    ui.add(egui::Slider::new(&mut band.threshold_db, -80.0..=0.0).text("Threshold (dB)"));
                    ui.add(egui::Slider::new(&mut band.ratio, 1.0..=20.0).text("Ratio"));
                });
            }
        });

        ui.collapsing("Gate / Expander", |ui| {
            ui.checkbox(&mut config.gate.enabled, "Gate enabled");
            ui.add(egui::Slider::new(&mut config.gate.open_threshold_db, -90.0..=0.0).text("Open threshold (dB)"));
            ui.add(egui::Slider::new(&mut config.gate.close_threshold_db, -90.0..=0.0).text("Close threshold (dB)"));
            ui.add(egui::Slider::new(&mut config.gate.attack_ms, 0.1..=500.0).text("Attack (ms)"));
            ui.add(egui::Slider::new(&mut config.gate.release_ms, 1.0..=5000.0).text("Release (ms)"));
        });

        ui.collapsing("Multiband Compressor", |ui| {
            for (index, band) in config.multiband_compressor.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.checkbox(&mut band.enabled, format!("Compressor band {}", index + 1));
                    ui.label(format!("{:.0}–{:.0} Hz", band.low_hz, band.high_hz));
                    ui.add(egui::Slider::new(&mut band.threshold_db, -60.0..=0.0).text("Threshold (dB)"));
                    ui.add(egui::Slider::new(&mut band.ratio, 1.0..=20.0).text("Ratio"));
                    ui.add(egui::Slider::new(&mut band.attack_ms, 0.1..=500.0).text("Attack (ms)"));
                    ui.add(egui::Slider::new(&mut band.release_ms, 1.0..=5000.0).text("Release (ms)"));
                    ui.add(egui::Slider::new(&mut band.makeup_db, -24.0..=24.0).text("Makeup (dB)"));
                });
            }
        });

        ui.collapsing("Main Compressor", |ui| {
            ui.checkbox(&mut config.compressor.enabled, "Compressor enabled");
            ui.add(egui::Slider::new(&mut config.compressor.threshold_db, -60.0..=0.0).text("Threshold (dB)"));
            ui.add(egui::Slider::new(&mut config.compressor.ratio, 1.0..=20.0).text("Ratio"));
            ui.add(egui::Slider::new(&mut config.compressor.knee_db, 0.0..=24.0).text("Knee (dB)"));
            ui.add(egui::Slider::new(&mut config.compressor.attack_ms, 0.1..=500.0).text("Attack (ms)"));
            ui.add(egui::Slider::new(&mut config.compressor.release_ms, 1.0..=5000.0).text("Release (ms)"));
            ui.add(egui::Slider::new(&mut config.compressor.makeup_db, -24.0..=24.0).text("Makeup (dB)"));
            ui.add(egui::Slider::new(&mut config.compressor.mix_percent, 0.0..=100.0).text("Parallel mix (%)"));
        });

        ui.collapsing("De-Esser", |ui| {
            ui.checkbox(&mut config.de_esser.enabled, "De-Esser enabled");
            ui.add(egui::Slider::new(&mut config.de_esser.frequency_hz, 2000.0..=12000.0).text("Sibilance frequency (Hz)"));
            ui.add(egui::Slider::new(&mut config.de_esser.threshold_db, -60.0..=0.0).text("Threshold (dB)"));
            ui.add(egui::Slider::new(&mut config.de_esser.ratio, 1.0..=20.0).text("Ratio"));
            ui.add(egui::Slider::new(&mut config.de_esser.attack_ms, 0.1..=100.0).text("Attack (ms)"));
            ui.add(egui::Slider::new(&mut config.de_esser.release_ms, 1.0..=1000.0).text("Release (ms)"));
            ui.add(egui::Slider::new(&mut config.de_esser.amount_percent, 0.0..=100.0).text("Amount (%)"));
        });

        ui.collapsing("Voice Enhancer", |ui| {
            ui.add(egui::Slider::new(&mut config.voice_enhancer.depth, 0.0..=100.0).text("Depth"));
            ui.label("Warmth, Body / Fullness, Clarity, Presence and Air are available in Basic controls above.");
        });

        ui.collapsing("Saturation", |ui| {
            ui.checkbox(&mut config.saturation.enabled, "Saturation enabled");
            ui.add(egui::Slider::new(&mut config.saturation.drive_db, 0.0..=24.0).text("Drive (dB)"));
            ui.add(egui::Slider::new(&mut config.saturation.mix_percent, 0.0..=100.0).text("Mix (%)"));
        });

        ui.collapsing("Limiter", |ui| {
            ui.checkbox(&mut config.limiter.enabled, "Limiter enabled");
            ui.add(egui::Slider::new(&mut config.limiter.ceiling_db, -12.0..=0.0).text("Ceiling (dBFS)"));
            ui.add(egui::Slider::new(&mut config.limiter.release_ms, 0.25..=20.0).text("Release (ms)"));
            ui.add(egui::Slider::new(&mut config.limiter.lookahead_ms, 0.0..=20.0).text("Look-ahead (ms)"));
            ui.checkbox(&mut config.limiter.true_peak, "True-peak protection");
        });
    });

    // Manual control always wins over VoicePilot for the specific automation domain
    // the user touched. Other AUTO domains stay adaptive.
    claim_user_control_domains(&before, config);

    let valid = config.validate().is_ok() && !config.name.trim().is_empty();
    let user_override = has_user_override(config);
    ui.add_space(8.0);
    if live_edits && user_override {
        ui.colored_label(theme::status_ok(), "LIVE / USER OVERRIDE");
    }
    ui.label(
        RichText::new(if live_edits {
            "Permanent live editing is ON: changes update the running processed mic immediately. Manual controls override only their VoicePilot domain."
        } else {
            "Permanent DSP is reconnecting automatically; your settings remain saved and will resume as soon as the microphone source returns."
        })
        .small()
        .color(theme::text_secondary()),
    );
    ui.horizontal(|ui| {
        if ui
            .add_enabled(valid, egui::Button::new("Save preset"))
            .clicked()
        {
            action = Some(MicControl::Save(config.clone()));
        }
        if ui
            .add_enabled(
                valid,
                egui::Button::new("Reattach DSP now").fill(theme::accent()),
            )
            .clicked()
        {
            action = Some(MicControl::Apply(config.clone()));
        }
    });
    if !valid {
        ui.colored_label(
            theme::status_warn(),
            "One or more DSP values are outside the safe range.",
        );
    }

    if live_edits && action.is_none() && valid && *config != before {
        action = Some(MicControl::LiveUpdate(config.clone()));
    }
    action
}

fn claim_manual(mode: &mut VoicePilotMode, changed: bool) {
    if changed && matches!(*mode, VoicePilotMode::Auto) {
        *mode = VoicePilotMode::Manual;
    }
}

fn claim_user_control_domains(before: &MicrophoneDspConfig, config: &mut MicrophoneDspConfig) {
    claim_manual(
        &mut config.voicepilot.auto_noise,
        before.noise_suppression != config.noise_suppression
            || before.click_suppression != config.click_suppression,
    );
    claim_manual(
        &mut config.voicepilot.auto_aec,
        before.echo_cancellation != config.echo_cancellation
            || before.playback_rejection != config.playback_rejection,
    );
    claim_manual(
        &mut config.voicepilot.auto_eq,
        before.eq != config.eq
            || before.multiband_eq != config.multiband_eq
            || before.dynamic_eq != config.dynamic_eq
            || before.voice_enhancer != config.voice_enhancer,
    );
    claim_manual(&mut config.voicepilot.auto_tone, before.tone != config.tone);
    claim_manual(
        &mut config.voicepilot.auto_dynamics,
        before.gate != config.gate
            || before.compressor != config.compressor
            || before.multiband_compressor != config.multiband_compressor
            || before.saturation != config.saturation,
    );
    claim_manual(
        &mut config.voicepilot.auto_de_esser,
        before.de_esser != config.de_esser,
    );
    claim_manual(
        &mut config.voicepilot.auto_loudness,
        before.input_gain_db != config.input_gain_db
            || before.output_gain_db != config.output_gain_db
            || before.limiter != config.limiter,
    );
}

fn has_user_override(config: &MicrophoneDspConfig) -> bool {
    [
        config.voicepilot.auto_noise,
        config.voicepilot.auto_aec,
        config.voicepilot.auto_eq,
        config.voicepilot.auto_tone,
        config.voicepilot.auto_dynamics,
        config.voicepilot.auto_de_esser,
        config.voicepilot.auto_loudness,
    ]
    .into_iter()
    .any(|mode| matches!(mode, VoicePilotMode::Manual | VoicePilotMode::Locked))
}

fn noise_state_label(state: NoiseAdaptationState) -> &'static str {
    match state {
        NoiseAdaptationState::Learning => "LEARNING",
        NoiseAdaptationState::Holding => "HOLDING",
        NoiseAdaptationState::Suppressing => "SUPPRESSING",
        NoiseAdaptationState::SpeechProtected => "SPEECH PROTECTED",
        NoiseAdaptationState::ReferenceUnavailable => "REFERENCE UNAVAILABLE",
    }
}

fn target_label(target: VoicePilotTarget) -> &'static str {
    match target {
        VoicePilotTarget::Natural => "Natural",
        VoicePilotTarget::Clear => "Clear Voice",
        VoicePilotTarget::Warm => "Warm",
        VoicePilotTarget::Full => "Full",
        VoicePilotTarget::Broadcast => "Broadcast",
        VoicePilotTarget::BroadcastFull => "Broadcast Full",
        VoicePilotTarget::DeepAndFull => "Deep & Full",
        VoicePilotTarget::CrispStream => "Crisp Stream",
        VoicePilotTarget::VoiceChat => "Voice Chat",
        VoicePilotTarget::Custom => "Custom",
    }
}

fn automation_mode(ui: &mut egui::Ui, label: &str, mode: &mut VoicePilotMode) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(("voicepilot_mode", label))
            .selected_text(mode_label(*mode))
            .show_ui(ui, |ui| {
                for candidate in [
                    VoicePilotMode::Auto,
                    VoicePilotMode::Manual,
                    VoicePilotMode::Locked,
                    VoicePilotMode::Bypassed,
                ] {
                    ui.selectable_value(mode, candidate, mode_label(candidate));
                }
            });
    });
}

fn mode_label(mode: VoicePilotMode) -> &'static str {
    match mode {
        VoicePilotMode::Auto => "AUTO",
        VoicePilotMode::Manual => "MANUAL",
        VoicePilotMode::Locked => "LOCKED",
        VoicePilotMode::Bypassed => "BYPASSED",
    }
}

fn eq_kind(ui: &mut egui::Ui, index: usize, kind: &mut MicEqFilterKind) {
    let label = match *kind {
        MicEqFilterKind::Bell => "Bell",
        MicEqFilterKind::LowShelf => "Low Shelf",
        MicEqFilterKind::HighShelf => "High Shelf",
        MicEqFilterKind::HighPass => "High Pass",
        MicEqFilterKind::LowPass => "Low Pass",
        MicEqFilterKind::Notch => "Notch",
    };
    egui::ComboBox::from_id_salt(("mic_eq_kind", index))
        .selected_text(label)
        .show_ui(ui, |ui| {
            for (candidate, text) in [
                (MicEqFilterKind::Bell, "Bell"),
                (MicEqFilterKind::LowShelf, "Low Shelf"),
                (MicEqFilterKind::HighShelf, "High Shelf"),
                (MicEqFilterKind::HighPass, "High Pass"),
                (MicEqFilterKind::LowPass, "Low Pass"),
                (MicEqFilterKind::Notch, "Notch"),
            ] {
                ui.selectable_value(kind, candidate, text);
            }
        });
}

#[cfg(test)]
mod live_control_tests {
    use super::*;

    #[test]
    fn tone_edit_claims_only_tone_domain() {
        let before = MicrophoneDspConfig::default();
        let mut edited = before.clone();
        edited.tone.bass_boost_db += 1.0;
        claim_user_control_domains(&before, &mut edited);
        assert_eq!(edited.voicepilot.auto_tone, VoicePilotMode::Manual);
        assert_eq!(edited.voicepilot.auto_noise, VoicePilotMode::Auto);
        assert_eq!(edited.voicepilot.auto_aec, VoicePilotMode::Auto);
        assert_eq!(edited.voicepilot.auto_eq, VoicePilotMode::Auto);
        assert_eq!(edited.voicepilot.auto_dynamics, VoicePilotMode::Auto);
        assert_eq!(edited.voicepilot.auto_de_esser, VoicePilotMode::Auto);
        assert_eq!(edited.voicepilot.auto_loudness, VoicePilotMode::Auto);
    }

    #[test]
    fn explicit_auto_selection_is_preserved() {
        let mut before = MicrophoneDspConfig::default();
        before.voicepilot.auto_tone = VoicePilotMode::Manual;
        let mut edited = before.clone();
        edited.voicepilot.auto_tone = VoicePilotMode::Auto;
        claim_user_control_domains(&before, &mut edited);
        assert_eq!(edited.voicepilot.auto_tone, VoicePilotMode::Auto);
    }

    #[test]
    fn voice_isolation_edit_does_not_claim_voicepilot_domain() {
        let before = MicrophoneDspConfig::default();
        let mut edited = before.clone();
        edited.speaker_lock.match_threshold = 0.95;
        claim_user_control_domains(&before, &mut edited);
        assert!(!has_user_override(&edited));
    }

    #[test]
    fn noise_monitor_sensitivity_edit_does_not_claim_voicepilot_domain() {
        let before = MicrophoneDspConfig::default();
        let mut edited = before.clone();
        edited.extraneous_noise_monitor.sensitivity_percent = 92.0;
        claim_user_control_domains(&before, &mut edited);
        assert!(!has_user_override(&edited));
    }

    #[test]
    fn playback_rejection_edit_claims_only_aec_domain() {
        let before = MicrophoneDspConfig::default();
        let mut edited = before.clone();
        edited.playback_rejection.correlation_threshold = 0.70;
        claim_user_control_domains(&before, &mut edited);
        assert_eq!(edited.voicepilot.auto_aec, VoicePilotMode::Manual);
        assert_eq!(edited.voicepilot.auto_noise, VoicePilotMode::Auto);
    }

    #[test]
    fn auto_adapt_off_is_preserved_without_claiming_voicepilot_domain() {
        let before = MicrophoneDspConfig::default();
        let mut edited = before.clone();
        edited.extraneous_noise_monitor.auto_adapt = false;
        claim_user_control_domains(&before, &mut edited);
        assert!(!edited.extraneous_noise_monitor.auto_adapt);
        assert!(!has_user_override(&edited));
    }
}
