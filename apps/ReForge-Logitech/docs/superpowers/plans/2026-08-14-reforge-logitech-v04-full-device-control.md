# ReForge Logitech v0.4 Full Device Control Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand ReForge v0.3.1 so every positively identified writable Logitech HID++, V4L2, and PipeWire capability is exposed through typed controls, profiles, CLI, daemon RPC, and capability-driven GUI sections without exposing arbitrary raw device writes.

**Architecture:** Keep the existing crate seams. `reforge-core` owns provider-neutral typed control values and profile snapshots; `reforge-hid` probes and writes verified HID++ capabilities; `reforge-daemon` merges Linux providers and routes `SetControl`; CLI/GUI consume only advertised `DeviceControl` metadata. Existing specialized DPI/lighting RPCs remain compatible but HID++ generic controls use the same validated router.

**Tech Stack:** Rust 2024, serde/serde_json, hidapi 2.6.6, clap, eframe/egui 0.35, Unix-domain JSON RPC, V4L2 `v4l2-ctl`, PipeWire `wpctl`, Arch `makepkg`/systemd/udev.

## Global Constraints

- Preserve v0.3.1 daemon socket path, binary names, systemd user service, udev model, and desktop launcher.
- No arbitrary raw HID/USB report RPC/CLI/GUI.
- Every writable HID++ control must be feature-gated, typed, range/choice validated, and routed only through `reforge-hid`.
- V4L2/PipeWire helpers may be absent without breaking HID++ discovery.
- Profiles must load v0.3.1 data and persist only profile-eligible writable controls.
- G515 LS TKL must appear once and retain 95-zone per-key RGB plus brightness.
- Arch installer must run `cargo test --workspace` before `cargo build --release --workspace` and install only after both succeed.

---

### Task 1: Typed control values and profile migration

**Files:**
- Modify: `crates/reforge-core/src/model.rs`
- Modify: `crates/reforge-core/src/profile.rs`
- Modify: `crates/reforge-core/src/rpc.rs`
- Modify: `crates/reforge-core/src/lib.rs`
- Test: `crates/reforge-core/tests/rpc_roundtrip.rs`
- Test: `crates/reforge-core/tests/profile_matching.rs`

**Interfaces:**
- Consumes: existing `ProviderKind`, `DeviceControl`, `Profile`, `RpcRequest::SetControl`.
- Produces: `ControlValue`, extended `ControlKind`, `ControlGroup`, `ReadbackKind`, `DeviceControl::{value,profile_eligible,readback,group}`, `Profile.controls: BTreeMap<String, ControlValue>`, `RpcRequest::{GetControl,SetControl}` and `RpcData::Control`.

- [ ] **Step 1: Write failing serde and profile-migration tests**

Add tests proving scalar, boolean, choice, vector/EQ, and null telemetry values round-trip and legacy `controls: {"id": 42}` JSON still loads:

```rust
#[test]
fn structured_control_values_round_trip() {
    let values = [
        ControlValue::Bool(true),
        ControlValue::Int(42),
        ControlValue::Choice(3),
        ControlValue::Vector(vec![100, 0, -2, 250]),
        ControlValue::None,
    ];
    for value in values {
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<ControlValue>(&json).unwrap(), value);
    }
}

#[test]
fn legacy_numeric_profile_controls_migrate() {
    let json = r#"{"name":"Old","product_id":null,"serial":null,"dpi":null,"controls":{"hidpp:report_rate":1}}"#;
    let profile: Profile = serde_json::from_str(json).unwrap();
    assert_eq!(profile.controls["hidpp:report_rate"], ControlValue::Int(1));
}
```

- [ ] **Step 2: Run targeted core tests and confirm RED**

Run:

```bash
cargo test -p reforge-core --tests
```

Expected: compile/test failure because `ControlValue` and new metadata do not exist yet.

