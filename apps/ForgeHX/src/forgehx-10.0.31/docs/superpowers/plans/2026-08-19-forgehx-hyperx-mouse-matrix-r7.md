# ForgeHX HyperX Mouse Matrix r7 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generalize ForgeHX mouse support into protocol families, recognize the complete approved HyperX current/legacy mouse matrix, retain Haste v1 native control, and add the Saga Pro native subset that can be written without overwriting coupled settings.

**Architecture:** Add a dedicated `forgehx-mouse` crate that owns model descriptors and native protocols. Device discovery remains generic; the daemon asks the mouse registry to identify a model and only grants native capability ownership for exact VID/PID identities with implemented handlers. Alias/name-only matches improve recognition and UI limits but remain write-protected except through runtime OpenRGB/libratbag owners.

**Tech Stack:** Rust 2021, hidapi linux-native, serde IPC, OpenRGB SDK fallback, libratbag fallback, udev.

**Spec:** `docs/superpowers/specs/2026-08-14-forgehx-all-hyperx-mice-full-support-design.md`

## Global Constraints

- No firmware flashing.
- No unknown vendor writes.
- Native ownership requires exact verified VID/PID + implemented handler.
- Name-only model recognition never grants native writes.
- Capability ownership remains ForgeHX Native > Linux Standard > OpenRGB/libratbag > Diagnostic.
- Preserve r6 input-only microphone architecture unchanged.
- IPC minimum remains v2; new mouse-matrix commands require v5.

---

### Task 1: Core mouse model/capability contract

**Files:**
- Modify: `crates/forgehx-core/src/lib.rs`
- Test: core unit tests plus `scripts/test-hyperx-mouse-matrix.sh`

**Produces:** `MouseProtocolFamily`, `MouseLimits`, `MouseModelInfo`, `MouseCapabilities` command/reply, IPC v5.

- [ ] Write failing static/unit tests for IPC v5 and typed model limits.
- [ ] Verify RED.
- [ ] Add the typed core structures and v5 command/reply.
- [ ] Preserve v2-v4 negotiation/projection.
- [ ] Verify GREEN and commit.

### Task 2: Dedicated HyperX mouse registry + Haste v1 migration

**Files:**
- Create: `crates/forgehx-mouse/Cargo.toml`
- Create: `crates/forgehx-mouse/src/lib.rs`
- Create: `crates/forgehx-mouse/src/registry.rs`
- Create: `crates/forgehx-mouse/src/protocol/mod.rs`
- Create: `crates/forgehx-mouse/src/protocol/haste_v1.rs`
- Modify: workspace `Cargo.toml`
- Modify: `crates/forgehx-device/src/drivers.rs`
- Modify: `crates/forgehx-device/src/lib.rs`

**Produces:** complete seeded logical model matrix, exact-ID and exact-alias recognition, Haste v1 native transport in `forgehx-mouse`.

- [ ] Write failing registry guard for every approved logical model and verified legacy/current IDs.
- [ ] Verify RED.
- [ ] Implement model descriptors and matching rules.
- [ ] Move Haste packet transport to the new crate without behavior changes.
- [ ] Remove duplicate native Haste transport from `forgehx-device`.
- [ ] Verify GREEN and commit.

### Task 3: Saga Pro verified native protocol

**Files:**
- Create: `crates/forgehx-mouse/src/protocol/saga_pro.rs`
- Test: packet golden tests + static guard.

**Produces:** exact `03f0:04bf` wired and `03f0:06bf` wireless matching; 64-byte battery query; tested 4-stage DPI/polling encoders; native static RGB/brightness and battery/status ownership. DPI/polling ownership remains gated until a verified read/preserve path exists because both values share report `0x32`.

- [ ] Write failing golden guards for IDs, DPI/polling encoding, battery query, and RGB packet prefixes.
- [ ] Verify RED.
- [ ] Implement exact interface selection and packet encoding/readback.
- [ ] Reject 2K/4K polling in wired mode.
- [ ] Keep DPI/polling packet encoders non-routed until read-before-write preservation is verified.
- [ ] Verify GREEN and commit.

### Task 4: Daemon ownership and IPC routing

**Files:**
- Modify: `crates/forgehx-daemon/Cargo.toml`
- Modify: `crates/forgehx-daemon/src/lib.rs`

**Produces:** model identification during refresh; native owners only for implemented exact-ID families; `MouseCapabilities`; Haste DPI/polling routing; Saga static-lighting/battery routing; v4 and older clients never receive v5-only model metadata because `MouseCapabilities` itself requires v5.

- [ ] Write failing daemon/source guards for Haste and Saga ownership plus unknown-ID no-write behavior.
- [ ] Verify RED.
- [ ] Implement registry attachment and routing.
- [ ] Keep partial support accurate when a model is recognized but some controls are fallback/unimplemented.
- [ ] Verify GREEN and commit.

### Task 5: Dynamic CLI/GUI mouse controls

**Files:**
- Modify: `crates/forgehx-cli/src/main.rs`
- Modify: `crates/forgehx-gui/src/app.rs`
- Modify: `crates/forgehx-gui/src/device_page.rs`
- Modify: `crates/forgehx-gui/src/mouse.rs`

**Produces:** `forgehx mouse capabilities`; model/family/limits display; DPI/polling controls derive their allowed range/rates from model metadata rather than global hard-coding.

- [ ] Write failing CLI/GUI source guards.
- [ ] Verify RED.
- [ ] Implement command/reply and dynamic UI limits.
- [ ] Preserve egui outer-action pattern.
- [ ] Verify GREEN and commit.

### Task 6: Exact permissions, package and release r7

**Files:**
- Modify: `packaging/udev/70-forgehx.rules`
- Modify: `PKGBUILD`
- Modify: `scripts/check-package.sh`
- Modify: `README.md`

**Produces:** exact udev rules for verified IDs, `pkgrel=7`, fresh source/build kit archives.

- [ ] Add exact rules for verified HyperX mouse IDs from public protocol/device metadata.
- [ ] Add package invariant guard and bump `pkgrel=7`.
- [ ] Run all static/source checks, manifest parsing, shell syntax, and `git diff --check`.
- [ ] Create exact committed source archive and pinned build kit.
- [ ] Fresh-extract and rerun all available checks.
- [ ] Attempt `cargo`/`makepkg` and report environment boundary honestly.
