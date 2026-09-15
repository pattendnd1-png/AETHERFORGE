# ReForge Logitech Linux v0.6.0 Logitech Services Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add persistent Logitech HID++ sessions, Linux hotplug service monitoring, fwupd/LVFS firmware integration, service/session status, and GUI/CLI surfaces without regressing v0.5 controls.

**Architecture:** `reforge-hid` owns reusable HID++ sessions and packet classification. `reforge-daemon` owns a service registry, hotplug monitor, fwupd adapter, and routes HID++ operations through persistent sessions. `reforge-core` defines typed service/firmware/session RPC data. GUI and CLI consume those RPCs; firmware privilege remains in fwupd/polkit.

**Tech Stack:** Rust 2024; hidapi 2.6.6; serde/serde_json; std process/thread/sync; Linux `udevadm`; `fwupdmgr --json`; egui/eframe.

**Spec:** `docs/superpowers/specs/2026-08-23-reforge-logitech-v06-services-design.md`

## Global Constraints

- No undocumented G HUB/cloud authentication.
- No raw HID writer exposed to users.
- No direct firmware flashing/downloading by ReForge.
- Existing v0.5 profiles/preferences/RPCs remain compatible.
- G515 receiver mirror deduplication remains enforced.
- Firmware operations use fwupd/polkit trust and authorization.
- Arch package runs tests before release build and aborts on failure.

---

### Task 1: Core service, session, and firmware types

**Files:**
- Create: `crates/reforge-core/src/services.rs`
- Modify: `crates/reforge-core/src/lib.rs`
- Modify: `crates/reforge-core/src/rpc.rs`
- Test: `crates/reforge-core/tests/rpc_roundtrip.rs`

**Interfaces:**
- Produces `ServiceKind`, `ServiceState`, `ServiceStatus`, `HidppSessionStatus`, `FirmwareDevice`, `FirmwareRelease`, `FirmwareInstallResult`.
- Extends `RpcRequest`/`RpcData` with service/session/firmware variants.

- [ ] Write serialization/round-trip tests for every new type/RPC variant.
- [ ] Verify tests fail before implementation with missing types/variants.
- [ ] Implement types using serde snake_case enums and stable fields suitable for GUI/CLI.
- [ ] Re-export from `lib.rs` and extend RPC enums.
- [ ] Run core tests and commit.

### Task 2: Persistent HID++ session manager and packet classification

**Files:**
- Create: `crates/reforge-hid/src/session.rs`
- Modify: `crates/reforge-hid/src/lib.rs`
- Test: `crates/reforge-hid/tests/helpers.rs`
- Test: `crates/reforge-hid/tests/session.rs`

**Interfaces:**
- Produces `SessionManager::ensure(&DeviceSummary)`, `remove`, `status`, `status_all`, `with_device`, `poll_notifications`.
- Produces `HidppPacketKind::{Reply, Notification, Error, Unknown}` classifier.

- [ ] Add tests proving replies are matched to requests and unmatched HID++ packets classify as notifications rather than replies.
- [ ] Add tests for session state/reconnect counters using a pure state helper independent of real hidraw hardware.
- [ ] Implement `PersistentSession` with one `Mutex<HidDevice>` per physical device and cached device metadata.
- [ ] Implement `SessionManager` with `Mutex<BTreeMap<String, Arc<PersistentSession>>>`.
- [ ] Reuse existing `open_summary_device`; keep compatibility wrappers for one-shot callers.
- [ ] Add nonblocking notification polling serialized through each session mutex.
- [ ] Run HID tests and commit.

### Task 3: Daemon service registry and persistent HID++ routing

**Files:**
- Create: `crates/reforge-daemon/src/services.rs`
- Modify: `crates/reforge-daemon/src/main.rs`
- Modify: `crates/reforge-daemon/src/providers.rs`

**Interfaces:**
- `AppState { runtime: Mutex<RuntimeState>, sessions: SessionManager, services: Mutex<ServiceRegistry> }`.
- Existing device/control RPC behavior remains unchanged externally.

- [ ] Add unit tests for service state transitions `Ready -> Degraded -> Ready` and sanitized messages.
- [ ] Replace global daemon state alias with `Arc<AppState>`.
- [ ] On discovery, `ensure` sessions for online HID++ devices and remove expired/offline sessions.
- [ ] Route HID++ DPI/lighting/control reads/writes through `SessionManager::with_device`; leave V4L2/PipeWire provider calls unchanged.
- [ ] Poll HID++ notifications and update telemetry/diagnostics without interpreting unknown notifications as commands.
- [ ] Add `ListServices`, `ReconnectService`, and `GetSessionStatus` RPC handlers.
- [ ] Run daemon tests and commit.

