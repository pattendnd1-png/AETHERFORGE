# ForgeHX Direct Mic Pass-Through Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make ForgeHX own direct HyperX microphone capture and publish one app-facing ForgeHX Mic source without `pw-loopback` or an injection sink.

**Architecture:** Add a native pipewire-rs capture/source runtime. Keep DSP/config/telemetry lifecycle in the existing registry and keep default-source policy in `MicrophoneDspManager`.

**Tech Stack:** Rust 2021, pipewire-rs 0.10.1, Sonora 0.2, PipeWire/WirePlumber, Arch `makepkg`.

**Spec:** `docs/superpowers/specs/2026-09-01-direct-mic-passthrough-design.md`

## Global Constraints

- Version is exactly `10.0.12`.
- Primary mic path contains no `pw-loopback`, no injection sink, and no rendered PipeWire loopback drop-in.
- Physical capture is 48 kHz mono F32LE; DSP frames are 480 samples / 10 ms.
- App-facing node is one stable `Audio/Source` named `ForgeHX Mic`.
- No PipeWire/WirePlumber restart is allowed for normal routing/reconnect.
- Preserve existing DSP, monitoring, Haste, firmware, GUI, stable identity, and communication-routing regressions.
- Arch build and tests deny Rust warnings.

---

### Task 1: Direct-path failing contract

**Files:**
- Create: `scripts/test-10.0.12-direct-mic-path.py`
- Modify: `scripts/test-10.0.12-base-release.sh`

**Interfaces:**
- Consumes: source tree text.
- Produces: static gate proving the old loopback architecture is absent and the direct PipeWire architecture is present.

- [ ] Write a Python contract that rejects `pw-loopback`, `processed_injection_name`, and `render_runtime_bridge`, and requires a `direct_pipewire` module with `Audio/Source`, F32LE, 48000 Hz, mono, stable target name, and bounded buffering.
- [ ] Run it against the 10.0.11-derived tree and confirm it fails for the old loopback code.
- [ ] Add it to the 10.0.12 base-release gate.

### Task 2: Native PipeWire capture/source runtime

**Files:**
- Create: `crates/forgehx-dsp/src/direct_pipewire.rs`
- Modify: `Cargo.toml`
- Modify: `crates/forgehx-dsp/Cargo.toml`
- Modify: `crates/forgehx-dsp/src/lib.rs`
- Modify: `crates/forgehx-dsp/src/runtime.rs`

**Interfaces:**
- Consumes: stable physical `raw_source: &str`, stable public `processed_source: &str`, shared `MicrophoneDspConfig`, DSP control/telemetry slots, stop flag.
- Produces: `run_direct_pipewire(...) -> Result<(), String>` that owns direct capture and one app-facing source until stop/error.

- [ ] Pin `pipewire = "0.10.1"` and add it to `forgehx-dsp`.
- [ ] Implement mono F32LE 48 kHz format negotiation for both streams.
- [ ] Implement physical Input stream targeted by `TARGET_OBJECT=raw_source` and public Output stream with `media.class=Audio/Source`, `node.name=processed_source`, `node.description=ForgeHX Mic`, and no AUTOCONNECT on the source.
- [ ] Accumulate capture samples into exact 480-sample frames, run `VoiceProcessingEngine`, and place processed samples in a bounded queue.
- [ ] Drain the processed queue into public-source buffers; zero-fill underflow and drop oldest queued audio on overflow to cap latency.
- [ ] Preserve config hot updates, enrollment controls, completed voiceprint handoff, VoicePilot telemetry, and isolation telemetry.
- [ ] Make `runtime.rs` call the direct runtime and remove `pw-loopback`, injection-sink creation, bridge probing, and primary-path `pw-cat` capture/playback.

### Task 3: Manager/daemon lifecycle and routing

**Files:**
- Modify: `crates/forgehx-dsp/src/lib.rs`
- Modify: `crates/forgehx-daemon/src/lib.rs`

**Interfaces:**
- Consumes: direct runtime active state and stable processed source name.
- Produces: always-on direct hardware capture plus existing default-source policy.

- [ ] Replace bridge lifecycle APIs with `ensure_direct_source`/always-on direct runtime lifecycle.
- [ ] Remove `processed_injection_name` and `render_runtime_bridge` from production code.
- [ ] Keep `ensure_communication_routing` setting the ForgeHX node as native and Pulse default without app-specific rewrites.
- [ ] Update daemon logs/details from “processed bridge” to “direct microphone runtime”.

### Task 4: Packaging and host verification

**Files:**
- Modify: `PKGBUILD`
- Modify: `scripts/build-release.sh`
- Modify: `README.md`
- Create/update release packaging files for 10.0.12.

**Interfaces:**
- Consumes: direct runtime implementation.
- Produces: canonical source archive, Arch BuildKit, updater, checksums, static verification, host verification contract.

- [ ] Ensure Arch runtime/build dependencies cover libpipewire development through the installed `pipewire` package and keep `-D warnings` build/test gates.
- [ ] Extend host verification to require exactly one ForgeHX Mic source, raw HyperX source present as upstream, ForgeHX default routing, no `pw-loopback` child owned by ForgeHX, and no injection sink node.
- [ ] Emit `DIRECT_HARDWARE_CAPTURE=PASS`, `APP_OWNED_MIC_SOURCE=PASS`, `NO_VIRTUAL_LOOPBACK_CHAIN=PASS`, and final `FORGEHX_VERIFY=PASS`.
- [ ] Run the complete static/base-release suite, updater dry-run, archive/hash equality checks, and package artifact checks before handoff.
