# AetherForge BEACN Control v0.1.20

v0.1.20 adds **On Device startup profiles** to the existing DragonGlass/Windows-parity BEACN control surface. When a supported BEACN Mic is present, the app queries the parameter set already stored on the microphone before the private DSP starts, maps the supported values into the AetherForge profile model, and activates that state as **On Device**.

The startup reader is intentionally one-way. `src/on_device.rs` builds its request list with `beacn_lib::audio::messages::Message::generate_fetch_message(...)` and dispatches those generated getter messages. It does not deliberately construct or issue BEACN setter messages. Hardware writes remain blocked by the existing fail-closed hardware boundary, and the physical audio device remains available to `snd_usb_audio`, ALSA, and PipeWire.

## Startup behavior

The normal startup sequence is:

`discover BEACN -> read mic parameters -> map supported DSP/headphone state -> cache read result -> activate On Device -> start private DSP`

If the current device read cannot complete but a cache for that microphone exists, the UI activates **ON DEVICE CACHED** and makes that fallback explicit. If neither the device read nor a cache is available, startup continues with the local profile rather than guessing that default values came from the mic.

The cache is stored under `~/.config/aetherforge-beacn-control/on-device/`. It is a host-side fallback only and is never presented as a successful live hardware read.

## New UI/UX

The Live Profile bar now has a dedicated DragonGlass **MIC MEMORY** strip. It shows the active source state (`READING MIC MEMORY`, `ON DEVICE ACTIVE`, `ON DEVICE CACHED`, `LOCAL EDIT`, `LOCAL PROFILE`, or unavailable), firmware, number of imported/unavailable settings, and a **RELOAD MIC** action.

When an On Device profile is active, changing a DSP control does not imply that the microphone was rewritten. The app immediately changes the profile context to **On Device - Local**, labels the state **LOCAL EDIT**, autosaves it locally, and leaves the mic memory unchanged. Loading another saved profile shows **LOCAL PROFILE**. Choosing REVERT while the actual On Device state is active re-reads the microphone.

The Device/Settings surfaces now make the ownership boundary explicit with **MIC MEMORY READ-ONLY**, **HARDWARE WRITES BLOCKED**, **RAW MIC PRESERVED**, and **SYSTEM DSP ISOLATED** states.

See `docs/UI-UX-v0.1.20.md` for the component/state contract and `docs/ON-DEVICE-PROFILE-v0.1.20.md` for the import mapping and safety boundary.

## What is imported

The reader maps supported BEACN Mic values for mic gain, active mic EQ mode/bands, compressor, expander, noise suppression, de-esser, exciter, mic output/headphone levels, headphone mode/FX, mono/balance, headphone-EQ linking, and per-ear headphone EQ.

The current `beacn-lib` v0.4.3 protocol model exposes nine hardware EQ bands for the microphone and each headphone channel, while the AetherForge local model retains its existing ten-band UI/DSP contract. The first nine slots are imported from the mic; the tenth remains at its safe local default. No missing/unknown parameter is written back to hardware.

## Private DSP isolation

The audible AetherForge path is unchanged:

`physical BEACN raw source -> targeted pw-record capture -> private Rust DSP -> namespaced PipeWire pipe source -> AetherForge BEACN Processed`

AetherForge BEACN Control does **not** use AetherStream or another system-DSP service. It does not set the system default source/sink. If the private worker stops, the raw BEACN microphone remains available.

The private Rust chain remains input gain -> 10-band mic EQ -> noise suppression -> expander -> compressor -> de-esser -> exciter -> output gain -> bounded safety limiter.

## BEACN vendor-interface permission

The package contains `packaging/50-aetherforge-beacn.rules`, using the standard `uaccess` mechanism for supported BEACN Mic USB IDs. A getter still has to send a USB request packet, so Linux must permit the active desktop user to access the vendor interface. The installer checks current access first and installs/reloads the rule only when needed. This OS permission is not itself a protocol write-protection mechanism; the write boundary is enforced by the application implementation and release contracts.

## Clean installation and host gate

`INSTALL-AND-VERIFY.sh` remains fail-fast. It runs the existing DragonGlass/render/private-DSP/audio-isolation contracts plus a new On Device getter-only contract, then runs `cargo fmt`, strict Clippy (`-D warnings`), tests, release build, private-DSP self-test, GUI smoke tests, output-profile recovery, and the read-only system probe before declaring PASS.

The previous installed binary, launcher, and desktop entry are backed up under:

`~/.local/share/aetherforge-beacn-control/rollback/pre-v0.1.20/`

The installed desktop launcher remains absolute-path based and logs ordinary GUI startup to `~/.local/state/aetherforge-beacn-control/launch.log`.

## Clean-room / third-party basis

User-supplied BEACN Windows installers remain static UI/interoperability reference material only and are not redistributed. The read-only parameter transport uses the MIT-licensed community `beacn-lib` v0.4.3 project; attribution is recorded in `THIRD_PARTY_NOTICES.md`. No proprietary BEACN binary, source, or artwork is included.
