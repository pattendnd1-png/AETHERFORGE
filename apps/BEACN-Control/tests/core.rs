use aetherforge_beacn_control::{pipewire, profile};

#[test]
fn parses_pipewire_nodes_from_wpctl_status() {
    let sample = r#"
Audio
 ├─ Devices:
 │      51. Built-in Audio                      [alsa]
 ├─ Sinks:
 │  *   67. Speakers                            [vol: 0.70]
 │      81. BEACN Headphones                    [vol: 0.55]
 ├─ Sources:
 │  *   75. BEACN Mic                           [vol: 0.82]
 │      76. Webcam Microphone                    [vol: 1.00]
 ├─ Filters:
"#;
    let graph = pipewire::parse_status(sample);
    assert_eq!(graph.sources.len(), 2);
    assert_eq!(graph.sources[0].id, "75");
    assert_eq!(graph.sources[0].name, "BEACN Mic");
    assert!(graph.sources[0].is_default);
    assert_eq!(graph.sinks.len(), 2);
    assert_eq!(graph.sinks[1].name, "BEACN Headphones");
}

#[test]
fn profile_round_trip_preserves_controls() {
    let mut original = profile::Profile {
        name: "Streaming".into(),
        source_name: "BEACN Mic".into(),
        source_volume: 0.86,
        source_muted: false,
        sink_name: "BEACN Headphones".into(),
        sink_volume: 0.55,
        sink_muted: false,
        dsp: aetherforge_beacn_control::software_dsp::SoftwareDspState::default(),
    };
    original.dsp.de_esser.enabled = true;
    original.dsp.de_esser.amount = 62.0;
    original.dsp.exciter.enabled = true;
    original.dsp.headphones.eq_left[9].gain_db = 4.5;
    original.dsp.headphones.eq_right[9].gain_db = -2.5;
    original.dsp.headphones.eq_linked = false;
    original.dsp.headphones.binaural_personalization = true;
    let text = profile::encode(&original);
    let decoded = profile::decode(&text).expect("profile should decode");
    assert_eq!(decoded, original);
}

#[test]
fn legacy_profile_without_dsp_decodes_with_safe_defaults() {
    let legacy = "name=Legacy\nsource_name=BEACN Mic\nsource_volume=0.900\nsource_muted=false\nsink_name=BEACN Headphones\nsink_volume=0.500\nsink_muted=false\n";
    let decoded = profile::decode(legacy).expect("legacy profile should decode");
    assert_eq!(decoded.dsp.headphones.eq_left.len(), 10);
    assert_eq!(decoded.dsp.de_esser.amount, 35.0);
}

#[test]
fn volume_is_clamped_to_safe_ui_range() {
    assert!((pipewire::clamp_volume(-0.5) - 0.0).abs() < f32::EPSILON);
    assert!((pipewire::clamp_volume(0.75) - 0.75).abs() < f32::EPSILON);
    assert!((pipewire::clamp_volume(3.0) - 1.5).abs() < f32::EPSILON);
}

#[test]
fn responsive_layout_breakpoints_reflow_without_fixed_desktop_width() {
    use aetherforge_beacn_control::layout::ResponsiveLayout;

    assert_eq!(
        ResponsiveLayout::from_width(640.0),
        ResponsiveLayout::Compact
    );
    assert_eq!(
        ResponsiveLayout::from_width(900.0),
        ResponsiveLayout::Standard
    );
    assert_eq!(ResponsiveLayout::from_width(1440.0), ResponsiveLayout::Wide);
    assert!(!ResponsiveLayout::Compact.uses_device_column());
    assert!(ResponsiveLayout::Standard.uses_device_column());
    assert!(ResponsiveLayout::Wide.uses_output_column());
}

#[test]
fn ten_second_recorder_targets_selected_pipewire_source() {
    use aetherforge_beacn_control::recorder;
    use std::path::Path;

    assert_eq!(
        recorder::record_arguments("75", Path::new("/tmp/mic-test.wav")),
        vec![
            "10s".to_owned(),
            "pw-record".to_owned(),
            "--target".to_owned(),
            "75".to_owned(),
            "/tmp/mic-test.wav".to_owned(),
        ]
    );
}

#[test]
fn hardware_dsp_values_are_clamped_before_usb_writes() {
    use aetherforge_beacn_control::hardware;

    assert_eq!(hardware::clamp_mic_gain(0), 3);
    assert_eq!(hardware::clamp_mic_gain(99), 20);
    assert_eq!(hardware::clamp_mic_gain(12), 12);
    assert_eq!(hardware::clamp_compressor_threshold(-100.0), -40.0);
    assert_eq!(hardware::clamp_compressor_threshold(8.0), 0.0);
    assert_eq!(hardware::clamp_expander_threshold(-100.0), -90.0);
    assert_eq!(hardware::clamp_suppressor_amount(125.0), 100.0);
}

