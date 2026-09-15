# ReForge Logitech Linux v0.6.1

Compiler-fix patch for v0.6 Logitech Services.

- Fixes the Services page session-status `match` so both arms return `()`, resolving Rust E0308 in `crates/reforge-gui/src/app.rs`.
- Adds a release regression guard that rejects the value-returning `None => ui.label(...)` form.
- No HID++, fwupd/LVFS, udev, firmware, receiver, profile, or device-control behavior changes from v0.6.0.

# ReForge Logitech Linux v0.6.0

Logitech Services release.

- Persistent per-device HID++ sessions replace one-shot open/write/close operations for daemon-controlled Logitech hardware.
- HID++ transactions are serialized through the owning session, with health, reconnect count, last-success, and last-error state exposed to the control center.
- Unsolicited HID++ notifications are polled through the active session; battery and supported headset mute events update runtime state without forcing full discovery.
- udev event monitoring drives Logitech USB/hidraw add, remove, and change discovery with a ten-second fallback scan if udev monitoring is unavailable.
- New Services page and CLI report HID++, udev, fwupd, V4L2, and PipeWire backend health plus selected-device HID++ session state.
- Logitech firmware discovery and updates use fwupd/LVFS through fwupdmgr JSON output and normal fwupd/polkit trust checks. ReForge does not implement a private firmware flasher.
- Firmware page and CLI support device discovery, metadata refresh, release inspection, update availability, and explicit confirmed installation.
- Firmware installation is restricted to enumerated Logitech devices with an advertised update and never adds force, downgrade, reinstall, or safety-bypass flags.
- HID permissions use logind uaccess ACLs; the package does not install world-writable hidraw rules.
- Existing v0.5 stable identity, offline retention, reconnect restoration, telemetry, profile lifecycle, preferences, diagnostics, G515 deduplication, RGB, camera, and audio controls remain intact.

ReForge remains an independent compatibility project. It uses local HID++/Linux device services and supported Linux firmware infrastructure; it does not require or emulate undocumented G HUB cloud authentication.
