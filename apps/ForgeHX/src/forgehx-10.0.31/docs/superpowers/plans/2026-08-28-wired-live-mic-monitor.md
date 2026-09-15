# Wired Live Mic Monitor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a low-latency live monitor of ForgeHX Processed Mic routed only to a wired ALSA/analog/headphone sink.

**Architecture:** Add a daemon-owned monitor control path that selects a non-Bluetooth sink and asks the DSP runtime to bridge the processed source to it. Keep the monitor in a separate worker so the authoritative stream/DSP path is not modified.

**Tech Stack:** Rust, PipeWire `pw-cat`, ForgeHX Unix-socket IPC, egui.

**Spec:** `docs/superpowers/specs/2026-08-28-wired-live-mic-monitor-design.md`

## Global Constraints
- Canonical release version is 10.0.5.
- Never route monitor audio to BlueZ/Bluetooth.
- Stream output remains ForgeHX Processed Mic and is never rerouted by monitor state.
- Monitor defaults off and is runtime-only.
- No new external dependency beyond existing PipeWire tools.

---

### Task 1: Core monitor state and IPC
**Files:** Modify `crates/forgehx-core/src/lib.rs`; test in the same module.
- [ ] Add `MicrophoneMonitorState` and monitor fields to `MicrophoneDspState` with serde defaults.
- [ ] Add `MicMonitorSet { device_id, enabled, level_percent }` command and protocol projection/version handling.
- [ ] Add tests for default state, valid 0..=100 level, and command protocol handling.

### Task 2: Wired sink selection
**Files:** Modify `crates/forgehx-daemon/src/lib.rs`; add focused unit tests.
- [ ] Write tests proving `bluez_output` and Bluetooth names are rejected.
- [ ] Write tests proving headphone > analog > other `alsa_output` preference.
- [ ] Implement deterministic `select_wired_monitor_sink`.

### Task 3: DSP monitor worker
**Files:** Modify `crates/forgehx-dsp/src/runtime.rs` and `crates/forgehx-dsp/src/lib.rs`.
- [ ] Add failing source-contract test for processed-source capture and wired-target playback.
- [ ] Add runtime monitor configuration/state and a reconnecting monitor worker using `pw-cat`.
- [ ] Add `MicDspManager::set_monitor` and expose monitor status through `MicrophoneDspState`.

### Task 4: Daemon command
**Files:** Modify `crates/forgehx-daemon/src/lib.rs`.
- [ ] Handle `MicMonitorSet` by validating MicDsp ownership, discovering audio sinks, rejecting no-wired-sink, and calling `MicDspManager::set_monitor`.
- [ ] Return updated `MicDspState`.

### Task 5: GUI controls
**Files:** Modify `crates/forgehx-gui/src/microphone.rs`, `device_page.rs`, `app.rs`.
- [ ] Add Live Headphone Monitor toggle and level slider.
- [ ] Display active wired sink or unavailable status.
- [ ] Route controls through new IPC command; debounce level changes.

### Task 6: Release and verification
**Files:** Update Cargo/PKGBUILD/docs/scripts for 10.0.5; add `scripts/test-10.0.5-wired-mic-monitor.py` and wire into `scripts/check-package.sh`.
- [ ] Retarget canonical release identity to 10.0.5.
- [ ] Verify monitor test fails before production behavior and passes after implementation.
- [ ] Run full `scripts/check-package.sh`.
- [ ] Package source, Arch build kit, updater, checksums, release manifest, and verification file.
