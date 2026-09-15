# ForgeHX 10.0.28 — Extraneous Noise Monitor Design

Date: 2026-09-06  
Status: Approved architecture; implementation pending user review of this written spec  
Baseline: ForgeHX 10.0.27  
Target release: ForgeHX 10.0.28

## 1. Purpose

ForgeHX 10.0.28 adds an always-on **Extraneous Noise Monitor** to the existing microphone DSP engine. Its job is to continuously model non-speech audio reaching the microphone, identify the dominant noise scene, and make bounded real-time adjustments to the existing rejection pipeline so the microphone can react to changing room noise without manual retuning.

The monitor must cover both environmental noise and audio leaking from the user's speakers. It is not a new audio graph, virtual microphone, or routing layer. It operates inside the current ForgeHX direct microphone DSP path and consumes the speaker-reference stream that ForgeHX already captures from the full system output mix.

## 2. Non-negotiable invariants

1. The physical input/output selection remains **`HyperX SoloCast 2 Analog Stereo`**.
2. ForgeHX must not call `wpctl set-default`, `pactl set-default-source`, or `pactl set-default-sink` as part of this feature.
3. AetherStream remains the authoritative system-wide audio graph owner and the sole publisher of the app-facing system microphone.
4. ForgeHX remains the microphone DSP owner.
5. The processed app-facing microphone remains **`aetherstream.system.microphone`**.
6. ForgeHX creates no additional app-facing microphone source.
7. Existing AFXHXM01 ForgeHX → AetherStream mic bridging remains intact.
8. DSP failures remain fail-closed per frame; raw microphone bypass is forbidden.
9. User/manual/locked DSP domains remain authoritative upper/lower bounds for automatic tuning.
10. Speaker rejection must preserve local speech during double-talk.

## 3. Existing pipeline being extended

Baseline 10.0.27 already processes 48 kHz mono audio in 10 ms / 480-sample frames and already supplies an optional render/reference frame to `VoiceProcessingEngine::process_10ms`.

Current high-level flow:

`SoloCast raw frame`
→ Sonora render/reference processing + AEC
→ Sonora capture cleanup
→ click/transient suppression
→ background rejection
→ speaker identity / playback leak guard
→ VoicePilot adaptive targets
→ gate / EQ / dynamics / enhancement / limiter
→ AetherStream bridge
→ `aetherstream.system.microphone`

10.0.28 must extend this flow without replacing it.

## 4. Selected architecture

### 4.1 In-engine adaptive Noise Scene Monitor

The selected approach is an in-engine monitor owned by `forgehx-dsp`, updated once per 10 ms frame. This avoids a second realtime thread, synchronization boundary, or additional audio transport.

New logical flow:

`SoloCast raw`
→ **speaker-reference/AEC path**
→ **double-talk + speech guard**
→ **Extraneous Noise Monitor**
→ **Adaptive Noise Controller**
→ Sonora cleanup / residual speaker suppression / background rejection
→ existing ForgeHX voice chain
→ AetherStream mic bridge

The monitor receives:

- raw microphone frame;
- full-system speaker/render reference frame when available;
- cleaned capture frame where needed for post-AEC residual analysis;
- speech/VAD confidence or equivalent speech-activity evidence;
- current user DSP configuration and automation-domain state.

## 5. Noise classes

The monitor must estimate confidence for these classes rather than force one mutually exclusive label:

- **White-like broadband noise** — approximately flat power spectral density.
- **Pink-like broadband noise** — approximately −3 dB/octave spectral slope.
- **Fan / HVAC / computer broadband noise** — stable or slowly varying broadband energy with persistent spectral shape.
- **Low-frequency hum / rumble** — persistent low-band energy, including 50/60 Hz families and mechanical rumble.
- **Narrowband whine** — one or more persistent high-Q spectral peaks.
- **Transient / click activity** — brief non-speech impulses such as keyboard/mouse or mechanical clicks.
- **Speaker leakage / residual echo** — microphone energy correlated with the full-system render reference after AEC.
- **Mixed / unknown extraneous noise** — stable non-speech energy that does not fit the above classes strongly enough.

The system must support mixed scenes such as HVAC + speaker leakage + keyboard clicks.

## 6. Analysis model

### 6.1 Frequency analysis

Use a lightweight realtime spectral analysis suitable for 10 ms frames. The implementation may use the existing Sonora FFT dependency or an equivalent already-present crate; no heavyweight external service is allowed.

Maintain normalized energy estimates for broad bands spanning approximately:

- sub/rumble: 20–80 Hz
- low: 80–250 Hz
- low-mid: 250–700 Hz
- mid: 700–2 kHz
- presence: 2–5 kHz
- high: 5–10 kHz
- air: 10–20 kHz

Exact bin aggregation is implementation detail, but the public telemetry should expose stable broad-band values rather than raw FFT bins.

### 6.2 Pink/white classification

Estimate spectral slope over usable voice-band-plus-high-band energy:

