# ForgeHX HyperX Microphone Input Stack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an input-only HyperX microphone processing subsystem that creates a selectable `ForgeHX Processed Mic` PipeWire source with typed DSP controls, daemon/IPC routing, CLI controls, and a dedicated GUI processing page.

**Architecture:** ForgeHX keeps hardware/native control, Linux audio control, and software DSP as separate ownership domains. The new `forgehx-dsp` crate owns only microphone capture processing and writes a namespaced PipeWire filter-chain drop-in; it never creates or targets a normal playback sink. The daemon remains the sole writer and exposes protocol-v4 microphone commands after the existing Ping negotiation.

**Tech Stack:** Rust 2021, serde/serde_json, PipeWire `libpipewire-module-filter-chain`, `wpctl`, optional RNNoise LV2/LADSPA provider, egui/eframe, Arch PKGBUILD.

**Spec:** `docs/superpowers/specs/2026-08-14-forgehx-hyperx-microphone-full-stack-design.md`

## Global Constraints

- HyperX microphones only; non-HyperX sources do not receive the ForgeHX microphone DSP capability.
- Input-only signal direction: physical microphone capture -> ForgeHX DSP -> `ForgeHX Processed Mic` virtual source.
- Never create or attach to a normal playback/output DSP sink.
- No JamesDSP or EasyEffects runtime dependency.
- Unknown vendor HID writes remain blocked; this plan does not add unverified HyperX hardware writes.
- Daemon remains the sole writer.
- DSP parameters are typed, bounded, serializable, and validated before graph generation.
- Failed graph replacement preserves raw microphone capture and never modifies unrelated PipeWire configuration.
- IPC v2/v3 compatibility remains for existing commands; new microphone DSP commands require v4.

---

### Task 1: Core microphone DSP schema and IPC v4

**Files:**
- Modify: `crates/forgehx-core/src/lib.rs`

**Interfaces:**
- Produces `MicrophoneDspConfig`, `MicEqBand`, `NoiseSuppressionConfig`, `GateConfig`, `CompressorConfig`, `DeEsserConfig`, `LimiterConfig`, `MicrophoneDspState`.
- Produces `Command::{MicDspGet,MicDspSave,MicDspApply,MicDspBypass}` and `Reply::MicDspState`.

- [ ] **Step 1: Write failing unit tests** proving default DSP config validates, unsafe thresholds/frequencies reject, and every new command reports minimum protocol version 4.
- [ ] **Step 2: Run `cargo test -p forgehx-core` when Cargo is available; in this container run a static guard that fails because the types/IPC variants do not yet exist.**
- [ ] **Step 3: Implement the typed config with exact bounds:** input/output gain `-30..=24 dB`, high-pass `20..=500 Hz`, EQ `20..=20000 Hz`/`-24..=24 dB`/Q `0.1..=18`, noise suppression amount `0..=100`, gate threshold `-90..=0 dB`, compressor threshold `-60..=0 dB`, ratios `1..=20`, de-esser frequency `2000..=12000 Hz`, limiter ceiling `-12..=0 dB`.
- [ ] **Step 4: Bump `IPC_PROTOCOL_VERSION` to 4 while retaining `IPC_MIN_PROTOCOL_VERSION = 2`, and mark all new mic DSP commands as v4-only.**
- [ ] **Step 5: Commit `feat(core): add typed microphone DSP IPC`**.

### Task 2: Input-only `forgehx-dsp` PipeWire graph manager