#[test]
fn hardware_dsp_defaults_stay_inside_beacn_ranges() {
    use aetherforge_beacn_control::hardware::HardwareState;

    let state = HardwareState::default();
    assert!((3..=20).contains(&state.mic_gain));
    assert!((-40.0..=0.0).contains(&state.compressor_threshold));
    assert!((1.0..=16.0).contains(&state.compressor_ratio));
    assert!((-90.0..=0.0).contains(&state.expander_threshold));
    assert!((1.0..=10.0).contains(&state.expander_ratio));
    assert!((0.0..=100.0).contains(&state.suppressor_amount));
}

#[test]
fn does_not_fall_through_to_another_source_when_selected_node_disappears() {
    use aetherforge_beacn_control::pipewire::AudioNode;

    let remaining = vec![
        AudioNode {
            id: "84".into(),
            name: "Brio 100".into(),
            is_default: true,
        },
        AudioNode {
            id: "63".into(),
            name: "Video Capture".into(),
            is_default: false,
        },
    ];

    let selected = pipewire::reselect_node(&remaining, Some("BEACN Mic"), true);
    assert_eq!(selected.as_deref(), Some("BEACN Mic"));
    assert!(pipewire::selected_by_name(&remaining, selected.as_deref()).is_none());

    let initial = pipewire::reselect_node(&remaining, None, true);
    assert_eq!(initial, None);
}

#[test]
fn reports_usb_present_but_beacn_pipewire_nodes_missing() {
    use aetherforge_beacn_control::pipewire::{AudioGraph, AudioNode, BeacnAudioHealth};

    let graph = AudioGraph {
        devices: vec![AudioNode {
            id: "108".into(),
            name: "alsa_card.usb-BEACN_BEACN_Mic_0011240700359B-00".into(),
            is_default: false,
        }],
        sources: vec![AudioNode {
            id: "84".into(),
            name: "Brio 100".into(),
            is_default: true,
        }],
        sinks: vec![AudioNode {
            id: "91".into(),
            name: "HDMI".into(),
            is_default: true,
        }],
        raw_status: String::new(),
    };

    assert_eq!(
        pipewire::beacn_audio_health(&graph, true),
        BeacnAudioHealth::NodesMissing
    );
}

#[test]
fn hardware_eq_values_are_clamped_before_usb_writes() {
    use aetherforge_beacn_control::hardware;

    assert_eq!(hardware::clamp_eq_gain(-99.0), -12.0);
    assert_eq!(hardware::clamp_eq_gain(99.0), 12.0);
    assert_eq!(hardware::clamp_eq_frequency(2.0), 20.0);
    assert_eq!(hardware::clamp_eq_frequency(50_000.0), 20_000.0);
    assert_eq!(hardware::clamp_eq_q(0.01), 0.1);
    assert_eq!(hardware::clamp_eq_q(99.0), 10.0);
    assert_eq!(hardware::clamp_headphone_balance(-500), -100);
    assert_eq!(hardware::clamp_headphone_balance(500), 100);
}

#[test]
fn hardware_defaults_cover_nine_band_mic_and_headphone_eq() {
    use aetherforge_beacn_control::hardware::HardwareState;

    let state = HardwareState::default();
    assert_eq!(state.mic_eq.len(), 9);
    assert_eq!(state.headphone_eq_left.len(), 9);
    assert_eq!(state.headphone_eq_right.len(), 9);
    assert!(
        state
            .mic_eq
            .iter()
            .all(|band| (20.0..=20_000.0).contains(&band.frequency_hz))
    );
}

#[test]
fn beacn_output_recovery_prefers_available_duplex_profile() {
    let sample = r#"
Card #106
    Name: alsa_card.usb-BEACN_BEACN_Mic_0011240700359B-00
    Profiles:
        input:analog-stereo: Analog Stereo Input (sinks: 0, sources: 1, priority: 65, available: yes)
        output:analog-stereo: Analog Stereo Output (sinks: 1, sources: 0, priority: 6500, available: yes)
        output:analog-stereo+input:analog-stereo: Analog Stereo Duplex (sinks: 1, sources: 1, priority: 6565, available: yes)
    Active Profile: input:analog-stereo
"#;
    let cards = pipewire::parse_pactl_cards(sample);
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(card.active_profile.as_deref(), Some("input:analog-stereo"));
    let profile = pipewire::preferred_output_profile(card).expect("duplex profile should exist");
    assert_eq!(profile.name, "output:analog-stereo+input:analog-stereo");
    assert_eq!(profile.sinks, 1);
    assert_eq!(profile.sources, 1);
}

