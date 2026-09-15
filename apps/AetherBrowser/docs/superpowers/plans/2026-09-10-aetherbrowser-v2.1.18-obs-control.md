# AetherBrowser v2.1.18 OBS Control Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship canonical AetherBrowser v2.1.18 with the Servo Clippy blocker fixed and real local OBS WebSocket 5.x control wired into Stream Studio.

**Architecture:** `aether-stream-studio` owns the OBS protocol/client boundary; `aether-native-pages` maps `aether://stream` actions into that boundary and renders observed state. AetherStream's supervisor socket contract remains intact and independent.

**Tech Stack:** Rust 2024, tungstenite 0.30, serde/serde_json, SHA-256, base64, Servo 0.5.0, shell release verification.

**Spec:** `docs/superpowers/specs/2026-09-10-aetherbrowser-v2.1.18-obs-control-design.md`

## Global Constraints

- Canonical release version is exactly `2.1.18`; no RC/rN suffixes.
- OBS defaults to `ws://127.0.0.1:4455` and may be overridden by `AETHER_OBS_WEBSOCKET_URL`.
- OBS password may be read from `AETHER_OBS_WEBSOCKET_PASSWORD` but must not be logged or rendered.
- AetherStream supervisor socket remains authoritative when present; direct OBS transport is compatibility control, not a replacement service architecture.
- No Clippy lint suppression for the Servo `Opts` blocker.

---

### Task 1: Servo Clippy repair

**Files:**
- Modify: `crates/aether-engine-servo/src/live.rs`
- Test: `tests/current-servo-opts-contract.sh`

**Interfaces:**
- Consumes: Servo `Opts: Default`.
- Produces: direct `Opts { config_dir, temporary_storage, ..Default::default() }` construction.

- [x] Write a static regression test that fails while `let mut servo_opts = Opts::default()` plus field reassignments remain.
- [x] Run the test and confirm RED on v2.1.17.
- [x] Replace the reassignment pattern with a direct struct initializer.
- [x] Run the regression test and confirm GREEN.

### Task 2: OBS WebSocket protocol and client

**Files:**
- Modify: `crates/aether-stream-studio/Cargo.toml`
- Create: `crates/aether-stream-studio/src/obs.rs`
- Modify: `crates/aether-stream-studio/src/lib.rs`
- Create: `crates/aether-stream-studio/tests/obs_protocol.rs`
- Test: `tests/current-obs-websocket-contract.sh`

**Interfaces:**
- Produces: `ObsWebSocketConfig`, `ObsWebSocketClient`, `ObsStatusSnapshot`, `ObsError`, `obs_authentication`, and `execute_obs_intent`.
- `ObsWebSocketClient::connect(config) -> Result<Self, ObsError>`.
- `ObsWebSocketClient::status(&mut self) -> Result<ObsStatusSnapshot, ObsError>`.
- `ObsWebSocketClient::send_intent(&mut self, StudioControlIntent) -> Result<(), ObsError>`.

- [x] Add failing Rust/static tests for default loopback endpoint, challenge authentication, opcode handshake, request ID/status validation, and concrete control request names.
- [x] Run static test and confirm RED because no OBS transport exists.
- [x] Add only the dependencies needed for JSON WebSocket/auth hashing.
- [x] Implement handshake/auth/request/state/control in `obs.rs`.
- [x] Export the OBS types/functions from `lib.rs` and implement `AetherStreamServiceBridge` for `ObsWebSocketClient`.
- [x] Run static protocol contract and confirm GREEN; leave Rust compile/test for host gate because this build environment has no Rust toolchain.

### Task 3: Wire Stream Studio controls to OBS

**Files:**
- Modify: `crates/aether-native-pages/src/lib.rs`
- Modify: `crates/aether-native-pages/tests/pages.rs`
- Test: `tests/current-obs-websocket-contract.sh`

**Interfaces:**
- Consumes: `ObsWebSocketConfig::from_env`, `ObsWebSocketClient`, `StudioControlIntent`.
- Produces: actionable `aether://stream?action=...` UI with observed OBS state.

- [x] Add failing page tests/static assertions for refresh, start/stop stream, start/stop record, save replay, scene selection, and mixer gain actions.
- [x] Confirm RED against the static v2.1.17 Stream Studio markup.
- [x] Parse Stream Studio actions and execute them through the OBS client.
- [x] Render observed OBS connection/version/scenes/current-scene/live/record/replay state and bounded errors.
- [x] Confirm Stream Studio action contract GREEN.

### Task 4: Canonical v2.1.18 identity and hard verification

**Files:**
- Modify: workspace version, README, packaging metadata, scripts, capture filenames, and current-release tests from `2.1.17`/`V2_1_17` to `2.1.18`/`V2_1_18`.
- Add OBS/Servo static gates to `scripts/verify.sh` via the existing `tests/current-*.sh` discovery/explicit gate pattern.

**Interfaces:**
- Produces: one canonical v2.1.18 source/package/install identity.

- [x] Bump all canonical release identity references.
- [x] Update README release notes with Servo fix and OBS WebSocket transport.
- [x] Run every `tests/current-*.sh` test and all shell syntax checks.
- [x] Verify no `2.1.17`/`V2_1_17` release identity remains outside historical docs/spec context.

### Task 5: Package and handoff

**Files:**
- Produce: `Aether-Browser-v2.1.18-source.zip`
- Produce: `Aether-Browser-v2.1.18-INSTALL.run`
- Produce: `Aether-Browser-v2.1.18-STATIC-VERIFY.txt`
- Produce: `Aether-Browser-v2.1.18-FINAL-VERIFY.txt`
- Produce: `Aether-Browser-v2.1.18-SHA256SUMS.txt`

**Interfaces:**
- Produces: self-contained one-line host install/run handoff.

- [x] Run fresh-extract static verification from the ZIP.
- [x] Build the consolidated `.run` installer with embedded v2.1.18 source.
- [x] Verify embedded payload checksum equals source ZIP checksum.
- [x] Record sandbox limitation honestly: host Rust compile/install/runtime OBS verification remains unclaimed until the user's AetherForge machine runs it.
