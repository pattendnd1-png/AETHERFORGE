# AetherForge BEACN Control v0.1.19

v0.1.19 is the launch-hardening release built directly on the fully host-qualified v0.1.18 render-target/private-DSP baseline. It fixes desktop startup without changing the qualified BEACN audio topology, private DSP, profile model, direct-USB block, or system-DSP isolation.

## v0.1.19 launch repair

v0.1.18 built, tested, probed, and installed successfully but could still fail when started from the Plasma application menu because its desktop entry used a PATH-dependent `Exec=aetherforge-beacn-control`. v0.1.19 removes that ambiguity. The desktop entry is generated with an absolute launcher path, and the launcher itself contains the absolute installed binary path.

The launcher writes normal GUI startup output to `~/.local/state/aetherforge-beacn-control/launch.log`, so a future desktop-startup failure is diagnosable instead of silent. The installer also opens a real self-closing eframe window from both the built binary and the installed launcher with Wayland/X11 display variables deliberately unset. Graphical-session recovery and actual native window creation must therefore both succeed before installation may report PASS.

## Canonical visual target

The user-approved AetherForge BEACN Control render is the UI contract for this release. The application now uses the same major composition and workflow:

- compact AetherForge-branded left navigation rail;
- Home, Microphone, Headphones, Profiles, Recorder, Routing, and Settings destinations;
- Live Profile command bar with Save, Save As, Revert, snapshots, and autosave state;
- Microphone tabs for Equalizer & Enhancement, Secondary Processing, Mic Output, and LED Control;
- large 10-band microphone EQ canvas;
- real-time Input / DSP / Output meter bank;
- processor-card row for Noise Suppression, Expander, Compressor, De-Esser, Exciter, and safety Limiter;
- right-side BEACN status, Quick Presets, and Monitor Mix surface on wide layouts;
- bottom capability strip matching the approved render hierarchy;
- DragonGlass navy/indigo/violet/cyan visual language.

The UI remains responsive down to the existing 640x480 minimum. Compact mode collapses the side rails and keeps the same functions accessible through the responsive navigation controls.

## Windows reference basis

The user-supplied BEACN Windows installers are static reference material only. They are never executed by the Linux build, and no proprietary BEACN artwork, binaries, or source are redistributed. Their hashes and observed packaging/resource evidence remain recorded in `REFERENCE-INSTALLERS.txt` and `docs/reference/BEACN-WINDOWS-STATIC-REFERENCE.md`.

## Hard DSP isolation

AetherForge BEACN Control does **not** use AetherStream or any other system-DSP service.

The physical BEACN device remains owned by Linux `snd_usb_audio`, ALSA, and PipeWire. Direct vendor USB/interface claims remain blocked and `beacn-lib` is not a runtime dependency.

The private microphone path is:

`physical BEACN raw source -> targeted pw-record capture -> private Rust DSP -> namespaced PipeWire pipe source -> AetherForge BEACN Processed`

The private processor applies only to the app-owned processed source. It never changes the system default source/sink, never replaces the raw BEACN source, and never mutates system DSP. If the private worker stops, the raw BEACN microphone remains available.

The private Rust chain is input gain -> 10-band mic EQ -> noise suppression -> expander -> compressor -> de-esser -> exciter -> output gain -> bounded safety limiter.

## Quick Presets

The render-target right column exposes Broadcast, Streaming, Podcast, Voice Chat, Music, and Custom 1. Presets only edit the private BEACN profile/DSP state and autosave into the active Live Profile. They never change the system audio graph or defaults.

## Headphone boundary

The Enhanced Headphones workflow remains a private profile/control surface with 10-band per-ear EQ, link/unlink, mono, balance, monitor/headphone levels, preset state, and binaural-personalization state. v0.1.19 does not intercept global system playback to make those profile controls audible; doing so would violate the hard isolation rule.

## Clean installation and host gate

The installer is fail-fast. It runs the render-target contract, Windows-parity contracts, private-DSP isolation, no-system-DSP coupling, no-direct-USB transport, no-default-device mutation, `cargo fmt --check`, strict Clippy with `-D warnings`, exactly 26 core tests, release build, private-DSP self-test, graphical-session recovery, safe BEACN duplex-profile repair, read-only probe, and only then replaces the installed binary/desktop entry.

The previous installed binary, launcher wrapper (when present), and desktop entry are copied to:

`~/.local/share/aetherforge-beacn-control/rollback/pre-v0.1.19/`

The desktop database is refreshed after installation when the helper is available.

## v0.1.19 launch hardening

The installed desktop file uses an absolute `Exec=`/`TryExec=` path to `~/.local/bin/aetherforge-beacn-control-launch`. That wrapper uses an absolute path to the installed binary and logs ordinary desktop launches to `~/.local/state/aetherforge-beacn-control/launch.log`. The release gate smoke-tests a real eframe window from the build artifact and again through the installed wrapper before the installer can declare success.
