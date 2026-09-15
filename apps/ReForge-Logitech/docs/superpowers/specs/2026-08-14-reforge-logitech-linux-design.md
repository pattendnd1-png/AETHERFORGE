# ReForge Logitech Linux v0.1 Design

## Goal
Build an unofficial, Linux-native Logitech-compatible device control stack in Rust, focused on safe HID++ configuration without replacing Linux's stable input drivers.

## Scope
v0.1 provides Logitech USB/Bluetooth HID discovery, HID++ 2.x-compatible feature probing, adjustable DPI discovery/read/write for devices exposing feature `0x2201`, profile persistence, a per-user daemon, CLI, egui GUI, udev permissions, systemd user integration, and Arch Linux packaging.

v0.1 intentionally does not flash firmware, inject arbitrary HID reports, claim support for every Logitech model, or expose controls that have not been implemented and verified. RGB, per-key lighting, button remapping, report rate, headset EQ, sidetone, and onboard profiles are represented as discoverable feature IDs and extension points for later releases.

## Architecture
- `reforge-core`: serializable data model, profile model, RPC protocol, XDG paths.
- `reforge-protocol`: HID++ short/long report encoding and parsing, feature constants, feature-set parsing, adjustable-DPI parsing and validation. Pure logic and hardware-independent tests.
- `reforge-hid`: Logitech HID enumeration via the Rust `hidapi` crate using Linux native hidraw access, safe request/response matching, feature probing, DPI read/write.
- `reforge-daemon`: per-user Unix-socket service at `$XDG_RUNTIME_DIR/reforge-logitech/reforge.sock`; rescans on requests so stale USB handles are not retained.
- `reforge-cli`: device inspection, DPI get/set, profile creation/application, daemon health.
- `reforge-gui`: native egui/eframe UI that talks to the daemon and exposes only verified controls.

Linux continues to own normal keyboard/mouse/audio input through `usbhid`, `hid-logitech-dj`, and `hid-logitech-hidpp`. ReForge is a userspace configuration driver using `hidraw`.

## HID++ Safety Model
- Only Logitech vendor ID `0x046d` is enumerated.
- Read-only root feature calls are used to identify HID++ feature indices.
- DPI writes are enabled only after feature `0x2201` is confirmed and the device reports a valid DPI range.
- Requested DPI must be within the reported range and aligned to the reported step.
- A DPI write is followed by a read-back; mismatch is reported as an error.
- Firmware/DFU feature IDs are never invoked by v0.1.
- No arbitrary raw-report API is exposed through RPC or CLI.

## HID++ Framing
Short report: 7 bytes, report ID `0x10`.
Long report: 20 bytes, report ID `0x11`.
Byte 1 is device index, byte 2 feature index, byte 3 contains function nibble in the high four bits and software ID in the low four bits. ReForge uses software ID `0x8`.

Root function 0 resolves a 16-bit feature ID to an index. Feature Set `0x0001` is enumerated when available. Adjustable DPI `0x2201` uses functions 0/1/2 for sensor count/range/current state and function 3 for setting current DPI.

## Profiles
Profiles are JSON under `$XDG_CONFIG_HOME/reforge-logitech/profiles.json` (fallback `~/.config`). A profile contains a name, optional product ID and serial matching fields, and an optional DPI. Applying a profile resolves the target device and uses the same validated DPI write path.

## GUI
A left device list, central device details, capability list, DPI slider/input when adjustable DPI is present, profile list, refresh/apply actions, and explicit daemon/offline errors. The UI title includes “Unofficial” and does not use Logitech logos or pretend to be G HUB.

## Error Handling
Errors cross RPC as structured strings with operation context. Device disconnects, permission errors, timeouts, unsupported features, invalid DPI, stale sockets, and daemon unavailability are surfaced directly. The daemon removes a stale socket on startup and sets socket permissions to `0600`.

## Testing
Pure protocol tests use captured public HID++ examples to verify framing, feature-index parsing, feature-list parsing, DPI range parsing, current DPI parsing, DPI validation, and set-DPI encoding. Core tests verify profile matching and XDG path behavior. Hardware tests are opt-in because the build environment cannot safely assume attached Logitech devices.

## Packaging
Arch `PKGBUILD` builds all release binaries, installs the udev rule, systemd user unit, desktop file, icon, and licenses. An install script performs a local `makepkg -si` workflow and reminds the user to reload udev and enable the user service.