#[test]
fn graphical_session_environment_parser_keeps_only_launch_context() {
    use aetherforge_beacn_control::session;

    let parsed = session::parse_graphical_environment(
        "WAYLAND_DISPLAY=wayland-0\nXDG_RUNTIME_DIR=/run/user/1000\nDBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus\nUNRELATED_SECRET=do-not-copy\n",
    );

    assert_eq!(
        parsed.get("WAYLAND_DISPLAY").map(String::as_str),
        Some("wayland-0")
    );
    assert_eq!(
        parsed.get("XDG_RUNTIME_DIR").map(String::as_str),
        Some("/run/user/1000")
    );
    assert!(!parsed.contains_key("UNRELATED_SECRET"));
    assert!(session::graphical_environment_is_usable(&parsed));
}

#[test]
fn graphical_session_requires_wayland_or_x11_display() {
    use aetherforge_beacn_control::session;
    use std::collections::BTreeMap;

    let mut values = BTreeMap::new();
    values.insert("WAYLAND_DISPLAY".to_owned(), "wayland-0".to_owned());
    assert!(!session::graphical_environment_is_usable(&values));

    values.insert("XDG_RUNTIME_DIR".to_owned(), "/run/user/1000".to_owned());
    assert!(session::graphical_environment_is_usable(&values));

    values.clear();
    values.insert("DISPLAY".to_owned(), ":0".to_owned());
    assert!(session::graphical_environment_is_usable(&values));
}

#[test]
fn protected_pass_through_blocks_direct_usb_claims() {
    use aetherforge_beacn_control::hardware::{
        DirectUsbControlPolicy, direct_usb_claims_allowed, direct_usb_control_policy,
    };

    assert_eq!(
        direct_usb_control_policy(),
        DirectUsbControlPolicy::BlockedToPreserveSystemAudio
    );
    assert!(!direct_usb_claims_allowed());
}

#[test]
fn protected_pass_through_keeps_controller_disconnected() {
    use aetherforge_beacn_control::hardware::HardwareController;

    let controller = HardwareController::connect();
    assert!(controller.is_err());
    let error = controller
        .err()
        .expect("protected mode must return a reason");
    assert!(error.contains("Protected pass-through"));
}

#[test]
fn software_dsp_defaults_match_windows_style_workstation_shape() {
    use aetherforge_beacn_control::software_dsp::{
        DspBackendState, HEADPHONE_EQ_BAND_COUNT, MIC_EQ_BAND_COUNT, SoftwareDspState,
    };

    let state = SoftwareDspState::default();
    assert_eq!(state.mic_eq.len(), MIC_EQ_BAND_COUNT);
    assert_eq!(state.headphones.eq_left.len(), HEADPHONE_EQ_BAND_COUNT);
    assert_eq!(state.headphones.eq_right.len(), 10);
    assert_eq!(DspBackendState::default(), DspBackendState::Unavailable);
    assert!(!DspBackendState::Unavailable.can_dispatch());
}

#[test]
fn software_dsp_linked_headphone_eq_mirrors_selected_ear() {
    use aetherforge_beacn_control::software_dsp::SoftwareDspState;

    let mut state = SoftwareDspState::default();
    let mut band = state.headphones.eq_left[3];
    band.gain_db = 7.5;
    band.frequency_hz = 333.0;
    state.set_headphone_band(true, 3, band).expect("band edit");
    assert_eq!(state.headphones.eq_left[3], state.headphones.eq_right[3]);

    state.set_headphones_linked(false, true);
    let mut right = state.headphones.eq_right[3];
    right.gain_db = -5.0;
    state
        .set_headphone_band(false, 3, right)
        .expect("right edit");
    assert_ne!(state.headphones.eq_left[3], state.headphones.eq_right[3]);

    state.set_headphones_linked(true, false);
    assert_eq!(state.headphones.eq_left, state.headphones.eq_right);
}

#[test]
fn software_dsp_sanitize_clamps_profile_parameters() {
    use aetherforge_beacn_control::software_dsp::SoftwareDspState;

    let mut state = SoftwareDspState::default();
    state.mic_gain_db = 99.0;
    state.de_esser.frequency_hz = 99_000.0;
    state.exciter.amount = -4.0;
    state.headphones.balance = 700;
    state.headphones.eq_left[0].q = 0.001;
    state.sanitize();
    assert_eq!(state.mic_gain_db, 24.0);
    assert_eq!(state.de_esser.frequency_hz, 12_000.0);
    assert_eq!(state.exciter.amount, 0.0);
    assert_eq!(state.headphones.balance, 100);
    assert_eq!(state.headphones.eq_left[0].q, 0.1);
}
