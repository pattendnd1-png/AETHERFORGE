# AetherForge BEACN Control v0.1.23

v0.1.23 keeps the host-qualified v0.1.22 read-only mic-memory importer, same-mic cache guard, locked dependency gate, DragonGlass UI, and system-audio protections, then changes one important runtime rule: **On Device is now hardware-authoritative**.

BEACN's own support material describes the microphone chain's Mic Output as the signal reaching the computer and describes custom mic profiles as being retained on the microphone itself. Because the Linux BEACN capture node can therefore already represent the microphone's onboard-processed output, v0.1.23 no longer mirrors the imported On Device profile into AetherForge private DSP at startup. That avoids applying the saved mic chain twice.

## Startup behavior

The normal startup sequence is now:

`discover BEACN -> read mic parameters -> map supported state -> cache read result -> activate On Device -> keep hardware capture authoritative -> private DSP bypassed`

If the live read cannot complete but a **same-mic, serial-verified** cache exists, the UI shows **ON DEVICE CACHED**. The cache is provenance/display state only; it does not cause AetherForge to replay the saved hardware chain in private DSP. If neither a device read nor matching cache is available, startup continues with the current local profile and the private DSP path remains available.

The cache remains under `~/.config/aetherforge-beacn-control/on-device/` and is accepted only for the connected mic identity.

## Hardware-authoritative On Device mode

While `On Device` is active:

- the imported microphone settings are shown read-only;
- the BEACN capture node remains the authoritative audio path;
- the AetherForge private DSP worker is intentionally bypassed;
- hardware setter messages remain blocked;
- system defaults are not changed;
- accidental edits are restored from the imported baseline rather than silently starting a duplicate software chain.

The MIC MEMORY strip shows **HARDWARE DSP ACTIVE** and **PRIVATE DSP BYPASSED** so the authority is visible rather than implicit.

## Local overlay workflow

Use **CREATE LOCAL OVERLAY** when you want AetherForge-only processing. v0.1.23 deliberately creates that overlay from a **flat `SoftwareDspState::default()` baseline** instead of cloning the imported On Device chain. This prevents the common double-processing failure where the same EQ/dynamics settings are applied once in BEACN hardware and again in software.

The local overlay is named `On Device - Local Overlay`. Mic memory is left unchanged. Loading a saved local profile also activates/synchronizes the private DSP worker so profile changes are actually audible.

## Read-only mic-memory transport

`src/on_device.rs` builds its request list with `beacn_lib::audio::messages::Message::generate_fetch_message(...)` and dispatches generated getter messages. It does not deliberately construct or issue BEACN setter messages. Hardware writes remain blocked by the fail-closed hardware boundary, and the physical device remains owned by `snd_usb_audio`, ALSA, and PipeWire.

## Imported state

The reader maps supported BEACN Mic values for mic gain, active mic EQ mode/bands, compressor, expander, noise suppression, de-esser, exciter, mic output/headphone levels, headphone mode/FX, mono/balance, headphone-EQ linking, and per-ear headphone EQ.

The current `beacn-lib` v0.4.3 protocol model exposes nine hardware EQ bands for the microphone and each headphone channel, while AetherForge retains its existing ten-band local model. The first nine slots are imported from the mic; the tenth stays at its safe local default. No missing/unknown parameter is written back to hardware.

## Private DSP isolation

The local overlay path remains:

`BEACN capture -> targeted pw-record capture -> private Rust DSP -> namespaced PipeWire pipe source -> AetherForge BEACN Processed`

This path is **local-only** and starts only outside hardware-authoritative On Device mode. AetherForge BEACN Control does not use AetherStream or another system-DSP service and does not set the system default source or sink.

## Host gate

`INSTALL-AND-VERIFY.sh` is fail-fast. It now includes `v0.1.23-hardware-authority-contract.sh` in addition to the existing UI, mic-memory, cache, private-DSP, audio-isolation, locked dependency, Clippy, test, release-build, GUI-smoke, output-profile and probe gates.

The gate reports:

`AETHERFORGE_BEACN_ON_DEVICE_DSP_AUTHORITY=HARDWARE`

`AETHERFORGE_BEACN_PRIVATE_DSP_START_POLICY=LOCAL_ONLY`

The previous installed binary, launcher, and desktop entry are backed up under `~/.local/share/aetherforge-beacn-control/rollback/pre-v0.1.23/`.

## Clean-room / third-party basis

User-supplied BEACN Windows installers remain static UI/interoperability reference material only and are not redistributed. The read-only parameter transport uses the MIT-licensed community `beacn-lib` v0.4.3 project; attribution is recorded in `THIRD_PARTY_NOTICES.md`. No proprietary BEACN binary, source, or artwork is included.
