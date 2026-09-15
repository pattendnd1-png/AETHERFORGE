# ReForge Logitech Linux v0.5.0 Reliability & Daily-Use Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make ReForge v0.5.0 an always-on control center with stable device identity, automatic reconnect, live telemetry, safe state restore, complete profile lifecycle management, persistent GUI preferences, and redacted diagnostics.

**Architecture:** Keep the v0.4 provider/control stack intact. Add lifecycle/telemetry/diagnostic models to `reforge-core`, a stable registry and discovery loop to `reforge-daemon`, name-keyed profile and device selection to the GUI, and additive RPC variants so v0.4 requests remain valid.

**Tech Stack:** Rust 2024 edition, serde/serde_json, Unix-domain JSON RPC, hidapi HID++, V4L2 command integration, PipeWire command integration, eframe/egui, systemd user service.

**Spec:** `docs/superpowers/specs/2026-08-19-reforge-logitech-v05-reliability-design.md`

## Global Constraints

- Start from ReForge Logitech Linux v0.4.0 and preserve all existing HID++/V4L2/PipeWire control paths.
- Existing v0.4.0 `profiles.json` must load unchanged.
- G515 LS TKL must remain one physical keyboard and retain per-key RGB support.
- Restore only controls still advertised as writable/profile-eligible after reconnect.
- Failed test/build/install gates must not replace a working installation.
- Add no arbitrary raw USB/HID write surface.
- Keep the daemon user-session scoped on its existing Unix socket.
- Default registry offline retention is 10 minutes; fallback discovery cadence is 2 seconds; GUI telemetry cadence is 1 second; diagnostics ring holds 200 events.

---

### Task 1: Lifecycle, telemetry, diagnostics, and UI preference core models

**Files:**
- Modify: `crates/reforge-core/src/model.rs`
- Create: `crates/reforge-core/src/diagnostics.rs`
- Create: `crates/reforge-core/src/preferences.rs`
- Modify: `crates/reforge-core/src/paths.rs`
- Modify: `crates/reforge-core/src/lib.rs`
- Test: inline unit tests in the new/modified modules

**Interfaces:**
- Produces `DevicePresence`, `DeviceRuntimeInfo`, `DeviceTelemetry`, `DesiredDeviceState`, `DiagnosticLevel`, `DiagnosticEvent`, `DiagnosticSnapshot`, `DiagnosticBundle`, `UiPreferences`.
- Produces `paths::ui_preferences_file()` and pure `diagnostics::redact_snapshot(snapshot, include_device_identifiers)`.

- [ ] **Step 1: Write lifecycle/telemetry model tests.** Add serde round-trip tests for `DevicePresence`, `DeviceRuntimeInfo`, and `DeviceTelemetry`, plus a backwards/default test proving missing runtime-only fields do not alter `DeviceSummary` JSON.
- [ ] **Step 2: Write diagnostics redaction tests.** Construct a snapshot containing a serial/unit identifier and home path; assert default redaction removes identifiers, replaces the home prefix with `$HOME`, preserves product/feature/control IDs, and never exposes raw HID payload fields.
- [ ] **Step 3: Write UI preference persistence tests.** Use explicit temp paths in helper methods so tests do not touch the real home directory; assert missing/corrupt JSON returns defaults and valid JSON round-trips selected device/profile/page/theme/live-preview.
- [ ] **Step 4: Implement the core types and pure redaction/persistence helpers.** Keep serde defaults on every additive field used by persisted state.
- [ ] **Step 5: Export new modules/types from `lib.rs` and add `ui_preferences_file()` to `paths.rs`.
- [ ] **Step 6: Run `cargo test -p reforge-core` and commit `feat(core): add lifecycle telemetry diagnostics models`.

### Task 2: Complete profile lifecycle operations with backwards compatibility

**Files:**
- Modify: `crates/reforge-core/src/profile.rs`
- Test: inline profile tests plus `crates/reforge-core/tests/profile_matching.rs`

**Interfaces:**
- Produces `ProfileStore::get`, `rename`, `remove`, `clone_as`, `import_json`, and `export_json` exactly as declared by the spec.
- Existing `load`, `save`, and `upsert` remain callable.

- [ ] **Step 1: Add failing tests for trimmed/non-empty names, duplicate rejection, rename, clone, delete, import with and without replace, export selection, and legacy v0.4 JSON import.**
- [ ] **Step 2: Add `normalize_profile_name(name: &str) -> Result<String, String>` and use it for all new lifecycle operations.
- [ ] **Step 3: Implement the name-based operations while preserving atomic `save()` semantics and existing `version=4` normalization.
- [ ] **Step 4: Run `cargo test -p reforge-core profile` and full `cargo test -p reforge-core`; commit `feat(profiles): add lifecycle import export operations`.