- [ ] **Step 3: Implement typed model with backwards-compatible numeric deserialization**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ControlValue {
    Bool(bool),
    Int(i64),
    Choice(i64),
    Vector(Vec<i64>),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlGroup { Overview, Assignments, Performance, Lighting, Audio, Camera, Power, Profiles, Receiver, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadbackKind { Readable, CanonicalWriteOnly, Telemetry }
```

Extend `ControlKind` with `Vector` and `Status`. Extend `DeviceControl` with serde-defaulted `group`, `readback`, and `profile_eligible`; change `value` to `ControlValue`. Add helper accessors `as_i64()` / `as_bool()` so existing scalar code migrates cleanly.

- [ ] **Step 4: Update profile store to version 4 while accepting v0.3.1 JSON**

Use serde defaults and a custom `deserialize_control_map` accepting either legacy JSON numbers or new tagged/untagged typed values. `ProfileStore::load()` sets `version = 4` after successful migration.

- [ ] **Step 5: Extend generic control RPC**

Change `SetControl.value` to `ControlValue`, add:

```rust
GetControl { key: String, control_id: String }
```

and:

```rust
RpcData::Control(ControlValue)
```

- [ ] **Step 6: Run core tests and full workspace check**

```bash
cargo test -p reforge-core --tests
cargo check --workspace
```

Expected: PASS; dependent crates may require mechanical updates in the next task only if `cargo check` identifies scalar call sites.

- [ ] **Step 7: Commit**

```bash
git add crates/reforge-core
git commit -m "feat(core): add typed device controls"
```

### Task 2: HID++ control adapter registry and performance/input controls

**Files:**
- Create: `crates/reforge-hid/src/control.rs`
- Create: `crates/reforge-hid/src/adapters/mod.rs`
- Create: `crates/reforge-hid/src/adapters/performance.rs`
- Modify: `crates/reforge-hid/src/lib.rs`
- Modify: `crates/reforge-protocol/src/feature.rs`
- Test: `crates/reforge-hid/tests/control_adapters.rs`

**Interfaces:**
- Consumes: `DeviceSummary.features`, existing HID++ `transact_retry`, `open_summary_device`, DPI implementation.
- Produces: `reforge_hid::probe_controls(handle,index,features) -> Vec<DeviceControl>`, `reforge_hid::set_control(device, control, value) -> Result<ControlValue,String>`, performance control IDs under `hidpp:*`.

- [ ] **Step 1: Write failing fake-transport adapter tests**

Tests cover feature-gating and validation for report rate, extended report rate, pointer speed, SmartShift, hi-res wheel flags/direction, thumb wheel, Fn inversion, G-key diversion, and onboard profile selection. Use pure request encoders/parsers or a small internal `HidppIo` trait so hardware is not required.

- [ ] **Step 2: Run adapter tests and confirm RED**

```bash
cargo test -p reforge-hid --test control_adapters
```

Expected: failure because adapter registry/control functions do not exist.

- [ ] **Step 3: Add verified feature constants**

Add HID++ feature IDs used by the adapters to `reforge-protocol/src/feature.rs`, using the same IDs already represented by current Solaar HID++ constants/reference behavior. Keep names explicit: `REPORT_RATE`, `EXTENDED_ADJUSTABLE_REPORT_RATE`, `POINTER_SPEED`, `SMART_SHIFT`, `SMART_SHIFT_ENHANCED`, `HIRES_WHEEL`, `THUMB_WHEEL`, `FN_INVERSION`, `NEW_FN_INVERSION`, `GKEY`, `ONBOARD_PROFILES`.

- [ ] **Step 4: Implement internal adapter interface**

Use:

```rust
pub(crate) trait ControlAdapter {
    fn probe(&self, io: &dyn HidppIo, ctx: &ProbeContext) -> Vec<DeviceControl>;
    fn write(&self, io: &dyn HidppIo, ctx: &WriteContext, control: &DeviceControl, value: &ControlValue)
        -> Result<ControlValue, String>;
}
```

The public crate still exposes only typed `DeviceControl` and `set_control`; no raw payload escapes the crate.

- [ ] **Step 5: Implement performance/input probing and writes**

For each present feature, read device-supported values first and advertise only validated controls. Keep bit-preserving read-modify-write for wheel flags. Treat DPI as a performance control alias while retaining `set_dpi()` compatibility.

- [ ] **Step 6: Attach HID++ controls during `scan_logitech_devices()`**

Populate `DeviceSummary.controls` after feature enumeration. Receiver mirror fingerprints must include stable control capability IDs but not mutable current values.

- [ ] **Step 7: Run HID tests and workspace check**

```bash
cargo test -p reforge-hid
cargo check --workspace
```

- [ ] **Step 8: Commit**

```bash
git add crates/reforge-hid crates/reforge-protocol
git commit -m "feat(hid): add performance control adapters"
```

### Task 3: Assignments, onboard profiles, analog keyboard tuning, and power telemetry

**Files:**
- Create: `crates/reforge-hid/src/adapters/assignments.rs`
- Create: `crates/reforge-hid/src/adapters/power.rs`
- Modify: `crates/reforge-hid/src/adapters/mod.rs`
- Modify: `crates/reforge-protocol/src/feature.rs`
- Test: `crates/reforge-hid/tests/assignments_power.rs`

**Interfaces:**
- Consumes: adapter registry from Task 2.
- Produces: capability-gated assignment controls, analog-key controls, battery/charging telemetry, writable auto-sleep/power controls where verified.

- [ ] **Step 1: Write failing tests for feature-gating, assignment validation, and telemetry**

Cover: no assignment control unless reprogrammable-key feature reports a writable target; onboard profile choice validation; G-key diversion bool; analog actuation/rapid-trigger/haptics preserving reserved bits; battery percentage/status read-only controls; auto-sleep range validation.

- [ ] **Step 2: Run tests and confirm RED**

```bash
cargo test -p reforge-hid --test assignments_power
```

- [ ] **Step 3: Implement assignment adapters**

Represent assignment choices as typed menu choices with semantic backend IDs. Do not expose arbitrary HID usage integers unless they came from the device-supported mapping set. Mark destructive reset/unpair-like actions `profile_eligible = false`.

- [ ] **Step 4: Implement analog-key tuning**

For HID++ analog-button config, decode logical values from wire bytes and preserve the firmware-managed sensitivity bit during rapid-trigger writes. Expose each verified analog key/control as grouped `Performance` ranges.

- [ ] **Step 5: Implement power adapter**

Expose readable battery/charge status as `ControlKind::Status`, `ReadbackKind::Telemetry`, `writable=false`, `profile_eligible=false`. Expose auto-sleep only if its feature version and payload shape are known.

- [ ] **Step 6: Run tests/check and commit**

```bash
cargo test -p reforge-hid
cargo check --workspace
git add crates/reforge-hid crates/reforge-protocol
git commit -m "feat(hid): add assignments and power controls"
```

### Task 4: HID++ audio, microphone, EQ, and headset lighting controls

**Files:**
- Create: `crates/reforge-hid/src/adapters/audio.rs`
- Modify: `crates/reforge-hid/src/adapters/mod.rs`
- Modify: `crates/reforge-protocol/src/feature.rs`
- Test: `crates/reforge-hid/tests/audio_controls.rs`

**Interfaces:**
- Consumes: typed vectors from Task 1 and adapter registry from Task 2.
- Produces: HID++ audio controls grouped under `Audio`: mute/volume when verified, sidetone, mic controls, structured EQ, headset lighting, battery/power telemetry integration.

- [ ] **Step 1: Write failing audio adapter tests**

Use synthetic responses for sidetone ranges, mute bools, basic graphic EQ bands, advanced EQ band tuples, mic gain, and headset RGB/effect controls. Verify malformed band counts/ranges are rejected.

- [ ] **Step 2: Run tests and confirm RED**

```bash
cargo test -p reforge-hid --test audio_controls
```

- [ ] **Step 3: Implement audio feature probing**

Only create controls after a successful feature-specific info/read query. `Vector` EQ values carry explicit ordered numeric band data; adapter metadata/labels identify frequency/band positions. Unknown EQ feature versions remain unadvertised.

- [ ] **Step 4: Implement validated writes/readback**

Range-check every band/gain before encoding. For readable features, re-read and compare after write. For write-only verified features, return/persist canonical typed state.

- [ ] **Step 5: Preserve PipeWire separation**

HID++ controls use IDs such as `hidpp:sidetone` and `hidpp:eq:*`; PipeWire transport controls retain `pipewire:volume` / `pipewire:mute`, so both may coexist on one device card.

- [ ] **Step 6: Run tests/check and commit**

```bash
cargo test -p reforge-hid
cargo check --workspace
git add crates/reforge-hid crates/reforge-protocol
git commit -m "feat(hid): add audio control adapters"
```

### Task 5: Receiver management and safe pairing actions

**Files:**
- Create: `crates/reforge-hid/src/adapters/receiver.rs`
- Modify: `crates/reforge-hid/src/adapters/mod.rs`
- Modify: `crates/reforge-hid/src/lib.rs`
- Test: `crates/reforge-hid/tests/receiver_controls.rs`

**Interfaces:**
- Consumes: physical-device dedup/routing from v0.3.1.
- Produces: receiver status controls and only positively identified safe pair/unpair actions.

- [ ] **Step 1: Write failing receiver-family tests**

Cover: unknown receiver exposes status but no pairing action; known receiver family exposes slot status; mirror-child route remains internal; distinct same-model children remain distinct; unpair rejects invalid slot.

- [ ] **Step 2: Run tests and confirm RED**

```bash
cargo test -p reforge-hid --test receiver_controls
```

- [ ] **Step 3: Implement receiver adapter with allowlisted protocol families**

Pair/unpair actions are advertised only when VID/PID/protocol feature evidence identifies a supported receiver family and the required operation is implemented. Actions are never profile-eligible.

- [ ] **Step 4: Keep physical-card dedup invariant**

Update fingerprint tests so mutable receiver status does not create duplicates, while route identity remains available to the writer.

- [ ] **Step 5: Run tests/check and commit**

```bash
cargo test -p reforge-hid
cargo check --workspace
git add crates/reforge-hid
git commit -m "feat(hid): add safe receiver controls"
```

### Task 6: Complete provider/router/profile integration and CLI

**Files:**
- Modify: `crates/reforge-daemon/src/providers.rs`
- Modify: `crates/reforge-daemon/src/main.rs`
- Modify: `crates/reforge-cli/src/main.rs`
- Test: `crates/reforge-daemon/src/providers.rs` unit tests
- Test: `crates/reforge-core/tests/rpc_roundtrip.rs`

**Interfaces:**
- Consumes: all advertised `DeviceControl`s and `reforge_hid::set_control`.
- Produces: provider-neutral get/set routing, typed V4L2/PipeWire validation, complete typed profile snapshots/application, CLI get/set/list commands.

- [ ] **Step 1: Write failing provider/router tests**

Add V4L2 fixtures for range/bool/menu/intmenu/button/read-only/disabled controls; PipeWire controls for volume/mute; route tests showing HID++ dispatch is selected for `ProviderKind::Hidpp`; profile filtering excludes actions/status and keeps writable eligible values.

- [ ] **Step 2: Run targeted tests and confirm RED**

```bash
cargo test -p reforge-daemon
cargo test -p reforge-core --tests
```

- [ ] **Step 3: Migrate V4L2/PipeWire controls to typed values/metadata**

Parse helper output into `ControlValue`; set `group=Camera` for V4L2 and `group=Audio` for PipeWire. Disabled/read-only V4L2 controls become non-writable telemetry/status; buttons remain actions and non-profile-eligible.

- [ ] **Step 4: Implement daemon get/set router**

For `SetControl`, locate the exact advertised control by ID, reject non-writable controls, validate type/min/max/step/choice, then dispatch by provider. For HID++ call `reforge_hid::set_control(device, control, &value)`. Add `GetControl` that refreshes where provider supports reads and otherwise returns canonical cached value.

- [ ] **Step 5: Expand profile snapshot/application**

Persist all controls with `profile_eligible && writable`. Apply in deterministic group order: prerequisites/profiles, performance/assignments, lighting, audio/camera/other. Report the exact failed control and stop only dependencies that require that prerequisite.

- [ ] **Step 6: Update CLI**

Add:

```text
reforge-logitechctl controls <device>
reforge-logitechctl control get <device> <control-id>
reforge-logitechctl control set <device> <control-id> <typed-value>
```

Keep existing DPI/lighting aliases.

- [ ] **Step 7: Run daemon/CLI/workspace tests and commit**

```bash
cargo test -p reforge-daemon
cargo test -p reforge-cli
cargo check --workspace
git add crates/reforge-daemon crates/reforge-cli crates/reforge-core
git commit -m "feat(daemon): route all typed device controls"
```

### Task 7: Capability-driven device pages and structured control widgets

**Files:**
- Modify: `crates/reforge-gui/src/app.rs`
- Modify: `crates/reforge-gui/src/keyboard.rs`
- Test: add pure GUI grouping tests in `crates/reforge-gui/src/app.rs`

**Interfaces:**
- Consumes: `ControlGroup`, `ControlKind`, `ControlValue`, existing lighting/profile state.
- Produces: device pages Overview/Assignments/Performance/Lighting/Audio/Camera/Power/Profiles/Receiver plus generic diagnostic controls.

- [ ] **Step 1: Write failing pure grouping/widget-selection tests**

Test that controls map into the correct visible section, unsupported sections are absent, `Vector` chooses structured editor metadata, and `Status` is non-interactive.

- [ ] **Step 2: Run GUI tests and confirm RED**

```bash
cargo test -p reforge-logitech
```

- [ ] **Step 3: Add capability-driven section navigation**

Build the sidebar section list from actual controls plus existing `dpi`/`lighting`. Keep system theme as default. A device with no `Camera` controls must not show Camera; same for Audio/Receiver/etc.

- [ ] **Step 4: Render typed controls**

`Bool` → checkbox/switch, `Int` → slider/spin with step, `Choice` → combo, `Vector` → labeled band/vector editor, `Action` → button with confirmation for destructive receiver actions, `Status` → read-only row.

- [ ] **Step 5: Keep dedicated Lighting Studio**

Do not regress per-key G515 editor or host effects. HID++ lighting controls may also appear in diagnostics, but Lighting remains the primary experience.

- [ ] **Step 6: Run GUI/workspace tests and commit**

```bash
cargo test -p reforge-logitech
cargo check --workspace
git add crates/reforge-gui
git commit -m "feat(gui): add capability-driven device pages"
```

### Task 8: Regression suite, release build gate, and Arch v0.4 package

**Files:**
- Modify: `crates/reforge-hid/tests/helpers.rs`
- Add/modify: HID regression tests under `crates/reforge-hid/tests/`
- Modify: `packaging/arch/PKGBUILD`
- Modify: `scripts/install-arch.sh`
- Modify: `README.md`
- Modify: `RELEASE_NOTES.md`
- Modify: `INSTALL-UPGRADE.txt`

**Interfaces:**
- Consumes: all v0.4 implementation.
- Produces: release-source archive and installer that cannot replace a working installation after a failed test/build.

- [ ] **Step 1: Add release regressions**

Tests must cover:

```text
G515 direct + six mirrored slots -> 1 physical device
G515 reports 95 per-key zones and brightness metadata
same-model distinct paired children -> 2 devices
legacy v0.3.1 profile -> v0.4 profile store
per-key failed sub-packet -> no FrameEnd
V4L2 read-only/disabled -> not writable
unknown receiver -> no pair/unpair action
```

- [ ] **Step 2: Run the full test suite**

```bash
cargo test --workspace
```

Expected: all tests PASS, zero failures.

- [ ] **Step 3: Run the release build**

```bash
cargo build --release --workspace
```

Expected: exit 0. Warnings may be cleaned if they indicate obsolete dead code; no compiler errors permitted.

- [ ] **Step 4: Bump package/version metadata to 0.4.0**

Update workspace/package release metadata and Arch `pkgver`. Preserve binary/service names.

- [ ] **Step 5: Verify installer ordering**

`install-arch.sh`/`PKGBUILD` must execute tests and release build before package installation. A failure exits before `pacman -U`/installation.

- [ ] **Step 6: Static package verification**

```bash
bash -n scripts/*.sh
python - <<'PY'
import tomllib, pathlib
for p in pathlib.Path('.').rglob('Cargo.toml'):
    tomllib.loads(p.read_text())
print('Cargo manifests parse')
PY
git diff --check
```

- [ ] **Step 7: Commit release source**

```bash
git add .
git commit -m "release: prepare ReForge Logitech v0.4.0"
```

- [ ] **Step 8: Create source ZIP and checksums**

Create `ReForge-Logitech-Linux-v0.4.0-source.zip` from the clean release tree, excluding `.git`, build artifacts, and prior archives. Generate SHA-256 alongside it and verify the archive can be listed/extracted.
