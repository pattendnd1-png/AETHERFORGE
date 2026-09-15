# ReForge Logitech Linux

ReForge Logitech Linux is an **unofficial**, Linux-native Logitech HID++ control stack written in Rust. It keeps ordinary mouse/keyboard input in the Linux kernel and performs vendor-specific configuration in a per-user daemon through HIDAPI/hidraw.

It is not Logitech G HUB, is not affiliated with Logitech, and contains no Logitech proprietary source code, artwork, or branding. The desktop app uses a ReForge system-themed control-center design.

## v0.6 Logitech Services

v0.6 keeps the v0.5 reliability foundation and adds a real daemon-owned service layer around Logitech hardware and supported Linux firmware infrastructure:

- Persistent HID++ sessions keep one serialized connection per online Logitech HID++ device instead of reopening hidraw for every daemon operation.
- Session health exposes Ready/Degraded/Disconnected state, reconnect count, last successful transaction, and bounded error detail.
- HID++ notifications can update battery and supported headset mute state without a full device rescan.
- udev events drive USB/hidraw hot-plug discovery; a ten-second scan remains as a fallback.
- Services status covers HID++, udev, fwupd, V4L2/UVC, and PipeWire.
- Firmware discovery and installation use fwupd/LVFS, including metadata refresh, release inspection, update availability, and explicit install confirmation.
- Firmware updates remain under fwupd/polkit trust and authorization; ReForge never adds force, downgrade, reinstall, or safety-bypass options.
- Logitech hidraw access is granted to the active local desktop session with `TAG+="uaccess"`; the package does not make hidraw world-writable.
- Existing stable device identity, reconnect restoration, profiles, diagnostics, G515 deduplication, per-key RGB, camera controls, and audio controls remain available.

ReForge does not require a Logitech cloud login for local device control and does not emulate undocumented G HUB account endpoints.

### Services and firmware CLI

```bash
reforge-logitechctl services
reforge-logitechctl services reconnect hidpp
reforge-logitechctl session 1

reforge-logitechctl firmware devices
reforge-logitechctl firmware refresh
reforge-logitechctl firmware releases DEVICE_ID
reforge-logitechctl firmware update DEVICE_ID
```

For firmware support on Arch, install fwupd:

```bash
sudo pacman -S --needed fwupd
```

## Safety boundary

ReForge exposes typed HID++ operations only. Firmware flashing/DFU and a generic arbitrary raw-report API remain intentionally disabled. Lighting controls are shown only when the device advertises the corresponding feature/capability.

## Upgrade/install on Arch Linux

Extract the v0.6 source ZIP/tarball, enter the extracted directory, then run:

```bash
./scripts/install-arch.sh
```

Run the installer as your normal desktop user, **not** with `sudo`. `makepkg` installs required build/runtime dependencies, runs the Rust test suite, builds the workspace, installs/upgrades the package, reloads udev, and restarts the per-user ReForge daemon so the new v0.6 code is active immediately.

After installation:

```bash
reforge-logitechctl health
reforge-logitechctl devices
reforge-logitech
```

## Reliability CLI examples

```bash
# Stable device lifecycle registry
reforge-logitechctl runtime

# Force an immediate discovery merge
reforge-logitechctl rescan

# Read lightweight telemetry for device 1
reforge-logitechctl telemetry 1

# Inspect the daemon diagnostic snapshot
reforge-logitechctl diagnostics show

# Export a redacted diagnostic bundle
reforge-logitechctl diagnostics export ''

# Profile lifecycle
reforge-logitechctl profile rename Gaming Gaming-Desktop
reforge-logitechctl profile clone Gaming-Desktop Gaming-Copy
reforge-logitechctl profile export profiles.json
```

## Lighting CLI examples

Devices are numbered from 1 in CLI output:

```bash
# Current lighting state
reforge-logitechctl lighting get 1

# Static cyan
reforge-logitechctl lighting set 1 --effect static --color '#00bfff'

# Breathing with a 2.5-second period
reforge-logitechctl lighting set 1 --effect breathing --color '#8a2be2' --period 2500

# Wave
reforge-logitechctl lighting set 1 --effect wave --color '#00bfff' --secondary '#ff2bd6' --period 3000

# Host-rendered gradient
reforge-logitechctl lighting set 1 --effect gradient --color '#00bfff' --secondary '#8a2be2'

# Screen reactive
reforge-logitechctl lighting set 1 --effect screen

# Audio reactive
reforge-logitechctl lighting set 1 --effect audio --color '#00ff88' --secondary '#001010'

# Set HID lighting key/zone 0x04 (A on standard USB HID keyboards)
reforge-logitechctl lighting key 1 4 '#ff0000'
```

