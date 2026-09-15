# ForgeHX 10.0.28 Extraneous Noise Monitor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an always-on Extraneous Noise Monitor that classifies environmental and speaker-leak noise in real time and safely retunes the existing ForgeHX microphone DSP without changing routing or learning local speech as noise.

**Architecture:** Extend the existing 48 kHz mono / 10 ms `VoiceProcessingEngine` with two focused units: `noise_monitor.rs` for measurement/classification and `adaptive_noise.rs` for policy/bounded targets. The monitor consumes the raw mic frame, post-Sonora cleaned frame, and the existing full-system `@DEFAULT_AUDIO_SINK@` render reference already captured by `runtime.rs`; telemetry is copied through the existing runtime registry into `MicrophoneDspState` and the Mic DSP GUI. No new audio graph, virtual source, thread, or transport is introduced.

**Tech Stack:** Rust 1.98-compatible workspace, PipeWire 0.10 direct capture, Sonora 0.2 AEC/NS, serde/serde_json, egui/eframe, Python static contract tests, Bash host verifier.

**Spec:** `docs/superpowers/specs/2026-09-06-forgehx-extraneous-noise-monitor-design.md`

## Global Constraints

- Baseline: **ForgeHX 10.0.27**; target canonical release: **ForgeHX 10.0.28**.
- Preserve physical endpoint exactly as **`HyperX SoloCast 2 Analog Stereo`**.
- Processed app-facing source remains **`aetherstream.system.microphone`**.
- AetherStream remains the system-wide audio graph owner and sole app-facing mic publisher.
- ForgeHX remains microphone DSP owner and creates **zero** additional app-facing microphone sources.
- Existing AFXHXM01 ForgeHX → AetherStream bridge remains unchanged.
- Existing full-system speaker reference remains `@DEFAULT_AUDIO_SINK@`; do not create a second render-reference path.
- Never add `wpctl set-default`, `pactl set-default-source`, or `pactl set-default-sink`.
- DSP frame errors remain fail-closed to silence; raw microphone fail-open is forbidden.
- `VoicePilotMode::{Auto,Manual,Locked,Bypassed}` remains authoritative for automatic noise/AEC changes.
- Speaker rejection must preserve local speech during double-talk.
- Clippy remains diagnostic-only/nonblocking; compile, tests, behavior, routing, bridge, and runtime DSP gates are blocking.
- Host verifier must write `~/Downloads/ForgeHX-10.0.28-VERIFY.txt` and parse `forgehx mic dsp get` as `{"reply":"mic_dsp_state","state":{...}}`.
- Do not bump again solely for style/lint warnings; a real post-10.0.28 functional fix becomes 10.0.29.

---

## File Structure

**Create**
- `crates/forgehx-dsp/src/noise_monitor.rs` — fixed-probe spectrum analysis, per-band floors, pink/white/fan/hum/whine/transient scores, render correlation, speech/double-talk guard, telemetry state.
- `crates/forgehx-dsp/src/adaptive_noise.rs` — converts monitor telemetry + user policy into rate-limited bounded DSP targets.
- `scripts/test-10.0.28-extraneous-noise-monitor.py` — static/source contract for new modules, routing invariants, telemetry plumbing, and no raw/default-route bypass.
- `scripts/test-10.0.28-runtime-state.py` — host JSON-shape/runtime telemetry contract helper.
- `scripts/test-10.0.28-packaging-version.py` — canonical version contract.
- `scripts/check-v10.0.28-source.py` — source-release contract.
- `scripts/verify-10.0.28.sh` — hard host gate.

**Modify**
- `crates/forgehx-core/src/lib.rs:449-478, 812-1020` — config defaults/validation and public telemetry schema.
- `crates/forgehx-dsp/src/lib.rs:1-15, MicDspManager::state/apply/live_update paths` — exports and runtime telemetry exposure.
- `crates/forgehx-dsp/src/engine.rs:1-170` — monitor/controller integration in 10 ms processing path and coarse Sonora target application.
- `crates/forgehx-dsp/src/background_rejection.rs:1-105` — bounded adaptive margin/rejection inputs while retaining scalar floor/speech grace behavior.
- `crates/forgehx-dsp/src/runtime.rs:37-126, run_worker` — add noise telemetry slot to the existing runtime handle only.
- `crates/forgehx-dsp/src/direct_pipewire.rs:21-29, 159-214` — copy engine noise telemetry after every processed frame; no new capture path.
- `crates/forgehx-gui/src/microphone.rs:1-185, 310-400` — Noise Environment controls/readouts and manual-domain ownership.
- `crates/forgehx-core/src/lib.rs:9` — IPC stays JSON-compatible; do **not** bump protocol solely for added serde-default state fields unless a new command is introduced.
- `Cargo.toml`, every workspace crate `Cargo.toml`, `PKGBUILD`, `README.md` — 10.0.28 version identity.
- `forgehx.install` only if needed to ensure post-install daemon restart remains present; do not add routing mutation.

---

### Task 1: Add Backward-Compatible Noise Monitor Config + Telemetry Schema

**Files:**
- Modify: `crates/forgehx-core/src/lib.rs:449-478,812-1020`
- Test: inline `#[cfg(test)]` module in `crates/forgehx-core/src/lib.rs`

**Interfaces:**
- Produces: `ExtraneousNoiseMonitorConfig`, `NoiseBandLevels`, `NoiseClassScores`, `NoiseAdaptationState`, `NoiseSceneTelemetry`.
- Extends: `MicrophoneDspConfig::extraneous_noise_monitor` and `MicrophoneDspState::noise_scene_telemetry`.
- Consumes: existing `VoicePilotMode` only indirectly; no new automation enum.

- [ ] **Step 1: Write the failing backward-compatibility/default tests**