- white-like confidence rises as the fitted spectral slope approaches approximately 0 dB/octave;
- pink-like confidence rises as the fitted slope approaches approximately −3 dB/octave;
- confidence is reduced when the frame is strongly tonal, transient, or speech-dominant.

The classifier must not require laboratory-perfect noise; it is a likeness score for adaptive control.

### 6.3 Persistent per-band noise floor

Maintain a rolling noise-floor estimate per broad frequency band with asymmetric adaptation:

- faster adaptation downward when the environment becomes quieter;
- slower adaptation upward only when non-speech stability is sustained;
- freeze or heavily slow upward learning while speech or double-talk is detected;
- freeze learning during strong transient events;
- reject implausibly fast floor jumps.

This extends the principle already used by the 10.0.26+ background rejector from a scalar floor to a spectral noise profile.

### 6.4 Speaker-leak / residual-echo estimate

Use the existing full-system render frame as the speaker reference. Compute a bounded correlation/coherence-like leakage score between render energy and microphone/post-AEC residual energy.

The monitor must distinguish:

- playback only;
- speech only;
- double-talk (speech + playback);
- uncertain/transient state.

During double-talk, echo/noise model adaptation is frozen or strongly slowed so local speech cannot be learned as speaker leakage.

## 7. Adaptive Noise Controller

The controller converts monitor telemetry into bounded targets. It must never directly mutate routing or bypass the existing configuration model.

Permitted adaptive outputs:

- background rejection threshold/margin target;
- background rejection strength target;
- Sonora noise-suppression target level/percent within user bounds;
- residual speaker-leak suppression amount;
- low-frequency cleanup target where the user has allowed Auto for that domain;
- spectral weighting used by the background rejector;
- learning/hold timing within safe predefined ranges.

### 7.1 Rate limits and hysteresis

All adaptive changes must be rate-limited and hysteretic to avoid audible pumping or rapid mode switching.

Recommended control cadence:

- monitor: every 10 ms frame;
- short-term state: 100–300 ms windows;
- stable scene recognition: roughly 300–1000 ms depending on class;
- heavy Sonora reconfiguration: retain the existing coarse cadence rather than reconfigure every frame;
- fine background-rejector weighting may update more frequently because it is internal and does not rebuild Sonora state.

### 7.2 Manual authority

Existing automation modes remain authoritative:

- **AUTO** — monitor may retune within configured safe bounds;
- **MANUAL** — user value is used; monitor observes but does not change that domain;
- **LOCKED** — user value is immutable to automatic systems;
- **BYPASSED** — corresponding processing remains bypassed.

The Extraneous Noise Monitor itself gets a master **Auto Noise Adapt** control. Turning it off leaves telemetry available but stops adaptive changes.

## 8. Speaker-noise rejection

Speaker-noise rejection must use the **entire system output mix** already captured as the render/reference path, covering games, browsers, Discord, music, alerts, and media.

Processing order must favor AEC first, then residual analysis/suppression. The monitor does not replace Sonora AEC; it supervises residual speaker leakage and supplies bounded cleanup targets.

Required safeguards:

- double-talk protection;
- no adaptation from local speech into the speaker model;
- no hard mute solely because playback exists;
- residual suppression proportional to measured playback correlation;
- graceful behavior when the render reference is unavailable;
- telemetry must explicitly show reference availability.

## 9. New DSP components

Preferred source boundaries:

### `crates/forgehx-dsp/src/noise_monitor.rs`
Owns realtime scene analysis and persistent model state.

Suggested public concepts:

- `ExtraneousNoiseMonitor`
- `NoiseSceneTelemetry`
- `NoiseBandLevels`
- `NoiseClassScores`
- `NoiseAdaptationState` (`Learning`, `Holding`, `Suppressing`, `SpeechProtected`, `ReferenceUnavailable` as applicable)

### `crates/forgehx-dsp/src/adaptive_noise.rs`
Owns mapping from telemetry + user policy to bounded DSP targets.

Suggested concept:

- `AdaptiveNoiseTargets`

This separation keeps measurement/classification independent from policy/control.

### Existing files touched

- `crates/forgehx-dsp/src/engine.rs` — integrate monitor/controller into the 10 ms processing path.
- `crates/forgehx-dsp/src/background_rejection.rs` — accept bounded spectral/adaptive targets without giving up existing speech grace or fail-closed behavior.
- `crates/forgehx-dsp/src/direct_pipewire.rs` / runtime code — only if telemetry plumbing requires it; no new capture graph.
- `crates/forgehx-core/src/lib.rs` — config and telemetry schema additions.
- `crates/forgehx-daemon/src/lib.rs` — expose state through existing IPC reply/state plumbing.
- `crates/forgehx-gui/src/microphone.rs` — Noise Environment UI and Auto Noise Adapt controls.
- CLI/IPC files as needed for telemetry inspection and host verification.

## 10. Configuration additions

Add a backward-compatible configuration block under microphone DSP, conceptually:

