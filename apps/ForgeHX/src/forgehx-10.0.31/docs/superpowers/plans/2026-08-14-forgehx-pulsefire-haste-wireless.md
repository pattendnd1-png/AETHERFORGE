# ForgeHX Pulsefire Haste Wireless Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make HyperX Pulsefire Haste Wireless (`03f0:028e`, wired transport `03f0:048e`) leave Diagnostic Only by adding exact device registration, native DPI/polling control, udev access, and compatibility-backend fallback.

**Architecture:** `forgehx-device` owns exact Haste identity and the native 64-byte HID packet transport. The driver registry advertises only implemented native capabilities; `forgehx-daemon` routes those capabilities to the native controller and continues to route lighting/bindings through OpenRGB/ratbag when available. Exact udev VID/PID rules provide access without broad HP VID permissions.

**Tech Stack:** Rust 2021, hidapi `linux-native`, ForgeHX IPC v3/profile schema v2, ratbagd compatibility backend, OpenRGB compatibility backend, Arch udev/systemd packaging.

## Global Constraints

- Exact HyperX Pulsefire Haste Wireless IDs: `03f0:028e` wireless and `03f0:048e` wired transport.
- Driver ID: `hyperx-pulsefire-haste-wireless-v1`.
- Native driver advertises only `Dpi` and `PollingRate` plus diagnostics until more native handlers exist.
- Native selection may never shadow a compatibility backend with an unimplemented native command.
- No firmware flashing or writes to unmatched HP hardware.
- HID writes are exactly 64 bytes and only to the matched Haste configuration interface.
- Package remains ForgeHX `0.3.0`; Arch `pkgrel` becomes `5`.

---

### Task 1: Register the Haste family and promote support safely

**Files:**
- Modify: `crates/forgehx-device/src/drivers.rs`
- Test: `crates/forgehx-device/src/drivers.rs`

**Interfaces:**
- Produces `driver_for(0x03f0, 0x028e|0x048e) -> hyperx-pulsefire-haste-wireless-v1`.
- Advertises exactly `Capability::Dpi` and `Capability::PollingRate`.

- [ ] Add tests asserting both PIDs map to the Haste wireless driver and an unrelated HP PID does not.
- [ ] Run the targeted test and confirm RED when Cargo is available; otherwise record tool unavailability.
- [ ] Add the `DriverDefinition` entry with `complete: false` and implemented capabilities only.
- [ ] Re-run the targeted test/static guard.
- [ ] Commit `feat(device): register Pulsefire Haste Wireless`.

### Task 2: Add native Haste HID packet transport

**Files:**
- Create: `crates/forgehx-device/src/pulsefire_haste.rs`
- Modify: `crates/forgehx-device/src/lib.rs`
- Test: `crates/forgehx-device/src/pulsefire_haste.rs`

**Interfaces:**
- Produces `PulsefireHasteWireless::{from_device,set_polling_rate,set_dpi}`.
- Produces pure `polling_rate_packet(hz)` and `dpi_packet(stage,dpi)` builders returning `[u8;64]`.

- [ ] Write unit tests for allowed rates 125/250/500/1000 and rejection of unsupported rates.
- [ ] Write unit tests for DPI range 200..=16000 in 100-DPI steps and stage range 0..5.
- [ ] Write a pure interface-selection test preferring interface 2 / usage page `0xff00` for `03f0:028e|048e`.
- [ ] Implement minimal packet builders and exact-device hidraw opening/writes.
- [ ] Re-run tests/static syntax checks.
- [ ] Commit `feat(device): add native Pulsefire Haste transport`.

### Task 3: Route native DPI and polling through the daemon

**Files:**
- Modify: `crates/forgehx-daemon/src/lib.rs`
- Test: `crates/forgehx-daemon/src/lib.rs`

**Interfaces:**
- Native `Capability::Dpi` routes to `PulsefireHasteWireless::set_dpi` for the selected device.
- Native `Capability::PollingRate` routes to `PulsefireHasteWireless::set_polling_rate`.
- ratbag remains fallback for capabilities not owned natively.

- [ ] Add daemon tests proving a Haste device becomes `PartiallySupported` and native owns only DPI/polling.
- [ ] Add a regression test proving native Haste registration does not claim bindings/lighting.
- [ ] Implement native controller lookup from the discovered device’s HID interfaces.
- [ ] Route DPI stages/active stage and polling rate to native Haste handlers.
- [ ] Re-run daemon/static checks.
- [ ] Commit `feat(daemon): control Pulsefire Haste DPI and polling`.

### Task 4: Fix Haste permissions and package guards

**Files:**
- Modify: `packaging/udev/70-forgehx.rules`
- Modify: `scripts/check-package-layout.sh`
- Modify: `README.md`

**Interfaces:**
- Exact `uaccess` rules cover `03f0:028e` and `03f0:048e`.
- Package regression guard rejects a release missing Haste registration or permissions.

- [ ] Add failing package guards for driver ID and both udev PIDs.
- [ ] Add exact Haste udev rules without broadening all `03f0` HP devices.
- [ ] Document Haste support and backend fallback behavior.
- [ ] Run shell/package guards.
- [ ] Commit `packaging: enable Pulsefire Haste Wireless access`.

### Task 5: Release ForgeHX 0.3.0-5

**Files:**
- Modify: `PKGBUILD`
- Generate: `/mnt/data/ForgeHX-0.3.0-source-r5.tar.gz`
- Generate: `/mnt/data/ForgeHX-0.3.0-Arch-BuildKit-r5.tar.gz`

**Interfaces:**
- `pkgrel=5` source archive is pinned by SHA-256 in the build kit.

- [ ] Set `pkgrel=5`.
- [ ] Run `git diff --check`, manifest parse, package guards, shell syntax, and Haste-specific source scans.
- [ ] Attempt `cargo test --workspace` and `cargo build --workspace --release`; report tool absence instead of claiming success if unavailable.
- [ ] Commit `release: prepare ForgeHX 0.3.0-5`.
- [ ] Build source archive from the committed tree, calculate SHA-256, create the Arch Build Kit, and verify both from fresh extraction.