Add tests that deserialize a pre-10.0.28 profile with no monitor block and assert synthesized defaults:

```rust
#[test]
fn legacy_microphone_profile_gets_safe_noise_monitor_defaults() {
    let legacy = r#"{
      "name":"Broadcast Full","enabled":true,"input_gain_db":0.0,
      "high_pass_hz":80.0,"output_gain_db":0.0
    }"#;
    let cfg: MicrophoneDspConfig = serde_json::from_str(legacy).unwrap();
    assert!(cfg.extraneous_noise_monitor.enabled);
    assert!(cfg.extraneous_noise_monitor.auto_adapt);
    assert_eq!(cfg.extraneous_noise_monitor.sensitivity_percent, 85.0);
    assert_eq!(cfg.extraneous_noise_monitor.max_adaptive_suppression_db, 72.0);
    assert!(cfg.extraneous_noise_monitor.speaker_rejection_enabled);
}

#[test]
fn noise_monitor_limits_are_validated() {
    let mut cfg = MicrophoneDspConfig::default();
    cfg.extraneous_noise_monitor.max_adaptive_suppression_db = 72.1;
    assert!(cfg.validate().is_err());
    cfg.extraneous_noise_monitor.max_adaptive_suppression_db = 72.0;
    cfg.extraneous_noise_monitor.sensitivity_percent = 100.0;
    assert!(cfg.validate().is_ok());
}
```

- [ ] **Step 2: Run the focused core tests and verify RED**

Run:

```bash
cargo test -p forgehx-core legacy_microphone_profile_gets_safe_noise_monitor_defaults noise_monitor_limits_are_validated
```

Expected: compile FAIL because `extraneous_noise_monitor` and its types do not yet exist.

- [ ] **Step 3: Add the config and telemetry types with exact defaults**

Insert before `MicrophoneDspConfig`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtraneousNoiseMonitorConfig {
    pub enabled: bool,
    pub auto_adapt: bool,
    pub sensitivity_percent: f32,
    pub max_adaptive_suppression_db: f32,
    pub speaker_rejection_enabled: bool,
}