- `extraneous_noise_monitor.enabled` — default true for the canonical SoloCast profile.
- `extraneous_noise_monitor.auto_adapt` — default true.
- `extraneous_noise_monitor.sensitivity_percent` — bounded user sensitivity.
- `extraneous_noise_monitor.max_adaptive_suppression_db` — hard cap; must not exceed the product-wide safety maximum.
- `extraneous_noise_monitor.speaker_rejection_enabled` — default true when AEC/reference path is available.

Schema migration must preserve existing user profiles and synthesize safe defaults for profiles created before 10.0.28.

## 11. Telemetry and GUI

The Mic DSP page must expose a live **Noise Environment** section with:

- monitor enabled/disabled;
- Auto Noise Adapt enabled/disabled;
- adaptation state;
- overall extraneous-noise level dBFS;
- pink-like confidence %;
- white-like confidence %;
- broadband/fan-HVAC confidence %;
- hum/rumble confidence %;
- narrowband whine confidence %;
- transient activity %;
- speaker-leak / residual-echo confidence %;
- per-band noise-floor readout;
- dominant noise bands;
- current adaptive suppression target dB;
- current Sonora noise-suppression target;
- speaker-reference available/unavailable;
- speech / double-talk protection state.

Telemetry is diagnostic/control-plane data only; it must not expose raw microphone samples or speaker samples through IPC.

## 12. Failure behavior

- If spectral analysis fails for a frame, retain the last safe monitor state and do not increase suppression.
- If speaker reference disappears, disable speaker-reference adaptation and continue environmental-noise monitoring.
- If the adaptive controller produces invalid targets, clamp/reject them and retain the last safe targets.
- If the core DSP frame itself errors, preserve the existing fail-closed-to-silence policy.
- Never fail open to the raw microphone.

## 13. Testing strategy

### 13.1 Unit/synthetic DSP tests

Create deterministic fixtures for:

- white noise;
- pink noise;
- steady fan/HVAC-like shaped noise;
- 50/60 Hz hum plus harmonics;
- narrowband whine;
- keyboard/click transients;
- clean speech-like varying signal;
- playback-reference leakage;
- speech + playback double-talk;
- mixed noise scenes;
- sudden quiet → loud and loud → quiet transitions.

Required assertions include:

- pink/white likeness scores separate their fixtures meaningfully;
- stable noise raises learned per-band floors only after stability criteria;
- speech does not train the noise floor upward;
- double-talk freezes/slows speaker-leak learning;
- speaker-only leakage is suppressed more strongly than uncorrelated local speech;
- transient events do not permanently shift the learned floor;
- adaptive targets remain inside configured bounds;
- user LOCKED/MANUAL/BYPASSED domains are respected;
- no raw fail-open path exists.

### 13.2 Integration tests

- existing AEC render path continues to feed the engine;
- monitor receives the full-system render reference when present;
- 10.0.27 always-on activation remains intact;
- exact SoloCast routing lock remains intact;
- AetherStream bridge remains intact;
- processed source remains `aetherstream.system.microphone`;
- no default-source/sink mutations are introduced.

### 13.3 Host runtime gate

The 10.0.28 verifier must hard-gate:

- installed version 10.0.28-1;
- daemon active after package cutover;
- DSP applied and enabled;
- Extraneous Noise Monitor enabled;
- Auto Noise Adapt enabled for canonical profile;
- raw source target non-empty and mapped to the SoloCast physical source;
- processed target equals `aetherstream.system.microphone`;
- speaker reference available when system playback path is available;
- live monitor telemetry returns valid bounded values;
- no unavailable required processors;
- AetherStream mic bridge socket healthy;
- AetherStream output-DSP socket health gate unchanged.

Clippy remains diagnostic-only/nonblocking. Build, tests, runtime state, routing lock, bridge health, and actual DSP behavior remain hard gates.

## 14. Release / compatibility policy

- Canonical release: **ForgeHX 10.0.28**.
- One source archive, one Arch BuildKit, one installer, one checksum file, one package verify, one static verify, one release manifest.
- Host verifier writes `~/Downloads/ForgeHX-10.0.28-VERIFY.txt`.
- No RC/rN suffixes.
- Do not bump again solely for Clippy/style warnings.
- Any real functional correction after 10.0.28 requires 10.0.29.

## 15. Acceptance criteria

ForgeHX 10.0.28 is accepted only when all of the following are true on the target host:

1. The SoloCast remains the preserved physical endpoint.
2. The microphone DSP remains continuously applied after install/restart.
3. The monitor continuously classifies environmental noise without learning active speech as noise.
4. Pink/white/broadband/hum/whine/transient classes produce usable realtime telemetry.
5. Full-system speaker playback is used as the rejection reference.
6. Speaker leakage is reduced while simultaneous local speech is preserved.
7. Adaptive changes occur automatically but never exceed user/manual/locked bounds.
8. AetherStream remains the sole app-facing microphone publisher.
9. No new virtual/app-facing mic source or default-route mutation is introduced.
10. Native tests/build and the new host runtime/behavior gates pass.