### Task 3: Add additive RPC surface and JSON round-trip coverage

**Files:**
- Modify: `crates/reforge-core/src/rpc.rs`
- Modify: `crates/reforge-core/tests/rpc_roundtrip.rs`

**Interfaces:**
- Adds requests: `RescanDevices`, `ListDeviceRuntime`, `GetDeviceTelemetry`, `RenameProfile`, `DeleteProfile`, `CloneProfile`, `ImportProfiles`, `ExportProfiles`, `GetDiagnostics`, `ClearDiagnostics`.
- Adds data: `DeviceRuntime`, `Telemetry`, `ExportedProfiles`, `Diagnostics`.

- [ ] **Step 1: Add one round-trip assertion for every new request and data variant while retaining existing v0.4 tests.
- [ ] **Step 2: Implement the enum variants/imports only; do not remove or rename any v0.4 RPC variant.
- [ ] **Step 3: Run `cargo test -p reforge-core rpc` and commit `feat(rpc): add reliability lifecycle operations`.

### Task 4: Stable daemon registry, discovery loop, and desired-state restore

**Files:**
- Create: `crates/reforge-daemon/src/registry.rs`
- Create: `crates/reforge-daemon/src/diagnostics.rs`
- Modify: `crates/reforge-daemon/src/main.rs`
- Test: unit tests in `registry.rs` and daemon-local pure helper tests

**Interfaces:**
- `DeviceRegistry::merge_scan(now_ms, devices) -> Vec<RegistryTransition>` updates online/offline/reconnecting lifecycle without changing stable keys.
- `DeviceRegistry::runtime_list() -> Vec<DeviceRuntimeInfo>` returns deterministic sorted output.
- `RuntimeState` gains registry, desired states, latest telemetry, diagnostics ring, and last applied profile per device.
- `discovery_loop(shared)` scans every 2 seconds and calls safe restore on offline->online transitions.

- [ ] **Step 1: Write pure registry tests for new device, refresh, missing->offline, same-key reconnect, reconnect count increment, deterministic sorting, and expiry after 10 minutes.
- [ ] **Step 2: Implement `registry.rs` without hardware calls; registry transitions carry old/new presence and stable key.
- [ ] **Step 3: Write desired-state restore ordering tests using a pure helper that sorts advertised control IDs by `ControlGroup` in the required order.
- [ ] **Step 4: Replace `RuntimeState.devices: HashMap<String, DeviceSummary>` with the registry and retain helper access to the current cached `DeviceSummary`.
- [ ] **Step 5: Make every successful `SetDpi`, `SetLighting`, `SetKeyColor`, `SetControl`, and `ApplyProfile` update `DesiredDeviceState` only after the hardware call succeeds.
- [ ] **Step 6: Implement `restore_device_state` so it revalidates every control against the fresh summary, skips missing/non-writable/non-profile-eligible controls, restores ordered groups, then lighting, and records partial failures without taking the device offline.
- [ ] **Step 7: Add the 2-second `discovery_loop`; initial scan occurs before listener service, disappeared devices are marked offline, host effect sessions for them are stopped, and returning keys restore safely.
- [ ] **Step 8: Run daemon/unit tests and commit `feat(daemon): add stable registry and reconnect restore`.

### Task 5: Lightweight telemetry, diagnostics ring, and reliability RPC handlers

**Files:**
- Modify: `crates/reforge-daemon/src/providers.rs`
- Modify: `crates/reforge-daemon/src/diagnostics.rs`
- Modify: `crates/reforge-daemon/src/main.rs`

**Interfaces:**
- `providers::read_telemetry(device, cached) -> DeviceTelemetry` performs only lightweight provider reads.
- Diagnostic ring exposes `push`, `snapshot`, and `clear`, bounded to 200 entries.
- New RPC handlers operate on cached registry state and profile store without forcing full device discovery, except `RescanDevices`.