### Task 4: udev hotplug service with fallback discovery

**Files:**
- Create: `crates/reforge-daemon/src/udev_monitor.rs`
- Modify: `crates/reforge-daemon/src/main.rs`
- Test: module tests in `udev_monitor.rs`

**Interfaces:**
- Produces `UdevEvent { action, subsystem, devname }` and `parse_udevadm_line`/debounce helper.
- Daemon schedules `refresh_devices` from add/remove/change events.

- [ ] Add parser tests for representative `udevadm monitor --property` event blocks and ignored unrelated events.
- [ ] Implement child process launch for `udevadm monitor --udev --property --subsystem-match=hidraw --subsystem-match=usb`.
- [ ] Debounce refreshes to avoid multiple scans for one physical attach.
- [ ] Mark Udev service degraded on missing executable/child exit and retain periodic fallback scan.
- [ ] Run daemon tests and commit.

### Task 5: fwupd/LVFS service adapter

**Files:**
- Create: `crates/reforge-daemon/src/fwupd.rs`
- Modify: `crates/reforge-daemon/src/main.rs`
- Add fixtures: `crates/reforge-daemon/tests/fixtures/fwupd-devices.json`, `fwupd-releases.json`

**Interfaces:**
- Produces `FwupdClient::probe`, `devices`, `refresh_metadata`, `releases`, `install_update`.
- Produces pure parsers for fwupdmgr JSON and `match_logitech_device`.

- [ ] Add fixture tests for Logitech filtering/matching and release parsing.
- [ ] Implement command execution with bounded timeout behavior via child polling and sanitized stderr.
- [ ] Use `fwupdmgr get-devices --json`, `get-releases <id> --json`, `refresh --force`, and `update <id>`/supported update invocation; never append force/downgrade flags for installation.
- [ ] Map fwupd absence to `Unavailable`, command failure to `Degraded/Error`, successful probe to `Ready`.
- [ ] Add firmware RPC handlers and explicit-install result metadata.
- [ ] Run daemon tests and commit.

### Task 6: GUI Services and Firmware pages

**Files:**
- Modify: `crates/reforge-gui/src/app.rs`
- Create: `crates/reforge-gui/src/services.rs`
- Create: `crates/reforge-gui/src/firmware.rs`
- Modify: `crates/reforge-gui/src/main.rs` if module wiring is required.

**Interfaces:**
- Adds `Page::Services` and `Page::Firmware`.
- Consumes service/session/firmware RPC data only; no direct subprocess/hidraw access from GUI.

- [ ] Add page enum/persistence round-trip tests.
- [ ] Render service cards with state/backend/version/message and reconnect action.
- [ ] Render per-device HID++ session status and receiver route.
- [ ] Render fwupd-matched device/release/update UI with explicit refresh/install buttons and confirmation text in-app.
- [ ] Disable update action when fwupd is unavailable or no update exists.
- [ ] Run GUI tests and commit.

### Task 7: CLI service and firmware commands

**Files:**
- Modify: `crates/reforge-cli/src/main.rs`

**Interfaces:**
- Commands: `services`, `services reconnect`, `session`, `firmware devices`, `firmware refresh`, `firmware releases`, `firmware update`.

- [ ] Add clap parser tests for every command.
- [ ] Implement RPC calls and concise output for service/session/firmware records.
- [ ] Require explicit firmware device ID for update; no “update all” in ReForge v0.6.
- [ ] Run CLI tests and commit.

### Task 8: Packaging, permissions, versioning, and regression gates

**Files:**
- Modify: `Cargo.toml`
- Modify: all crate manifests only if version inheritance requires no direct edits
- Modify: `packaging/arch/PKGBUILD`
- Modify: `packaging/udev/70-reforge-logitech.rules`
- Modify: `README.md`
- Modify: `RELEASE_NOTES.md`
- Modify: `INSTALL-UPGRADE.txt`
- Modify: `scripts/make-dist.sh`

**Interfaces:**
- Release version `0.6.0`.
- udev rule grants `uaccess`, never `0666`.
- fwupd is optional/recommended for firmware functionality.

- [ ] Add a regression check/script assertion that udev rules contain `TAG+="uaccess"` and not `MODE="0666"`/`MODE="0666"` variants.
- [ ] Preserve `cargo test --workspace` before `cargo build --release --workspace` in Arch package.
- [ ] Update documentation and command examples.
- [ ] Run `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --release --workspace` when Rust is available.
- [ ] Run `bash -n` on shell scripts, `git diff --check`, manifest parsing, archive integrity, and checksum verification.
- [ ] Create one canonical ZIP and TAR.GZ plus install/checksum files; commit release tree.
