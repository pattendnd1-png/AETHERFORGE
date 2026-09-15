# ReForge Logitech Linux v0.5.0 — Reliability & Daily-Use Design

## Goal

Turn the v0.4.0 full-device-control build into a dependable always-on control center without replacing its provider architecture. v0.5.0 must preserve HID++/V4L2/PipeWire control behavior while adding stable device identity, automatic hot-plug/reconnect, live telemetry, reconnect restoration, complete profile lifecycle management, persistent GUI preferences, and a redacted diagnostic bundle.

## Baseline

The implementation starts from ReForge Logitech Linux v0.4.0. The existing architecture remains authoritative:

- `reforge-hid` owns Logitech HID++ discovery and typed HID++ writes.
- `reforge-daemon::providers` merges HID++, generic HID, USB/sysfs, V4L2, and PipeWire devices and routes provider controls.
- `reforge-core` defines `DeviceSummary`, `DeviceControl`, typed `ControlValue`, lighting/profile models, RPC, paths, and the synchronous Unix-socket client.
- `reforge-daemon` owns cached devices, lighting state, host-driven lighting effects, app/game auto-profile activation, and request routing.
- `reforge-gui` renders capability-driven controls and currently selects devices/profiles by vector index.

v0.5.0 is an additive reliability release, not a provider rewrite.

## Non-negotiable compatibility requirements

1. Existing v0.4.0 `profiles.json` must load without manual migration.
2. Existing typed `SetControl`, lighting, DPI, V4L2, PipeWire, and HID++ write paths remain capability-gated and range-validated.
3. The G515 LS TKL must continue to appear as one physical keyboard, not receiver-slot mirrors, and retain per-key RGB support.
4. A reconnect may restore only controls still advertised as writable/profile-eligible by the newly discovered device.
5. Failed test/build/install gates must not replace a working installed version.
6. No arbitrary raw USB/HID write surface is added.
7. ReForge remains Linux-first and user-session scoped; the daemon continues to use its Unix socket under the runtime directory.

## Architecture

### 1. Stable device registry

Replace the daemon's current `HashMap<String, DeviceSummary>` scan cache with a registry that tracks lifecycle state per stable device key.

Add shared core types:

```rust
pub enum DevicePresence {
    Online,
    Offline,
    Reconnecting,
}

pub struct DeviceRuntimeInfo {
    pub summary: DeviceSummary,
    pub presence: DevicePresence,
    pub first_seen_unix_ms: u64,
    pub last_seen_unix_ms: u64,
    pub reconnect_count: u32,
}
```

`DeviceSummary.key` remains the stable public identity used by RPC. Discovery may update transport/provider endpoints attached to that key, but reconnects must not generate a new GUI identity when the physical device identity is unchanged.

The registry owns an `offline_since` timestamp internally. Missing devices are marked `Offline`; they are not immediately removed. Offline entries age out only after a bounded retention period (default 10 minutes) unless they are referenced by a saved profile or current GUI preference, in which case they may remain visible for the daemon session.

### 2. Discovery lifecycle

Add a daemon discovery loop separate from the existing effect and auto-profile loops.

Behavior:

- Perform an initial full scan at daemon startup.
- Run a low-frequency fallback full scan every 2 seconds.
- Diff discovered stable keys against the registry.
- New key: insert as `Online`.
- Existing online key: refresh summary/providers/controls and update `last_seen`.
- Previously offline key: set `Reconnecting`, refresh capabilities/endpoints, run safe restore, then set `Online` on success. If restore partially fails, the device still becomes `Online` and the error is recorded.
- Missing key: mark `Offline` and stop host-driven effects that require an open device endpoint, but retain desired lighting/profile state for future restore.

The design intentionally uses a polling fallback instead of adding a mandatory udev subscription dependency in v0.5.0. A future release may add event-driven udev/DBus hints, but correctness must not depend on them.

### 3. Desired-state restoration

The daemon already stores `lighting_states`; extend runtime state with a per-device desired control map and last applied profile name.

```rust
pub struct DesiredDeviceState {
    pub dpi: Option<u16>,
    pub lighting: Option<LightingState>,
    pub controls: BTreeMap<String, ControlValue>,
    pub profile_name: Option<String>,
}
```

Every successful UI/RPC write updates desired state after the hardware write succeeds. Applying a profile updates desired state only for settings successfully written.

Reconnect restore order:

1. Refresh capabilities/endpoints.
2. Restore profile/onboard-mode controls first.
3. Restore performance/assignments.
4. Restore lighting.
5. Restore audio/camera controls.
6. Restore power controls last.
7. Restart host-driven lighting effect only if its capability requirements are still present.