- [ ] **Step 1: Add bounded-ring tests proving the 201st event evicts the oldest and clear leaves zero events.
- [ ] **Step 2: Implement provider-health telemetry using cached provider presence; use HID++ battery/DPI read helpers only where already supported and do not enumerate the HID++ feature tree.
- [ ] **Step 3: Implement `ListDeviceRuntime` from registry cache and `GetDeviceTelemetry` from cached endpoints; hardware disappearance sets presence offline and returns latest telemetry.
- [ ] **Step 4: Implement profile lifecycle RPC handlers using Task 2 methods and persist after successful mutations.
- [ ] **Step 5: Implement `GetDiagnostics`/`ClearDiagnostics`; record discovery transitions, reconnect results, profile auto-switch, provider/request failures, and rejected restore entries.
- [ ] **Step 6: Implement `RescanDevices` as an immediate scan/merge rather than a second registry implementation.
- [ ] **Step 7: Run workspace daemon/core tests and commit `feat(daemon): add telemetry diagnostics reliability rpc`.

### Task 6: Stable GUI selection, polling, preferences, profile CRUD, and diagnostics page

**Files:**
- Modify: `crates/reforge-gui/src/app.rs`
- Create: `crates/reforge-gui/src/diagnostics.rs`
- Modify: `crates/reforge-gui/src/main.rs`

**Interfaces:**
- Replace index selection with `selected_device_key: Option<String>` and `selected_profile_name: Option<String>`.
- GUI calls `ListDeviceRuntime` on a 1-second cadence and fetches selected-device telemetry without triggering `ListDevices`.
- UI preferences load at startup and save on relevant state changes.

- [ ] **Step 1: Add pure GUI helper tests for preserving selection by stable key across reorder/removal and for profile selection by name across sorting.
- [ ] **Step 2: Change device/profile selection fields and all call sites to stable strings; offline devices remain selectable and write buttons are disabled when presence is not `Online`.
- [ ] **Step 3: Add one-second polling timestamps; refresh runtime list and selected telemetry while preserving page/selection.
- [ ] **Step 4: Load `UiPreferences` in `new()`, apply theme/page/selection/live-preview, and save preferences when those values change.
- [ ] **Step 5: Expand Profiles page with rename/clone/delete confirmation/import/export buttons wired to the new RPCs. Import reads JSON in the GUI process; export writes returned JSON in the GUI process.
- [ ] **Step 6: Add Diagnostics page showing versions, providers, lifecycle state, reconnect counts, recent events, plus redacted export and optional identifier inclusion.
- [ ] **Step 7: Add Settings buttons for `Rescan devices` and explicit-click `systemctl --user restart reforge-logitechd.service`; connection failures after restart remain status messages rather than app crashes.
- [ ] **Step 8: Run GUI/core tests and commit `feat(gui): add reliability control center workflows`.

### Task 7: CLI reliability commands and diagnostics access

**Files:**
- Modify: `crates/reforge-cli/src/main.rs`

**Interfaces:**
- Adds CLI commands for runtime listing, telemetry, rescan, profile rename/clone/delete/import/export, diagnostics show/clear/export while preserving all v0.4 commands.

- [ ] **Step 1: Add parser tests for each new subcommand if the existing CLI has test hooks; otherwise extract command->RPC mapping into a pure helper with unit tests.
- [ ] **Step 2: Implement new commands using the exact RPC variants from Task 3; diagnostics export uses the same core redaction helper.
- [ ] **Step 3: Run `cargo test -p reforge-logitechctl` (or the actual CLI package name) and commit `feat(cli): expose reliability and diagnostics commands`.

### Task 8: Version, packaging, release documentation, and full verification

**Files:**
- Modify: `Cargo.toml`
- Modify: `README.md`
- Modify: `RELEASE_NOTES.md`
- Modify: `INSTALL-UPGRADE.txt`
- Modify: `packaging/arch/PKGBUILD`
- Review: `scripts/install-arch.sh`, `scripts/build-release.sh`, `scripts/make-dist.sh`

**Interfaces:**
- Workspace/package version becomes `0.5.0`.
- Installer/package build keeps the test-before-release-build gate.

- [ ] **Step 1: Update version/package metadata and document hot-plug, telemetry, profile lifecycle, preferences, diagnostics, and v0.4 compatibility.
- [ ] **Step 2: Confirm `PKGBUILD` runs `cargo test --workspace` before `cargo build --release --workspace` and does not install on either failure.
- [ ] **Step 3: Run `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --release --workspace`.
- [ ] **Step 4: Run `bash -n scripts/*.sh`, parse all Cargo manifests, `git diff --check`, archive extraction tests, and SHA-256 verification.
- [ ] **Step 5: Commit `release: ReForge Logitech Linux v0.5.0` and build canonical ZIP/TAR/INSTALL/SHA256 artifacts without duplicate names.