**Files:**
- Create: `crates/forgehx-dsp/Cargo.toml`
- Create: `crates/forgehx-dsp/src/lib.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Produces `MicDspManager::{list,get,save,apply,bypass,status}`.
- Produces `render_filter_chain(config, raw_node_name)` returning a PipeWire configuration whose exposed endpoint is `media.class = "Audio/Source"` with `node.name = "forgehx_processed_mic"`.

- [ ] **Step 1: Write failing tests** for profile path confinement, config validation, input-only graph invariants, stable source targeting, atomic profile writes, and bypass removing only ForgeHX mic config.
- [ ] **Step 2: Run static RED guard** requiring `forgehx_processed_mic`, `Audio/Source`, and absence of a ForgeHX `Audio/Sink` endpoint.
- [ ] **Step 3: Implement profile persistence under `$XDG_CONFIG_HOME/forgehx/microphone/` and managed PipeWire drop-in `$XDG_CONFIG_HOME/pipewire/pipewire.conf.d/91-forgehx-mic.conf`.**
- [ ] **Step 4: Render an input-only filter-chain with built-in gain/high-pass/EQ stages and optional RNNoise provider detection. Missing optional processors are represented in state instead of silently pretending to run.**
- [ ] **Step 5: Resolve raw source IDs to stable `node.name` via `wpctl inspect`, write atomically, restart only the user PipeWire service, and verify `forgehx_processed_mic` appears via `wpctl status`.**
- [ ] **Step 6: Commit `feat(dsp): add input-only microphone graph manager`**.

### Task 3: HyperX-only daemon integration and capability ownership

**Files:**
- Modify: `crates/forgehx-daemon/Cargo.toml`
- Modify: `crates/forgehx-daemon/src/lib.rs`

**Interfaces:**
- Consumes `MicDspManager` and new core IPC.
- Exposes mic DSP only for logical devices where `device_class == Microphone && vendor_family == HyperX` and a current PipeWire source is deterministically associated.

- [ ] **Step 1: Write failing daemon tests** showing a HyperX microphone receives `MicDsp` ownership while a Logitech microphone does not, and that DSP application refuses an unassociated/stale source node.
- [ ] **Step 2: Add `Capability::MicDsp` and `BackendKind::ForgeHxDsp`; ensure owner selection cannot displace native hardware gain/mute controls.**
- [ ] **Step 3: Add `MicDspManager` to `DaemonState`, populate microphone DSP ownership during refresh, and route get/save/apply/bypass commands.**
- [ ] **Step 4: On apply, resolve the selected logical HyperX microphone to its associated live `source` node and pass that node to the DSP manager.**
- [ ] **Step 5: Commit `feat(daemon): route HyperX microphone DSP controls`**.

### Task 4: CLI microphone processing controls

**Files:**
- Modify: `crates/forgehx-cli/src/main.rs`

**Interfaces:**
- Produces commands `forgehx mic dsp get|save|apply|bypass`.

- [ ] **Step 1: Write parser tests** for `mic dsp get`, `mic dsp apply`, and `mic dsp bypass` argument shapes.
- [ ] **Step 2: Add CLI structs/enums and map them to protocol-v4 mic DSP commands through the shared IPC negotiation client.**
- [ ] **Step 3: Print applied/bypass state and unavailable processor details without hiding errors.**
- [ ] **Step 4: Commit `feat(cli): add HyperX microphone DSP controls`**.

### Task 5: GUI microphone processing page

**Files:**
- Create: `crates/forgehx-gui/src/microphone.rs`
- Modify: `crates/forgehx-gui/src/main.rs`
- Modify: `crates/forgehx-gui/src/app.rs`
- Modify: `crates/forgehx-gui/src/device_page.rs`

**Interfaces:**
- Produces `MicControl::{Save,Apply,Bypass}` and an editor for the typed `MicrophoneDspConfig`.

- [ ] **Step 1: Write capability/tab tests** proving Processing is shown only when `Capability::MicDsp` is present.
- [ ] **Step 2: Add a microphone processing editor for input/output gain, HPF, suppression, gate, EQ bands, compressor, de-esser, limiter and bypass.**
- [ ] **Step 3: Label the page `Input DSP` and display `ForgeHX Processed Mic`; do not render output/speaker DSP controls.**
- [ ] **Step 4: Wire actions through existing IPC refresh/error handling.**
- [ ] **Step 5: Commit `feat(gui): add HyperX input DSP controls`**.

### Task 6: Packaging, regression guards, docs, and r6 archive

**Files:**
- Modify: `PKGBUILD`
- Modify: `scripts/check-package.sh`
- Modify: `README.md`
- Modify: `scripts/create-source-package.sh`
- Modify: `scripts/build-arch-package.sh`
- Modify: `scripts/build-release.sh`

**Interfaces:**
- Produces ForgeHX `0.3.0-6` Arch build kit and source archive.

- [ ] **Step 1: Add runtime dependencies `noise-suppression-for-voice` and `lsp-plugins-lv2`; keep them strictly microphone-processing dependencies.**
- [ ] **Step 2: Add package guards requiring `forgehx-dsp`, IPC v4, `forgehx_processed_mic`, `Audio/Source`, and forbidding JamesDSP/EasyEffects/runtime ForgeHX playback-DSP markers.**
- [ ] **Step 3: Document the raw-source -> processed-source workflow and HyperX-only behavior.**
- [ ] **Step 4: Set `pkgrel=6`, run package/static checks, shell syntax checks, Cargo manifest parsing, and `git diff --check`.**
- [ ] **Step 5: If Cargo/makepkg are unavailable, record that limitation instead of claiming compile success.**
- [ ] **Step 6: Build source/build-kit archives from the committed tree, pin SHA-256, fresh-extract them, and rerun static/package verification.**
- [ ] **Step 7: Commit `release: prepare ForgeHX 0.3.0-6 microphone input stack`**.
