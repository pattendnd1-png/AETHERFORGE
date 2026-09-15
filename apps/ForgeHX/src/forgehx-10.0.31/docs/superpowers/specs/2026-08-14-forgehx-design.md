# ForgeHX v0.1 Design

## Goal

ForgeHX is a native Rust control suite for HyperX peripherals on Arch Linux and Arch-derived distributions. It is not a Wine wrapper around HyperX NGENUITY. The first release establishes a safe, installable hardware-control platform with device discovery, profiles, GUI/CLI control, PipeWire integration, and an extensible protocol layer for adding verified HyperX devices.

## Scope

### In scope for v0.1

- Native Rust workspace with daemon, GUI, CLI, core, and device crates.
- HyperX USB/HID discovery by vendor/product identifiers.
- Read-only Device Doctor for unknown devices.
- Safe capability model that prevents unsupported writes by default.
- Persistent per-device profiles.
- RGB/effect abstraction with implementations enabled only for verified devices.
- Mouse DPI-stage and key-binding abstractions with implementations enabled only for verified devices.
- Keyboard macro/key-binding profile model.
- PipeWire-backed audio device discovery and user-space volume/mute controls for HyperX audio devices when exposed by PipeWire.
- Headset/microphone EQ profile model, with hardware or PipeWire implementation selected per device capability.
- User daemon with hotplug monitoring.
- Native egui/eframe GUI.
- CLI and diagnostic output suitable for adding new device definitions.
- udev rule installation.
- systemd user service.
- desktop launcher.
- Arch PKGBUILD and local packaging scripts.
- Source release archive.

### Explicitly out of scope for v0.1

- Firmware flashing or firmware update writes.
- Sending guessed or reverse-engineered write packets to unknown devices.
- Claiming support for every HyperX device before its protocol has been verified.
- Cloud accounts, telemetry, or vendor services.
- Windows NGENUITY emulation.

## Architecture

ForgeHX is divided into small crates with one purpose each.

### `forgehx-core`

Owns stable application concepts independent of Linux device I/O:

- `DeviceId`
- device metadata and capability flags
- RGB/effect types
- DPI-stage configuration
- macro/key binding types
- audio/EQ profile types
- profile serialization and validation
- command/reply protocol shared by daemon clients

This crate must not perform direct HID, USB, PipeWire, filesystem, or GUI operations.

### `forgehx-device`

Owns peripheral discovery and device protocol implementations:

- HID enumeration through `hidapi`
- HyperX vendor/product registry
- interface/report metadata collection
- read-only Device Doctor
- protocol trait for device-specific behavior
- capability negotiation
- safe write gate

Unknown devices are exposed as discovered devices with diagnostic capabilities only. Device writes require an explicit protocol implementation whose product identifiers and supported operations are registered in source.

### `forgehx-audio`

Owns Linux audio integration:

- discover HyperX PipeWire nodes
- volume/mute operations
- optional software EQ integration where practical
- mapping between USB/HID devices and PipeWire-visible audio nodes

Failure to find PipeWire audio endpoints must not prevent non-audio device control.

### `forgehx-daemon`

Runs as a systemd user service and owns exclusive state-changing hardware access:

- hotplug detection
- active-device registry
- profile application
- persistence
- IPC server
- request validation
- logging and recoverable error state

Clients never directly write to HID devices while the daemon is active.

### `forgehx-cli`

Provides terminal workflows:

- `forgehx list`
- `forgehx doctor [device]`
- `forgehx profile list|get|apply|save`
- `forgehx rgb ...`
- `forgehx dpi ...`
- `forgehx audio ...`
- `forgehx daemon status`

Unsupported operations return a clear capability error instead of attempting a write.

### `forgehx-gui`

Uses `eframe`/`egui` and communicates with the daemon through the same IPC model as the CLI.

Primary layout:

1. Device rail: connected HyperX devices and capability badges.
2. Main editor: tabs shown only when supported (`Lighting`, `Keys`, `DPI`, `Audio`, `Microphone`, `Profiles`).
3. Status/footer: daemon state, active profile, write protection status, and diagnostics shortcut.

Unknown devices open directly into Device Doctor rather than showing nonfunctional controls.

## IPC and Data Flow

The daemon exposes a local Unix domain socket under the user's runtime directory. Messages are length-delimited JSON for v0.1 to keep diagnostics and versioning easy to inspect.

Flow:

1. daemon discovers a physical device;
2. device crate resolves a registered protocol or diagnostic-only fallback;
3. daemon publishes device + capability metadata;
4. GUI/CLI requests an operation;
5. daemon validates requested capability and profile data;
6. protocol implementation generates the exact verified device command;
7. daemon performs I/O and returns structured success/error data;
8. profile changes are persisted only after validation.

