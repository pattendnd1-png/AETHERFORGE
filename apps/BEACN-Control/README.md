# AetherForge BEACN Control v0.1.13

## v0.1.13 host-gate repair

v0.1.13 is a surgical follow-up to v0.1.12. It changes the new software-DSP clamp test to initialize `SoftwareDspState` with struct-update syntax so Rust 1.98 strict Clippy no longer rejects the test with `clippy::field_reassign_with_default`. The Windows-BEACN-style UI, Live Profiles, 10-band headphone workflow, PipeWire/ALSA pass-through ownership, direct-USB block, and AetherStream capability gating are otherwise retained unchanged.

v0.1.13 is the Windows-BEACN-1.4 workflow parity release built on the host-qualified v0.1.11 system-audio protection baseline.

## Windows-style AetherForge workspace

The clean-room DragonGlass workspace now centers the same interaction model used by the current Windows BEACN generation: collapsible device/module rail, Live Profiles, anchored **Equalizer & Enhancement**, tabbed secondary processing, Voice Recorder, and a distinct end-of-chain **Mic Output** surface.

Secondary processing includes Compressor, Expander, Noise Suppression, De-Esser, Exciter, and Enhanced Headphones. Microphone EQ remains a nine-band parametric model. Enhanced Headphones use a ten-band parametric EQ per ear with left/right selection, linking, mono mode, balance, monitor/headphone levels, preset state, and binaural-personalization state.

## Live Profiles

Live Profiles use schema v2 and continuously persist source/sink identity, PipeWire level/mute state, and the complete software-DSP control state. Manual Snapshots create fixed restore points. Legacy v0.1.11 profiles load with safe DSP defaults.

## System-audio protection

Linux `snd_usb_audio`, ALSA, and PipeWire remain the permanent BEACN audio owners. Direct userspace USB DSP claims remain hard-blocked; the application does not call `HardwareController::connect()` and cannot silently fall back to the vendor interface.

The pinned `beacn-lib` v0.4.3 protocol implementation remains staged for reference/future non-disruptive transport work, not active runtime ownership.

## AetherStream DSP boundary

AetherStream remains authoritative for system-wide PipeWire/software-DSP execution. v0.1.13 exposes a truthful DSP-backend capability state. The Windows-style controls are fully editable, autosaved, snapshotted, and restorable; they become audible software-DSP mutations only through a separately verified AetherStream mutating adapter. v0.1.13 does not guess or fabricate postcard enum/wire layouts and does not report a live DSP change when that backend is unavailable.

Existing proven operations remain live: PipeWire source/sink selection by stable node name, volume/mute, 10-second recorder, duplex-profile repair, node-health recovery, graphical-session recovery, and read-only probe generation.

## Build gate

The installer fails fast on source contracts, `cargo fmt --check`, strict Clippy with `-D warnings`, exactly 20 core tests, release build, graphical-session recovery, safe BEACN duplex-profile repair, read-only probe, and installation.

Rollback of the previous installed binary/desktop entry is stored under:

`~/.local/share/aetherforge-beacn-control/rollback/pre-v0.1.13/`