impl Default for ExtraneousNoiseMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_adapt: true,
            sensitivity_percent: 85.0,
            max_adaptive_suppression_db: 72.0,
            speaker_rejection_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NoiseAdaptationState {
    #[default] Learning,
    Holding,
    Suppressing,
    SpeechProtected,
    ReferenceUnavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseBandLevels {
    pub sub_rumble_dbfs: f32,
    pub low_dbfs: f32,
    pub low_mid_dbfs: f32,
    pub mid_dbfs: f32,
    pub presence_dbfs: f32,
    pub high_dbfs: f32,
    pub air_dbfs: f32,
}

impl Default for NoiseBandLevels {
    fn default() -> Self {
        Self {
            sub_rumble_dbfs: -90.0, low_dbfs: -90.0, low_mid_dbfs: -90.0,
            mid_dbfs: -90.0, presence_dbfs: -90.0, high_dbfs: -90.0, air_dbfs: -90.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NoiseClassScores {
    pub white_like: f32,
    pub pink_like: f32,
    pub broadband: f32,
    pub hum_rumble: f32,
    pub narrowband_whine: f32,
    pub transient: f32,
    pub speaker_leak: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoiseSceneTelemetry {
    pub active: bool,
    pub reference_available: bool,
    pub adaptation_state: NoiseAdaptationState,
    pub overall_noise_dbfs: f32,
    pub classes: NoiseClassScores,
    pub band_floor_dbfs: NoiseBandLevels,
    pub dominant_bands: Vec<String>,
    pub adaptive_suppression_db: f32,
    pub sonora_noise_target_percent: f32,
    pub speech_protected: bool,
    pub double_talk: bool,
}

impl Default for NoiseSceneTelemetry {
    fn default() -> Self {
        Self {
            active: false,
            reference_available: false,
            adaptation_state: NoiseAdaptationState::ReferenceUnavailable,
            overall_noise_dbfs: -90.0,
            classes: NoiseClassScores::default(),
            band_floor_dbfs: NoiseBandLevels::default(),
            dominant_bands: Vec::new(),
            adaptive_suppression_db: 0.0,
            sonora_noise_target_percent: 0.0,
            speech_protected: false,
            double_talk: false,
        }
    }
}
```

Add to `MicrophoneDspConfig`:

```rust
#[serde(default)]
pub extraneous_noise_monitor: ExtraneousNoiseMonitorConfig,
```

Add `extraneous_noise_monitor: ExtraneousNoiseMonitorConfig::default()` to `Default`.

Add validation:

```rust
if !(0.0..=100.0).contains(&self.extraneous_noise_monitor.sensitivity_percent)
    || !(0.0..=72.0).contains(&self.extraneous_noise_monitor.max_adaptive_suppression_db)
{
    return Err(ForgeHxError::InvalidProfile(
        "extraneous noise monitor parameters are outside safe limits".into(),
    ));
}
```

Add to `MicrophoneDspState`:

```rust
#[serde(default)]
pub noise_scene_telemetry: NoiseSceneTelemetry,
```

- [ ] **Step 4: Run core tests and serialization compatibility tests**

Run:

```bash
cargo test -p forgehx-core
```

Expected: all `forgehx-core` tests PASS; old JSON profiles deserialize with defaults.

- [ ] **Step 5: Commit checkpoint if executing inside a Git worktree**

```bash
git add crates/forgehx-core/src/lib.rs
git commit -m "feat(forgehx): add extraneous noise config and telemetry schema"
```

---

### Task 2: Implement Deterministic Realtime Noise Scene Analysis

**Files:**
- Create: `crates/forgehx-dsp/src/noise_monitor.rs`
- Modify: `crates/forgehx-dsp/src/lib.rs:1-15`
- Test: `crates/forgehx-dsp/src/noise_monitor.rs` unit tests

**Interfaces:**
- Consumes: `&[f32; 480]` raw capture, `&[f32; 480]` post-Sonora cleaned capture, `Option<&[f32; 480]>` render reference, `&ExtraneousNoiseMonitorConfig`.
- Produces: `NoiseObservation { telemetry: NoiseSceneTelemetry, stable_non_speech: bool }`.
- Public engine-owned type: `ExtraneousNoiseMonitor` with `analyze(...) -> NoiseObservation`.

- [ ] **Step 1: Write RED tests for white, pink, hum, whine, transient, and stable broadband separation**

Use deterministic pseudo-random fixtures, never `thread_rng()`. Include helpers:

```rust
fn white_noise(seed: u32, amplitude: f32) -> [f32; FRAME_SAMPLES] { /* xorshift32 */ }
fn pink_noise(seed: u32, amplitude: f32) -> [f32; FRAME_SAMPLES] { /* deterministic 1/f approximation */ }
fn sine(hz: f32, amplitude: f32) -> [f32; FRAME_SAMPLES] { /* SAMPLE_RATE */ }
```

Required assertions:

```rust
#[test]
fn white_fixture_scores_more_white_than_pink() {
    let mut monitor = ExtraneousNoiseMonitor::default();
    let cfg = ExtraneousNoiseMonitorConfig::default();
    let frame = white_noise(0x12345678, 0.03);
    let t = run_stable(&mut monitor, &frame, None, &cfg, 120);
    assert!(t.classes.white_like > 0.65, "{:#?}", t.classes);
    assert!(t.classes.white_like > t.classes.pink_like + 0.15);
}

#[test]
fn pink_fixture_scores_more_pink_than_white() { /* pink > .65 and > white + .15 */ }
#[test]
fn sixty_hz_hum_is_detected_without_becoming_broadband() { /* hum_rumble > .70 */ }
#[test]
fn persistent_6khz_whine_is_detected() { /* narrowband_whine > .70 */ }
#[test]
fn click_is_transient_and_does_not_raise_floor_after_one_frame() { /* transient > .75; floor unchanged */ }
#[test]
fn steady_shaped_fan_noise_becomes_broadband_scene() { /* broadband > .60 after stability */ }
```

- [ ] **Step 2: Run the tests and verify RED**

```bash
cargo test -p forgehx-dsp noise_monitor::tests -- --nocapture
```

Expected: compile FAIL because module/types do not exist.

- [ ] **Step 3: Implement a fixed-probe Goertzel analyzer**

Use no new crate. Define a fixed probe bank adequate for broad bands + tonal detection:

```rust
const PROBE_HZ: &[f32] = &[
    50.0, 60.0, 100.0, 120.0, 180.0, 240.0,
    320.0, 450.0, 630.0, 900.0, 1250.0, 1800.0,
    2500.0, 3500.0, 5000.0, 6500.0, 8000.0, 10000.0,
    12500.0, 15000.0, 18000.0,
];

fn goertzel_power(frame: &[f32; FRAME_SAMPLES], hz: f32) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * hz / SAMPLE_RATE;
    let coeff = 2.0 * omega.cos();
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &x in frame {
        let s0 = x + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(1.0e-12)
}
```

Aggregate broad-band dBFS values into `NoiseBandLevels`. Compute spectral slope with linear regression of probe `log2(hz)` against `10*log10(power)`. Map slope likeness with bounded triangular/Gaussian-style scores centered at 0 dB/oct (white) and -3 dB/oct (pink), suppressing both when tonal/transient evidence is strong.

- [ ] **Step 4: Add stable per-band floor learning**

State in `ExtraneousNoiseMonitor`:

```rust
band_floor_dbfs: NoiseBandLevels,
last_band_dbfs: NoiseBandLevels,
stable_frames: u32,
last_overall_dbfs: f32,
last_probe_db: Vec<f32>,
render_history: VecDeque<[f32; FRAME_SAMPLES]>,
telemetry: NoiseSceneTelemetry,
```

Use these exact floor rules:
- downward alpha `0.18`;
- upward alpha `0.025` only after >= 30 stable non-speech frames (~300 ms);
- band stability tolerance `2.0 dB`;
- never learn upward while `speech_protected`, `double_talk`, or transient score >= `0.60`;
- clamp all floors to `-90..=-12 dBFS`.

- [ ] **Step 5: Run noise-scene tests and tune only classifier constants**

```bash
cargo test -p forgehx-dsp noise_monitor::tests -- --nocapture
```

Expected: all Task 2 tests PASS with deterministic scores; no timing/sleep-based tests.

- [ ] **Step 6: Export the module**

In `crates/forgehx-dsp/src/lib.rs`:

```rust
pub mod noise_monitor;
pub use noise_monitor::{ExtraneousNoiseMonitor, NoiseObservation};
```

- [ ] **Step 7: Commit checkpoint if Git metadata exists**

```bash
git add crates/forgehx-dsp/src/noise_monitor.rs crates/forgehx-dsp/src/lib.rs
git commit -m "feat(forgehx): classify realtime extraneous noise scenes"
```

---

### Task 3: Add Speaker-Leak Correlation and Double-Talk Speech Protection

**Files:**
- Modify: `crates/forgehx-dsp/src/noise_monitor.rs`
- Test: same module

**Interfaces:**
- Extends `ExtraneousNoiseMonitor::analyze` with render-reference history and speech/double-talk flags.
- Produces `telemetry.classes.speaker_leak`, `telemetry.reference_available`, `telemetry.speech_protected`, `telemetry.double_talk`, and adaptation state.

- [ ] **Step 1: Write RED speaker/double-talk tests**

```rust
#[test]
fn correlated_playback_scores_as_speaker_leak() {
    let mut monitor = ExtraneousNoiseMonitor::default();
    let cfg = ExtraneousNoiseMonitorConfig::default();
    let render = mixed_tones(&[(440.0, 0.03), (1300.0, 0.02), (4200.0, 0.01)]);
    let capture = scaled_plus_noise(&render, 0.65, 0.002);
    let t = run_pair_stable(&mut monitor, &capture, &capture, Some(&render), &cfg, 80);
    assert!(t.classes.speaker_leak > 0.70, "{:#?}", t.classes);
    assert!(!t.double_talk);
}

#[test]
fn local_speech_over_playback_enters_double_talk_protection() {
    // capture = correlated render leakage + independently varying speech-like signal
    // assert double_talk=true and adaptation_state=SpeechProtected
}

#[test]
fn uncorrelated_speech_is_not_classified_as_speaker_leak() {
    // assert speaker_leak < 0.35
}

#[test]
fn reference_loss_keeps_environment_monitoring_active() {
    // reference_available=false; environmental class scores still update
}
```

- [ ] **Step 2: Run tests and verify RED**

```bash
cargo test -p forgehx-dsp noise_monitor::tests::correlated_playback_scores_as_speaker_leak noise_monitor::tests::local_speech_over_playback_enters_double_talk_protection
```

Expected: FAIL until correlation/double-talk logic exists.

- [ ] **Step 3: Implement bounded render correlation using existing frames only**

Keep up to 6 render frames (60 ms) in `render_history`. For each analysis frame, compute normalized absolute sample correlation against the current and stored render frames and retain the best score:

```rust
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
```

Use post-AEC cleaned capture for **residual** leak score and retain existing `PlaybackLeakGuard` as the independent pre-AEC anti-loop guard.

- [ ] **Step 4: Implement speech/double-talk guard**

Speech protection must require **dynamic non-stationary local energy**, not just loudness:
- overall capture >= learned broad floor + 8 dB;
- transient score < 0.60;
- frame-to-frame spectral flux >= 0.10 **or** level delta >= 2.5 dB;
- local-speech evidence must remain true for 2 of 3 frames before `speech_protected=true`;
- hold protection 12 frames (120 ms) after it drops.

Double-talk is true when:
- render reference is available and render RMS > -55 dBFS;
- speech protection is true;
- residual speaker-leak score > 0.20.

During `speech_protected` or `double_talk`, freeze upward per-band floor learning and speaker-model learning.

- [ ] **Step 5: Run all monitor tests**

```bash
cargo test -p forgehx-dsp noise_monitor::tests -- --nocapture
```

Expected: correlated playback detected, uncorrelated speech preserved, double-talk protected, and reference loss graceful.

- [ ] **Step 6: Commit checkpoint if Git metadata exists**

```bash
git add crates/forgehx-dsp/src/noise_monitor.rs
git commit -m "feat(forgehx): protect speech while rejecting speaker leakage"
```

---

### Task 4: Implement Bounded Adaptive Noise Controller

**Files:**
- Create: `crates/forgehx-dsp/src/adaptive_noise.rs`
- Modify: `crates/forgehx-dsp/src/lib.rs`
- Test: `crates/forgehx-dsp/src/adaptive_noise.rs`

**Interfaces:**
- Consumes: `&NoiseSceneTelemetry`, `&MicrophoneDspConfig`, previous `AdaptiveNoiseTargets`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveNoiseTargets {
    pub sonora_noise_percent: f32,
    pub background_margin_offset_db: f32,
    pub rejection_cap_db: f32,
    pub residual_speaker_suppression_db: f32,
}
```

- [ ] **Step 1: Write RED policy/bounds tests**

Required tests:

```rust
#[test]
fn auto_noise_can_raise_cleanup_but_never_past_user_and_product_bounds() { /* <=100%, <=72dB */ }
#[test]
fn manual_noise_domain_returns_user_noise_strength_unchanged() { /* auto_noise=Manual */ }
#[test]
fn locked_noise_domain_returns_user_noise_strength_unchanged() { /* auto_noise=Locked */ }
#[test]
fn bypassed_noise_domain_never_enables_adaptive_noise_suppression() { /* zero internal adaptive contribution */ }
#[test]
fn auto_adapt_off_observes_but_returns_neutral_targets() { /* no adaptive changes */ }
#[test]
fn double_talk_caps_residual_speaker_suppression_to_six_db() { /* preserve local speech */ }
#[test]
fn playback_only_may_use_up_to_thirty_six_db_residual_suppression() { /* bounded */ }
#[test]
fn target_changes_are_rate_limited_between_frames() { /* no pumping */ }
```

- [ ] **Step 2: Run tests and verify RED**

```bash
cargo test -p forgehx-dsp adaptive_noise::tests -- --nocapture
```

Expected: compile FAIL before module exists.

- [ ] **Step 3: Implement exact target policy**

Use these bounds:
- `sonora_noise_percent`: base user strength through `100.0`, but only when `voicepilot.auto_noise == Auto` and monitor auto-adapt is on;
- background margin offset: `0.0..=8.0 dB`;
- rejection cap: `18.0..=config.extraneous_noise_monitor.max_adaptive_suppression_db.min(72.0)` expressed as positive attenuation dB;
- residual speaker suppression: `0.0..=36.0 dB`, capped at `6.0 dB` during double-talk/speech protection;
- no speaker suppression when `speaker_rejection_enabled=false`, reference unavailable, or `voicepilot.auto_aec` is `Manual`, `Locked`, or `Bypassed`;
- per-frame fine-target movement <= `1.5 dB` and <= `2 percentage points` noise strength;
- Sonora level itself still changes only at the existing coarse `NOISE_RECONFIGURE_FRAMES` cadence.

Use sensitivity as a `0.0..=1.0` multiplier on class confidence. A broad/fan/pink/white scene can raise margin/rejection; hum/rumble biases margin modestly; speaker-leak drives residual speaker suppression; transient score must not raise persistent suppression.

- [ ] **Step 4: Run controller tests**

```bash
cargo test -p forgehx-dsp adaptive_noise::tests -- --nocapture
```

Expected: all bounds/manual authority/double-talk tests PASS.

- [ ] **Step 5: Export module**

```rust
pub mod adaptive_noise;
pub use adaptive_noise::{AdaptiveNoiseController, AdaptiveNoiseTargets};
```

- [ ] **Step 6: Commit checkpoint if Git metadata exists**

```bash
git add crates/forgehx-dsp/src/adaptive_noise.rs crates/forgehx-dsp/src/lib.rs
git commit -m "feat(forgehx): add bounded adaptive noise controller"
```

---

### Task 5: Integrate Monitor + Controller into Existing 10 ms Voice Engine

**Files:**
- Modify: `crates/forgehx-dsp/src/engine.rs:1-170`
- Modify: `crates/forgehx-dsp/src/background_rejection.rs:1-105`
- Test: unit tests in both files

**Interfaces:**
- `VoiceProcessingEngine` gains `noise_monitor`, `adaptive_noise`, `noise_scene_telemetry`, and current `AdaptiveNoiseTargets`.
- `BackgroundRejector::process` becomes:

```rust
pub fn process(
    &mut self,
    frame: &mut [f32],
    config: &NoiseSuppressionConfig,
    margin_offset_db: f32,
    rejection_cap_db: f32,
)
```

- Engine exposes `pub fn noise_scene_telemetry(&self) -> NoiseSceneTelemetry`.

- [ ] **Step 1: Write RED engine tests for integrated behavior**

Add tests:
- `full_engine_adapts_to_stable_white_noise_without_muting_varying_speech`
- `full_engine_reduces_correlated_speaker_leak_more_than_uncorrelated_speech`
- `double_talk_preserves_local_voice_peak`
- `monitor_disabled_keeps_10_0_27_processing_behavior`
- `reference_missing_does_not_fail_capture_processing`
- `adaptive_targets_cannot_exceed_72_db_or_user_modes`

Use deterministic synthetic frames and run 100-200 10 ms iterations to allow stable learning.

- [ ] **Step 2: Run focused tests and verify RED**

```bash
cargo test -p forgehx-dsp engine::tests::full_engine_adapts_to_stable_white_noise_without_muting_varying_speech -- --nocapture
```

Expected: FAIL before engine integration.

- [ ] **Step 3: Integrate analysis after Sonora capture cleanup and before background rejection**

Preserve the existing AEC order:

```rust
if let Some(render) = render {
    // existing process_render_f32 first
}
self.apm.process_capture_f32(...)?;
self.click_suppressor.process(&mut cleaned);

let observation = self.noise_monitor.analyze(
    capture,
    &cleaned,
    render,
    &self.config.extraneous_noise_monitor,
);
let adaptive = self.adaptive_noise.update(
    &observation.telemetry,
    &self.config,
);
self.noise_scene_telemetry = observation.telemetry.clone();
self.noise_scene_telemetry.adaptive_suppression_db = adaptive.rejection_cap_db;
self.noise_scene_telemetry.sonora_noise_target_percent = adaptive.sonora_noise_percent;
```

Do not move the existing `process_render_f32` or `process_capture_f32` later in the chain.

- [ ] **Step 4: Add bounded residual speaker suppression before background rejection**

Only when monitor speaker rejection is enabled, reference exists, residual speaker score is nonzero, and controller target > 0:

```rust
let gain = 10.0f32.powf(-adaptive.residual_speaker_suppression_db / 20.0);
for sample in &mut cleaned {
    *sample *= gain;
}
```

This must never be a hard mute solely because playback exists. During double-talk target is <=6 dB by Task 4.

- [ ] **Step 5: Pass fine adaptive targets into BackgroundRejector**

Change threshold calculation:

```rust
let open_threshold_db = (self.noise_floor_db
    + decision_margin_db(config.strength_percent, config.vad_threshold_percent)
    + margin_offset_db.clamp(0.0, 8.0))
    .min(voice_open_limit_db(config.strength_percent));
```

Change rejection gain so the final attenuation magnitude cannot exceed `rejection_cap_db.clamp(18.0, 72.0)`. Retain existing 120 ms grace and smoothing.

- [ ] **Step 6: Merge Adaptive Noise target with existing VoicePilot noise target**

When `VoicePilotMode::Auto`, use:

```rust
let desired_noise_percent = targets
    .noise_strength_percent
    .max(adaptive.sonora_noise_percent)
    .clamp(self.config.noise_suppression.strength_percent, 100.0);
```

For Manual/Locked/Bypassed, Task 4 returns the unmodified/neutral target. Keep `NOISE_RECONFIGURE_FRAMES = 200`; do not reconfigure Sonora every 10 ms.

- [ ] **Step 7: Preserve failure behavior**

If monitor analysis returns a recoverable invalid observation, keep the last safe `noise_scene_telemetry` and neutralize **increases** in adaptive suppression for that frame. Do not convert monitor failure into a raw bypass. `process_10ms` core Sonora errors continue returning `Err`, and `direct_pipewire.rs` continues replacing that frame with silence.

- [ ] **Step 8: Run full DSP tests**

```bash
cargo test -p forgehx-dsp -- --nocapture
```

Expected: all existing 10.0.27 tests plus new monitor/controller/integration tests PASS.

- [ ] **Step 9: Commit checkpoint if Git metadata exists**

```bash
git add crates/forgehx-dsp/src/engine.rs crates/forgehx-dsp/src/background_rejection.rs
git commit -m "feat(forgehx): adapt live mic rejection to current noise scene"
```

---

### Task 6: Plumb Noise Telemetry Through the Existing Runtime/State Path

**Files:**
- Modify: `crates/forgehx-dsp/src/runtime.rs:37-126`
- Modify: `crates/forgehx-dsp/src/direct_pipewire.rs:21-29,159-214`
- Modify: `crates/forgehx-dsp/src/lib.rs` `MicDspManager::state/apply/live_update/ensure_always_on...`
- Test: runtime/unit tests in `forgehx-dsp`

**Interfaces:**
- `RuntimeHandle` gains `noise_scene_telemetry: Arc<Mutex<NoiseSceneTelemetry>>`.
- `DirectRuntimeShared` gains same slot.
- `DspRuntimeRegistry::noise_scene_telemetry(&self, key: &str) -> Option<NoiseSceneTelemetry>`.
- `MicrophoneDspState.noise_scene_telemetry` is populated from runtime or default.

- [ ] **Step 1: Write RED runtime telemetry test**

Construct/start a registry test helper or test the handle/state helper directly so `noise_scene_telemetry()` returns a copied value and defaults cleanly when runtime is absent.

- [ ] **Step 2: Run focused test and verify RED**

```bash
cargo test -p forgehx-dsp runtime::tests -- --nocapture
```

Expected: compile FAIL until slot/accessor exists.

- [ ] **Step 3: Add one telemetry slot to existing runtime handle**

At `DspRuntimeRegistry::start` initialize:

```rust
let noise_scene_telemetry = Arc::new(Mutex::new(NoiseSceneTelemetry::default()));
```

Pass it through `RuntimeWorkerShared` → `DirectRuntimeShared`. Do **not** create another worker or reference thread.

- [ ] **Step 4: Copy engine telemetry after each processed frame**

In `direct_pipewire::process_frame`, after existing VoicePilot/isolation telemetry copies:

```rust
if let Ok(mut telemetry) = state.shared.noise_scene_telemetry.lock() {
    *telemetry = state.engine.noise_scene_telemetry();
}
```

- [ ] **Step 5: Populate every `MicrophoneDspState` constructor**

Use:

```rust
noise_scene_telemetry: self.runtimes
    .noise_scene_telemetry(&key)
    .unwrap_or_default(),
```

No new IPC command is required: `Reply::MicDspState` already serializes the state as JSON, and the new field is serde-default/backward-compatible.

- [ ] **Step 6: Run workspace tests that cover DSP state/daemon/IPC**

```bash
cargo test -p forgehx-dsp -p forgehx-daemon -p forgehx-ipc -p forgehx-cli
```

Expected: PASS; `forgehx mic dsp get <device>` JSON contains `reply`, nested `state`, and nested `noise_scene_telemetry`.

- [ ] **Step 7: Commit checkpoint if Git metadata exists**

```bash
git add crates/forgehx-dsp/src/runtime.rs crates/forgehx-dsp/src/direct_pipewire.rs crates/forgehx-dsp/src/lib.rs
git commit -m "feat(forgehx): expose live noise-scene telemetry"
```

---

### Task 7: Add Mic DSP Noise Environment Controls and Live Telemetry UI

**Files:**
- Modify: `crates/forgehx-gui/src/microphone.rs:1-185,310-400`
- Test: `crates/forgehx-gui/src/microphone.rs` `live_control_tests`

**Interfaces:**
- Uses `config.extraneous_noise_monitor` through existing `MicControl::LiveUpdate` / `Save` behavior.
- Reads `state.noise_scene_telemetry`; no new GUI IPC method.

- [ ] **Step 1: Write RED GUI-domain ownership tests**

Add tests proving:
- changing monitor sensitivity does **not** force `VoicePilotMode::Manual` because it configures the monitor itself;
- manually editing `noise_suppression` still claims only `auto_noise` as today;
- manually editing `echo_cancellation` or playback rejection still claims `auto_aec` only;
- `auto_adapt=false` persists via normal live update.

- [ ] **Step 2: Run GUI tests and verify RED where new field expectations are missing**

```bash
cargo test -p forgehx-gui microphone::live_control_tests -- --nocapture
```

- [ ] **Step 3: Add `Noise Environment` group after Cleanup**

Controls:

```rust
ui.checkbox(&mut config.extraneous_noise_monitor.enabled, "Extraneous Noise Monitor");
ui.checkbox(&mut config.extraneous_noise_monitor.auto_adapt, "Auto Noise Adapt");
ui.add(egui::Slider::new(
    &mut config.extraneous_noise_monitor.sensitivity_percent, 0.0..=100.0
).text("Noise monitor sensitivity (%)"));
ui.add(egui::Slider::new(
    &mut config.extraneous_noise_monitor.max_adaptive_suppression_db, 0.0..=72.0
).text("Maximum adaptive rejection (dB)"));
ui.checkbox(
    &mut config.extraneous_noise_monitor.speaker_rejection_enabled,
    "Reject speaker/system playback leakage",
);
```

- [ ] **Step 4: Render live telemetry without raw samples**

Show:
- state label `LEARNING / HOLDING / SUPPRESSING / SPEECH PROTECTED / REFERENCE UNAVAILABLE`;
- overall dBFS;
- pink %, white %, broadband/fan %, hum/rumble %, whine %, transient %, speaker leak %;
- 7 broad-band floor values;
- dominant bands as joined text;
- adaptive suppression dB;
- Sonora target %;
- reference ACTIVE/UNAVAILABLE;
- speech protection and DOUBLE-TALK flags.

All scores display as `score * 100.0`, clamped 0..100. Use existing DragonGlass `theme::status_ok/status_warn/text_secondary` functions; do not introduce a new visual palette.

- [ ] **Step 5: Update descriptive copy**

Replace the stale `Processed source: ForgeHX Mic` label with the actual state source when present, falling back to `aetherstream.system.microphone`. Explicitly explain that system playback is used **only as the rejection/AEC reference** and is not routed into the mic output.

- [ ] **Step 6: Run GUI tests**

```bash
cargo test -p forgehx-gui
```

Expected: PASS; existing live-edit/manual-domain tests remain intact.

- [ ] **Step 7: Commit checkpoint if Git metadata exists**

```bash
git add crates/forgehx-gui/src/microphone.rs
git commit -m "feat(forgehx): show and control live noise environment"
```

---

### Task 8: Add 10.0.28 Static Contracts and Host Runtime Behavior Gate

**Files:**
- Create: `scripts/test-10.0.28-extraneous-noise-monitor.py`
- Create: `scripts/test-10.0.28-runtime-state.py`
- Create: `scripts/verify-10.0.28.sh`
- Modify or copy-forward existing 10.0.27 routing/bridge tests with 10.0.28 version identity where required.

**Interfaces:**
- Host verifier consumes `forgehx mic dsp get <SoloCast device id>` JSON in full tagged-reply shape.
- Hard outputs include `FORGEHX_EXTRANEOUS_NOISE_MONITOR=PASS`, `FORGEHX_SPEAKER_REFERENCE=PASS` when playback reference is available, `FORGEHX_LIVE_DSP_STATE=PASS`, and final `FORGEHX_10_0_28_VERIFY=PASS`.

- [ ] **Step 1: Write RED static contract script first**

`test-10.0.28-extraneous-noise-monitor.py` must assert source contains:
- `noise_monitor.rs` and `adaptive_noise.rs` modules;
- all 7 class score fields;
- `NoiseAdaptationState` variants;
- `@DEFAULT_AUDIO_SINK@` reference remains in `runtime.rs` exactly once as the full-system reference target;
- no new app-facing source publisher in ForgeHX;
- no `wpctl set-default`, `pactl set-default-source`, `pactl set-default-sink`;
- direct_pipewire fail-closed silence path remains;
- GUI has `Extraneous Noise Monitor`, `Auto Noise Adapt`, and speaker-leak telemetry;
- `MicrophoneDspState` carries `noise_scene_telemetry`.

Run against 10.0.27 baseline first and verify it FAILS.

- [ ] **Step 2: Run static contract on implemented 10.0.28 and make it PASS**

```bash
python3 scripts/test-10.0.28-extraneous-noise-monitor.py
```

Expected:

```text
FORGEHX_10_0_28_EXTRANEOUS_NOISE_MONITOR=PASS
```

- [ ] **Step 3: Implement corrected nested-state runtime parser**

`test-10.0.28-runtime-state.py` reads JSON and always unwraps:

```python
doc = json.load(open(path))
assert doc.get("reply") == "mic_dsp_state"
state = doc.get("state") or {}
cfg = state.get("config") or {}
monitor_cfg = cfg.get("extraneous_noise_monitor") or {}
noise = state.get("noise_scene_telemetry") or {}
```

Hard checks:
- `state.applied is True`;
- `cfg.enabled is True`;
- monitor enabled and auto-adapt true;
- raw source contains `SoloCast` **or** stable ALSA node identity previously proven for the same device;
- processed source exactly `aetherstream.system.microphone`;
- class scores are numeric and each within `0..=1`;
- each band floor within `-90..=-12 dBFS`;
- adaptive suppression `0..=72 dB`;
- Sonora target `0..=100`;
- no unavailable required processors.

The parser must not assume reference availability when no audio is playing; reference-unavailable is valid idle state. A separate active-playback phase below proves speaker reference behavior.

- [ ] **Step 4: Add host active-playback speaker-reference phase**

In `verify-10.0.28.sh`, if `pw-cat` and a normal system sink are available, play a short deterministic low-volume local test tone/noise to the current output while polling `forgehx mic dsp get`. Require within 3 seconds:
- `noise_scene_telemetry.reference_available == true`;
- speaker-leak score remains bounded 0..=1;
- daemon stays active;
- processed target remains `aetherstream.system.microphone`.

Do **not** change default sink/source and do not record or expose raw samples in the verify file. If no playable sink exists, print `FORGEHX_SPEAKER_REFERENCE=INFO:not_testable_no_playback_sink`; this is nonblocking only when AetherStream/PipeWire reports no playback path. When playback is available, failure is blocking.

- [ ] **Step 5: Keep existing hard gates and corrected Clippy policy**

`verify-10.0.28.sh` must run:

```bash
run SOURCE python3 scripts/check-v10.0.28-source.py
run EXTRANEOUS_NOISE_MONITOR python3 scripts/test-10.0.28-extraneous-noise-monitor.py
run RUNTIME_STATE python3 scripts/test-10.0.28-runtime-state.py "$tmp_state"
run SOLOCAST_ROUTING_LOCK python3 scripts/test-10.0.28-solocast-routing-lock.py
run MIC_AETHERSTREAM_BRIDGE python3 scripts/test-10.0.28-aetherstream-bridge.py
run TEST bash -lc 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test --workspace'
run BUILD bash -lc 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo build --workspace --release'
```

Clippy remains:

```bash
if RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo clippy --workspace --all-targets -- -D warnings; then
    printf 'FORGEHX_CLIPPY=PASS\n'
else
    printf 'FORGEHX_CLIPPY=INFO:FAIL:%s\n' "$?"
fi
```

- [ ] **Step 6: Make verify-file creation robust**

Pre-create output before any dependency check:

```bash
OUT="${HOME}/Downloads/ForgeHX-10.0.28-VERIFY.txt"
mkdir -p "${HOME}/Downloads"
: > "$OUT"
exec > >(tee -a "$OUT") 2>&1
```

At the very end always print the output path. Do not use a separate probe as a required release gate.

- [ ] **Step 7: Run scripts locally/static**

```bash
bash -n scripts/verify-10.0.28.sh
python3 -m py_compile scripts/check-v10.0.28-source.py scripts/test-10.0.28-extraneous-noise-monitor.py scripts/test-10.0.28-runtime-state.py
python3 scripts/test-10.0.28-extraneous-noise-monitor.py
```

Expected: syntax clean and static contract PASS.

- [ ] **Step 8: Commit checkpoint if Git metadata exists**

```bash
git add scripts
git commit -m "test(forgehx): gate 10.0.28 adaptive noise runtime"
```

---

### Task 9: Version, Package, and Produce Canonical 10.0.28 Release Artifacts

**Files:**
- Modify: root `Cargo.toml`, workspace crate manifests, `PKGBUILD`, `README.md`, relevant versioned test/check scripts.
- Create release artifacts outside source tree after verification.

**Interfaces:**
- Canonical package version: `10.0.28-1`.
- Host verification file: `~/Downloads/ForgeHX-10.0.28-VERIFY.txt`.

- [ ] **Step 1: Write RED packaging-version test**

`scripts/test-10.0.28-packaging-version.py` asserts:
- workspace/package versions are `10.0.28`;
- `PKGBUILD pkgver=10.0.28`;
- verifier and source-check filenames/version markers are 10.0.28;
- no stale 10.0.27 release identity appears in active packaging metadata except documented baseline references.

Run on baseline first; expected FAIL.

- [ ] **Step 2: Bump canonical version identity to 10.0.28**

Update all first-party ForgeHX workspace crates consistently. Do not change AetherStream requirement from `10.2.33` unless a concrete new AetherStream API is introduced; this plan introduces none.

- [ ] **Step 3: Ensure package install restarts the daemon**

Retain the proven post-install/user-daemon restart behavior so installed 10.0.28 binaries are active before host verification. Do not add default-route commands.

- [ ] **Step 4: Run full native gate on target host**

From source root:

```bash
cargo fmt --all
cargo fmt --all -- --check
RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test --workspace
RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo build --workspace --release
bash scripts/verify-10.0.28.sh
```

Expected hard endpoint:

```text
FORGEHX_EXTRANEOUS_NOISE_MONITOR=PASS
FORGEHX_LIVE_DSP_STATE=PASS
FORGEHX_10_0_28_VERIFY=PASS
```

Clippy may print informational failure without failing release.

- [ ] **Step 5: Build one canonical artifact set**

Produce exactly:
- `ForgeHX-10.0.28-source.tar.gz`
- `ForgeHX-10.0.28-Arch-BuildKit.tar.gz`
- `ForgeHX-10.0.28-INSTALL.sh`
- `ForgeHX-10.0.28-SHA256SUMS.txt`
- `ForgeHX-10.0.28-STATIC-VERIFY.txt`
- `ForgeHX-10.0.28-PACKAGE-VERIFY.txt`
- `ForgeHX-10.0.28-release-manifest.json`

No RC/rN variants.

- [ ] **Step 6: Verify source/BuildKit/installer identity**

Require:
- source tar extracts cleanly;
- BuildKit embeds the exact source tar bytes;
- installer embeds the exact BuildKit bytes;
- installer `--self-test` passes;
- SHA256 file covers all canonical artifacts;
- package/static verify record the host-native gate as PASS only after actual host execution, never from static inference.

- [ ] **Step 7: Final acceptance check against the approved spec**

Verify all 10 acceptance criteria:
1. SoloCast endpoint preserved.
2. DSP continuously applied after restart.
3. Environmental monitor active and speech-safe.
4. Pink/white/broadband/hum/whine/transient telemetry valid.
5. Full-system playback reference used.
6. Speaker leakage reduced while double-talk speech survives.
7. Adaptive changes stay inside manual/locked bounds.
8. AetherStream remains sole app-facing mic publisher.
9. No new virtual mic or default-route mutation.
10. Native tests/build/runtime behavior gates PASS.

- [ ] **Step 8: Commit release checkpoint if Git metadata exists**

```bash
git add -A
git commit -m "release: ForgeHX 10.0.28 extraneous noise monitor"
```

---

## Plan Self-Review

### Spec coverage
- Purpose + existing path extension: Tasks 2-6.
- Pink/white/broadband/hum/whine/transient monitoring: Tasks 2-3.
- Per-band noise floors: Task 2.
- Full-system speaker leakage + double-talk: Tasks 3 and 5.
- Bounded adaptive controller/manual authority: Task 4 and Task 5.
- Backward-compatible config migration: Task 1.
- Telemetry/GUI: Tasks 1, 6, 7.
- Failure behavior/no raw fail-open: Tasks 5 and 8.
- Synthetic/integration/host tests: Tasks 2-5 and 8.
- Canonical release policy: Task 9.

### Type consistency
- `NoiseSceneTelemetry` is defined in Task 1 and is the sole public telemetry type used by Tasks 2-8.
- `AdaptiveNoiseTargets` is defined in Task 4 and consumed by Task 5 only.
- `DspRuntimeRegistry::noise_scene_telemetry` and `DirectRuntimeShared.noise_scene_telemetry` use the Task 1 public type.
- No new IPC command or protocol version is required because existing IPC uses serde JSON and `MicrophoneDspState` additions are `#[serde(default)]`.

### No-placeholder scan
- No `TBD`, `TODO`, “implement later”, or unspecified error-handling steps remain.
- Every behavioral task includes a RED test, exact implementation boundary, verification command, and expected result.