IPC messages include a protocol version to permit future migration without silently misinterpreting commands.

## Profiles

Profiles are stored beneath the user's XDG config directory, e.g. `~/.config/forgehx/profiles/`.

Each profile contains:

- schema version
- optional matching constraints (device model/serial)
- lighting settings
- DPI settings
- binding/macro settings
- audio/microphone settings

Profiles ignore unsupported sections when applied to a device, but the daemon reports which sections were skipped.

## Safe Hardware Policy

Safety is a core v0.1 requirement.

- Enumeration and descriptor inspection are permitted for all devices.
- Read operations are permitted only when defined as safe by the protocol implementation or standard HID behavior.
- Vendor-specific write operations are disabled for unregistered/unknown product protocols.
- Firmware/bootloader commands are excluded from the v0.1 API.
- Every protocol implementation declares its product identifiers and supported operations explicitly.
- A failed hardware operation leaves the daemon alive and returns the device to an error/reconnect state rather than retrying writes indefinitely.

## Device Protocol Interface

Each verified device family implements a common trait conceptually containing:

- metadata matching
- capabilities
- optional state readback
- set lighting
- set DPI
- set bindings/macros
- device-specific audio/microphone controls

Methods not supported by the hardware return `Unsupported`, not a no-op success.

## Device Doctor

Device Doctor is the expansion mechanism for hardware not yet implemented. It records information necessary to build a protocol implementation without changing device state:

- USB vendor/product ID
- manufacturer/product/serial strings when available
- HID interface/path identifiers
- usage page/usage where exposed
- report descriptor/report sizes where obtainable safely
- kernel driver binding information when practical
- PipeWire node matches
- ForgeHX protocol match result

The CLI supports machine-readable JSON diagnostic output in addition to human-readable output.

## Packaging

The repository includes:

- `PKGBUILD`
- `forgehx.install` only if post-install messaging is genuinely required
- `packaging/udev/70-forgehx.rules`
- `packaging/systemd/forgehx-daemon.service`
- `packaging/desktop/io.forgehx.ForgeHX.desktop`
- `scripts/build-release.sh`
- `scripts/build-arch-package.sh`
- `scripts/install-local.sh`
- `scripts/uninstall-local.sh`

Installed executables:

- `/usr/bin/forgehx`
- `/usr/bin/forgehx-gui`
- `/usr/bin/forgehx-daemon`

The user service is enabled/started by an explicit installer action rather than forcing a system-wide daemon.

## Error Handling

- Use structured Rust errors with user-readable context.
- A single failed device must not crash the daemon.
- HID permission errors must identify udev-rule remediation.
- Daemon connection failures in GUI/CLI must distinguish service-not-running from protocol-version mismatch.
- Profile parse/validation failures must preserve the previous valid profile.
- Unsupported features must be represented as capability errors, never as misleading successes.

## Logging

The daemon logs concise state transitions through stderr/journald when run by systemd. Debug logging may expose HID metadata but must not dump arbitrary write payloads unless explicitly enabled for protocol development.

## Testing

### Unit tests

- profile serialization/version validation
- capability checks
- protocol registry matching
- IPC encode/decode
- unsupported-operation behavior
- state transitions after device errors

### Integration tests

Use fake protocol/device backends so tests run without HyperX hardware:

- hotplug add/remove
- apply profile to supported device
- reject write to unknown device
- daemon survives failed device operation
- CLI request/response over Unix socket

### Packaging checks

- `cargo test --workspace`
- release workspace build
- shell syntax checks on helper scripts
- `PKGBUILD` metadata inspection where Arch tooling is available
- packaged service/desktop/udev paths verified against install layout

## Acceptance Criteria for v0.1

v0.1 is complete when:

1. The Rust workspace builds in release mode on Linux.
2. Unit/integration tests pass without physical hardware.
3. `forgehx list` enumerates available HID devices and identifies HyperX devices.
4. `forgehx doctor` produces human-readable and JSON diagnostics without vendor-specific writes.
5. Unknown devices cannot receive state-changing commands.
6. The daemon starts as a user service and survives device-level failures.
7. GUI launches and displays daemon/device/profile state.
8. Profiles can be created, saved, loaded, and applied through the shared capability model.
9. PipeWire-backed volume/mute controls operate when matching HyperX audio nodes are present, without blocking the rest of the application when absent.
10. Arch packaging files and local installer scripts are included.
11. A source release archive can be produced from the repository.
12. Firmware flashing is not exposed.

## Future Device Expansion

After v0.1, exact models are added one family at a time from Device Doctor output and verified captures/documentation. A model is promoted from diagnostic-only to writable support only when command semantics are known and regression tests cover the implemented operations.
