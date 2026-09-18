# AetherForge BEACN Control v0.1.15

v0.1.15 is a clean-room Windows-BEACN-workflow reforge with a hard architectural separation from AetherForge system DSP.

## Windows reference basis

The user-supplied BEACN Windows installers are treated as static reference material only. They are never executed by the Linux build and no proprietary BEACN artwork, binaries, or source are redistributed. Their hashes and observed packaging/resource evidence are recorded in `REFERENCE-INSTALLERS.txt` and `docs/reference/BEACN-WINDOWS-STATIC-REFERENCE.md`.

The DragonGlass workspace preserves the Windows-style interaction model already established in the project: Live Profiles and snapshots, device/module rail, anchored **Equalizer & Enhancement**, secondary processing, Voice Recorder, **Mic Output**, and Enhanced Headphones. Secondary processing exposes Compressor, Expander, Noise Suppression, De-Esser, and Exciter. Microphone EQ is nine-band; the headphone profile model is ten-band per ear with linking, mono, balance, preset state, monitor/headphone levels, and binaural-personalization state.

## Hard DSP isolation

AetherForge BEACN Control does **not** use AetherStream or any other system-DSP service.

The physical BEACN device remains owned by Linux `snd_usb_audio`, ALSA, and PipeWire. Direct vendor USB/interface claims are blocked and the `beacn-lib` runtime dependency has been removed.

The private microphone path is:

`physical BEACN raw source -> targeted pw-record capture -> private Rust DSP -> namespaced PipeWire pipe source -> AetherForge BEACN Processed`

The private processor applies only to the app-owned processed source. It never changes the system default source/sink, never replaces the raw BEACN source, and never mutates system DSP. If the private worker stops, the raw BEACN microphone remains available.

The private Rust chain is input gain -> nine-band mic EQ -> noise suppression -> expander -> compressor -> de-esser -> exciter -> output gain -> bounded safety limiter.

## Live Profiles

Live Profiles use schema v2 and continuously persist source/sink identity, PipeWire level/mute state, and the complete BEACN profile model. Manual Snapshots create fixed restore points. Legacy profiles load with safe DSP defaults.

Profile changes are sent to the private DSP worker when it is running. When it is stopped, the controls remain editable and persistent but the app does not falsely report audible processing.

## Headphone boundary

The Windows-style Enhanced Headphones state/workflow remains available in the clean-room UI. v0.1.15 does not intercept or rewrite global system playback in order to make those profile controls audible, because that would violate the hard isolation requirement. Existing explicit BEACN/PipeWire output level and mute controls remain live.

## Build gate

The installer fails fast on Windows-parity/source contracts, private-DSP isolation, no-system-DSP coupling, no-direct-USB transport, no-default-device mutation, `cargo fmt --check`, strict Clippy with `-D warnings`, exactly 26 core tests, release build, the pure-Rust private-DSP self-test, graphical-session recovery, safe BEACN duplex-profile repair, read-only probe, and installation.

Rollback of the previous installed binary/desktop entry is stored under:

`~/.local/share/aetherforge-beacn-control/rollback/pre-v0.1.15/`


## v0.1.15 host-Clippy repair

This release preserves the v0.1.14 private-DSP architecture and Windows-style workflow while fixing the two Rust 1.98 strict-Clippy failures proven by the v0.1.14 host VERIFY: the stopped-capture guard no longer binds an unused error, and fixed-width f32 decoding uses `as_chunks::<4>()`. A static regression contract guards both patterns.