Each restoration write must be validated against the newly scanned `DeviceSummary`. Missing/changed controls are skipped and logged, never synthesized.

### 4. Telemetry path

Add a lightweight telemetry RPC that does not trigger a full discovery scan.

Core model:

```rust
pub struct DeviceTelemetry {
    pub key: String,
    pub presence: DevicePresence,
    pub battery_percent: Option<u8>,
    pub charging: Option<bool>,
    pub active_dpi: Option<u16>,
    pub provider_health: BTreeMap<ProviderKind, bool>,
    pub updated_unix_ms: u64,
}
```

Add RPC requests/data:

```rust
RpcRequest::ListDeviceRuntime
RpcRequest::GetDeviceTelemetry { key: String }
RpcData::DeviceRuntime(Vec<DeviceRuntimeInfo>)
RpcData::Telemetry(DeviceTelemetry)
```

Telemetry reads reuse cached capability/endpoints and provider-specific lightweight reads. If a read fails because hardware vanished, mark the device offline and return the latest known telemetry with `presence=Offline` rather than forcing a complete rescan from the GUI.

GUI polling cadence: once per second while the window is active. Full device list refresh is no longer driven by user interaction alone; the GUI periodically consumes `ListDeviceRuntime` and preserves selection by key.

### 5. Profile lifecycle management

Keep `Profile` compatible. Extend `ProfileStore` with explicit name-based operations:

```rust
pub fn get(&self, name: &str) -> Option<&Profile>;
pub fn rename(&mut self, old: &str, new: &str) -> Result<(), String>;
pub fn remove(&mut self, name: &str) -> Result<Profile, String>;
pub fn clone_as(&mut self, source: &str, new: &str) -> Result<Profile, String>;
pub fn import_json(&mut self, json: &str, replace: bool) -> Result<Vec<String>, String>;
pub fn export_json(&self, names: &[String]) -> Result<String, String>;
```

Rules:

- Profile names are trimmed and non-empty.
- Rename/clone/import reject duplicate names unless `replace=true` for import.
- Save remains atomic through a temporary file and rename.
- Imported profiles deserialize through the same backwards-compatible `Profile` model and are normalized before persistence.

Add RPC operations for rename, delete, clone, import, and export. Export returns JSON text through RPC; the GUI is responsible for saving it to a user-selected/default config path available to the app. Import reads a path in the GUI process and sends validated JSON text to the daemon; the daemon never accepts an arbitrary filesystem path from RPC.

### 6. Persistent GUI preferences

Add `UiPreferences` in `reforge-core` and persist to `~/.config/reforge-logitech/ui.json` through `paths::ui_preferences_file()`.

Persist:

- selected stable `device_key`,
- selected profile name,
- page,
- theme (`system`, `light`, `dark`),
- live-preview enabled state,
- last window size if available from eframe persistence hooks.

Preferences are best-effort. Corrupt `ui.json` falls back to defaults and is surfaced in Diagnostics; it must not prevent the app from starting.

The GUI replaces `selected_device: Option<usize>` with `selected_device_key: Option<String>` and `selected_profile: Option<usize>` with `selected_profile_name: Option<String>`.

### 7. Diagnostics and recent errors

Add a bounded in-memory daemon event/error ring buffer (default 200 entries).

```rust
pub enum DiagnosticLevel { Info, Warning, Error }

pub struct DiagnosticEvent {
    pub unix_ms: u64,
    pub level: DiagnosticLevel,
    pub component: String,
    pub message: String,
    pub device_key: Option<String>,
}
```

Record discovery transitions, reconnect attempts/results, profile auto-switches, provider failures, rejected restores, and request errors. Do not record arbitrary payload bytes or raw HID reports.

Add:

```rust
RpcRequest::GetDiagnostics
RpcRequest::ClearDiagnostics
RpcData::Diagnostics(DiagnosticSnapshot)
```

`DiagnosticSnapshot` includes daemon/package version, runtime socket path category (not full home path), integration availability, registry/device summaries, feature IDs, provider/control inventory, recent events, and profile-store version/count.

### 8. Diagnostic bundle export and redaction

The GUI's Diagnostics page can export a JSON bundle. Default export is redacted:

- replace serial/unit IDs with deterministic per-bundle hashes,
- replace home-directory paths with `$HOME`,
- omit raw process lists, profile application rules, and raw HID reports,
- keep product IDs, feature IDs, provider kinds, control IDs, error text, and package versions.

