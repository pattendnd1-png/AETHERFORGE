# ReForge Logitech Linux v0.6.0 — Logitech Services Design

## Goal

Turn the v0.5 reliability/control center into a service-backed Logitech control application that maintains real persistent device sessions, integrates with the supported Linux firmware service, reacts to hardware events, and exposes service state clearly in GUI/CLI.

## Non-goals

- No undocumented G HUB account or profile-cloud authentication.
- No firmware flashing by ReForge itself.
- No arbitrary/raw HID writer exposed to users.
- No replacement of Linux kernel input drivers.
- No removal of v0.5 HID++/V4L2/PipeWire control providers.

## Authoritative integration paths

1. **Local Logitech device control** uses Logitech HID++ on hidraw. HID++ 2.0 is self-describing and has per-device feature tables; ReForge already probes and controls these features.
2. **Firmware** uses the system fwupd service and LVFS metadata. ReForge acts as a client and does not download/flash arbitrary firmware itself.
3. **Camera controls** remain V4L2/UVC.
4. **Audio controls** remain PipeWire, supplemented by verified HID++ headset features.
5. **Hotplug** uses Linux udev where available with a periodic discovery fallback.

## Architecture

### 1. Persistent HID++ session service

Create `reforge-hid::SessionManager` owning one `HidDevice` per online HID++ physical device key. Each session stores:

- stable ReForge device key
- hidraw path and HID++ device index
- cached feature table
- last successful transaction timestamp
- reconnect/error counters
- connection/session state

All HID++ reads/writes for daemon-owned devices route through this manager. A per-session mutex serializes request/reply traffic because HID++ requests and asynchronous notifications share the same hidraw stream.

Existing free functions remain as compatibility wrappers where practical, but daemon paths use session APIs.

### 2. HID++ notifications

Each live session has a notification reader that can distinguish request replies from unsolicited HID++ notifications. Notifications are converted to typed `DeviceServiceEvent` values for known events (connection/battery/profile/mute where identifiable) and `UnknownHidppNotification` metadata otherwise. Unknown packets are never executed as commands.

The session service updates telemetry and triggers a targeted device refresh when a notification indicates capabilities or connectivity changed.

### 3. Service registry

Add a daemon `ServiceRegistry` independent from the device registry. Service types:

- `Hidpp`
- `Udev`
- `Fwupd`
- `V4l2`
- `PipeWire`

Each publishes `ServiceStatus` with state (`Ready`, `Degraded`, `Unavailable`, `Error`), optional version/backend information, last success timestamp, and a sanitized message.

RPC exposes list/reconnect operations. Reconnect is service-specific and never bypasses capability validation.

### 4. udev hotplug service

Use `udevadm monitor --udev --subsystem-match=hidraw --subsystem-match=usb` as a zero-new-Rust-dependency event source in v0.6. The daemon parses only add/remove/change actions and schedules debounced discovery. If `udevadm` is absent or the monitor exits, mark the service degraded and keep the existing 2-second fallback discovery loop.

The Arch package already depends on systemd/udev through the target OS; no embedded privileged daemon is added.

### 5. fwupd/LVFS firmware service

Use the supported system `fwupdmgr --json` client surface in v0.6 rather than reimplementing D-Bus variant parsing without a D-Bus crate. The service:

- probes fwupd availability/version
- refreshes metadata only on explicit user request
- enumerates fwupd devices as JSON
- matches Logitech devices by VID/PID/GUID/name/serial-safe identity where available
- lists update/release metadata
- invokes trusted fwupd update flows, never direct flashing

Firmware writes require explicit user action. ReForge never uses force/downgrade flags by default.

`fwupd` is an optional runtime integration; absence does not break ordinary HID++ control.

### 6. Receiver service state

Receiver/device relationships are represented explicitly in runtime status. ReForge keeps a physical child device as one UI device and records receiver path/index internally. Pair/unpair is exposed only when a documented/safely implemented protocol operation exists; v0.6 does not invent a pairing write.

### 7. RPC contract

Add requests:

- `ListServices`
- `ReconnectService { service }`
- `ListFirmwareDevices`
- `RefreshFirmwareMetadata`
- `GetFirmwareReleases { device_id }`
- `InstallFirmwareUpdate { device_id }`
- `GetSessionStatus { key }`

Add responses for service list, firmware devices/releases, and session status.

Firmware installation responses represent acceptance/result from fwupd and any reboot/replug requirement reported by fwupd. No raw firmware bytes cross ReForge RPC.

### 8. GUI

Add two pages:

**Services**
- status cards for HID++, udev, fwupd, V4L2, PipeWire
- backend/version/status message
- reconnect/rescan action where applicable
- per-device HID++ session status and receiver routing

**Firmware**
- matched Logitech/fwupd devices
- current firmware version
- update availability
- trusted/update metadata supplied by fwupd
- explicit Refresh Metadata and Install Update actions
- reboot/replug requirements

Offline or unavailable integrations are shown, not hidden.

### 9. CLI

Add:

- `services`
- `services reconnect <service>`
- `session <device>`
- `firmware devices`
- `firmware refresh`
- `firmware releases <fwupd-device-id>`
- `firmware update <fwupd-device-id>`

### 10. Packaging and permissions

- Add `fwupd` as an optional/recommended Arch dependency, not a hard dependency for HID++ use.
- Keep `v4l-utils` and PipeWire integration behavior.
- Replace permissive Logitech hidraw rules with `TAG+="uaccess"` and a controlled group fallback; do not install mode `0666`.
- User daemon remains unprivileged. Firmware privilege mediation is delegated to fwupd/polkit.

## Compatibility

- v0.5 profile store and preferences remain readable.
- Existing RPC variants remain unchanged.
- Existing G515 single-device deduplication stays enforced.
- Existing HID++ controls remain capability-gated.
- If persistent session setup fails for a device, the service marks the session degraded and discovery continues; it does not silently send unverified writes.

## Safety

- No private Logitech credentials.
- No arbitrary USB/HID write UI.
- No ReForge-managed firmware payload download or flashing.
- Firmware operations use the system fwupd trust/policy path.
- HID++ requests remain typed and feature-index validated.
- Session reader never interprets unsolicited packets as write requests.

## Testing

Add tests for:

- session state transitions and reconnect counters
- reply-vs-notification classification
- service status serialization
- fwupd JSON parsing and Logitech matching using fixtures
- udev event parsing/debounce
- new RPC round trips
- CLI parser coverage
- v0.5 profile/preference compatibility
- receiver mirror dedup regression for G515
- package udev rule does not grant world read/write

The Arch package must run `cargo test --workspace` before `cargo build --release --workspace` and abort before installation on either failure.