## Profiles and automatic game switching

```bash
# Save current DPI + lighting for device 1
reforge-logitechctl profile save Gaming 1

# Save and auto-activate for exact process names
reforge-logitechctl profile save SpaceGame 1 \
  --app game.exe \
  --app launcher \
  --auto-switch

reforge-logitechctl profile list
reforge-logitechctl profile apply Gaming 1
```

The daemon checks running `/proc` process/executable names and switches only when a saved profile has `auto_switch` enabled and an exact case-insensitive process-name match.

## Reactive integrations

Check what is available:

```bash
reforge-logitechctl integrations
```

Optional Arch packages:

```bash
# wlroots Wayland (Sway/Hyprland etc.) screen sampling
sudo pacman -S --needed grim

# X11 screen sampling fallback
sudo pacman -S --needed imagemagick

# PipeWire audio-reactive sampling
sudo pacman -S --needed pipewire
```

The audio-reactive mode asks PipeWire to capture sink output (`stream.capture.sink=true`) so the lighting reacts to system playback. The screen-reactive mode averages the current captured frame before sending a lighting frame.

## Existing pointer controls

```bash
reforge-logitechctl dpi get 1
reforge-logitechctl dpi set 1 1600
```

DPI writes are still validated against the exact values the device reports and read back after every write.

## Generic device-control CLI

```bash
# List verified controls for device 1
reforge-logitechctl controls 1

# Live-read one control
reforge-logitechctl control get 1 hidpp:report_rate

# Set a typed control
reforge-logitechctl control set 1 hidpp:report_rate 1
```

Camera controls use IDs such as `v4l2:exposure_absolute`; PipeWire controls are listed by `controls`. Only advertised writable values are accepted.

## Build from source

Required Rust version: **1.92 or newer**.

```bash
./scripts/build-release.sh
```

Or:

```bash
cargo test --workspace
cargo build --release --workspace
```

Release binaries:

- `target/release/reforge-logitech` — system-themed desktop control center
- `target/release/reforge-logitechctl` — CLI
- `target/release/reforge-logitechd` — per-user hardware/effects daemon

## Architecture

```text
ReForge Control Center ─┐
                       ├─ JSON RPC over $XDG_RUNTIME_DIR/reforge-logitech/reforge.sock
reforge-logitechctl ───┘
                                    │
                          reforge-logitechd
                         ┌──────────┴──────────┐
                    profile watcher      effect scheduler
                         │                     │
                         └──────────┬──────────┘
                                    │
                              reforge-hid
                                    │
              typed HID++ DPI / brightness / RGB / per-key calls
                                    │
                           hidapi / Linux hidraw
                                    │
                  kernel usbhid / hid-logitech-* drivers
```

Crates:

- `reforge-protocol`: HID++ framing, feature IDs, DPI and lighting wire parsers/builders.
- `reforge-hid`: discovery, capability probing, typed hardware operations, verification.
- `reforge-core`: shared device/lighting/profile models, software effect renderer, RPC/client.
- `reforge-logitechd`: per-user RPC daemon, host effect scheduler, reactive capture, app watcher.
- `reforge-logitechctl`: CLI.
- `reforge-logitech`: egui desktop control center.

## Troubleshooting

```bash
systemctl --user status reforge-logitechd.service
journalctl --user -u reforge-logitechd.service -b
reforge-logitechctl health
reforge-logitechctl devices
reforge-logitechctl integrations
```

If a Logitech hidraw node is inaccessible:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw
```

If the keyboard appears but Lighting is unavailable, run:

```bash
reforge-logitechctl --json devices
```

and inspect the advertised HID++ feature list. ReForge deliberately does not assume RGB capability based only on a product name.

## Verification note

The source/package generator environment used by ChatGPT does not contain `cargo`/`rustc`, so it cannot truthfully claim an in-sandbox Rust compile. The Arch installer runs `cargo test --workspace` and `cargo build --release --workspace` as part of `makepkg`; a failed compile/test stops installation rather than replacing a working package with an unverified build.

## License

MIT. Logitech and related marks are trademarks of their respective owner. ReForge Logitech Linux is an independent compatibility project.


## Logitech-wide discovery

ReForge 0.4 discovers Logitech hardware through HID++, USB/sysfs, V4L2/UVC, and PipeWire. A physical device is shown once when stable identity can be established. The Device Controls page exposes only controls reported writable by the active Linux provider. Camera controls require `v4l2-ctl` from `v4l-utils`; audio controls require `wpctl` from WirePlumber.