An explicit `Include device identifiers` checkbox may include serial/unit IDs, but raw HID payloads remain excluded in all modes.

Bundle generation should live in `reforge-core::diagnostics` as a pure transformation so it is unit-testable without hardware.

### 9. GUI structure

Add a `Diagnostics` navigation page. Existing pages remain.

Dashboard device cards show:

- online/offline/reconnecting badge,
- battery/charging if known,
- transport/providers,
- current profile if known,
- last-seen/reconnect count for offline/recovered hardware.

Offline devices remain selectable. Device Controls and Lighting pages disable hardware writes while offline but continue to display cached capabilities/settings and explain that changes require the device to reconnect. No write is queued merely because the user changed a widget while offline.

Profiles page adds Rename, Clone, Delete, Import, Export, and Apply. Destructive delete requires an in-app confirmation state before RPC.

Settings adds persistent theme/live-preview controls plus buttons for `Rescan devices` and `Restart daemon`. `Rescan devices` maps to an RPC that triggers immediate discovery. `Restart daemon` invokes `systemctl --user restart reforge-logitechd.service` from the GUI only after an explicit click; the GUI then retries connection on subsequent refresh cycles.

### 10. RPC additions

Add the following requests while preserving all v0.4 variants:

```rust
RescanDevices,
ListDeviceRuntime,
GetDeviceTelemetry { key: String },
RenameProfile { old_name: String, new_name: String },
DeleteProfile { name: String },
CloneProfile { source_name: String, new_name: String },
ImportProfiles { json: String, replace: bool },
ExportProfiles { names: Vec<String> },
GetDiagnostics,
ClearDiagnostics,
```

Add data variants:

```rust
DeviceRuntime(Vec<DeviceRuntimeInfo>),
Telemetry(DeviceTelemetry),
ExportedProfiles { json: String },
Diagnostics(DiagnosticSnapshot),
```

All new variants require JSON round-trip tests.

## Error handling

- Discovery failures do not crash the daemon; they generate diagnostic events and preserve the last registry state.
- A single provider failure cannot mark unrelated providers on the same device unhealthy.
- Reconnect restoration is best-effort per control. One rejected control does not abort the remaining restore sequence.
- Profile mutations return descriptive errors and persist atomically only after validation succeeds.
- GUI RPC failures update a visible status area without dropping the current device/profile selection.
- Corrupt UI preferences fall back to defaults and can be overwritten by the next successful preference save.

## Testing strategy

### Core unit tests

- `DevicePresence`, `DeviceRuntimeInfo`, telemetry, diagnostics, and new RPC variants round-trip through JSON.
- v0.4 profile JSON continues to deserialize.
- profile rename/delete/clone/import/export behavior, duplicate handling, and atomic normalization.
- UI preference defaulting and round-trip.
- diagnostic redaction removes serials/home paths/application rules while retaining useful hardware metadata.

### Daemon tests

Extract registry diff/transition logic into pure functions and test:

- new device -> Online,
- disappeared device -> Offline without deletion,
- same stable key returns -> Reconnecting -> Online,
- capability-changed reconnect skips unsupported desired controls,
- G515 mirrored receiver-slot scan still yields one stable registry entry,
- telemetry failure marks only the affected device/provider,
- restore ordering is deterministic.

### GUI tests

Where practical, extract selection reconciliation and preference conversion into pure functions:

- selection survives device list reorder,
- selection remains on an offline device,
- missing preferred device falls back predictably,
- profile selection survives rename/clone/delete operations,
- offline state disables writes.

### Build/install gate

The Arch package must run, in order:

```bash
cargo test --workspace
cargo build --release --workspace
```

Only after both commands succeed may package installation replace the current installed binaries.

## Release success criteria

v0.5.0 is ready to package when all of the following are true:

1. A device can be unplugged/replugged without manual Refresh and returns under the same stable GUI identity.
2. The selected device does not change merely because enumeration order changes.
3. Battery/connection telemetry updates without a full feature rescan.
4. A reconnect restores only still-supported desired controls and restarts eligible host lighting effects.
5. Profile create/rename/clone/delete/import/export/apply all work with v0.4 profile compatibility.
6. Theme, selected device/profile, page, and live-preview preference survive app restart.
7. Diagnostics show lifecycle/errors and can export a redacted bundle.
8. Existing HID++ DPI/lighting/assignments/audio, V4L2 camera, PipeWire audio, auto-profile, and G515 single-device behavior remain intact.
9. Failed Rust test/build gates leave the previously installed release untouched.
