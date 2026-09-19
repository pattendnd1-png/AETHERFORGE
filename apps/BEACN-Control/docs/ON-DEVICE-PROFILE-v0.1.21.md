# v0.1.21 On Device Profile Import

## Read path

`src/on_device.rs` enumerates supported BEACN Mic devices through `beacn-lib` v0.4.3, opens the vendor parameter interface, asks `Message::generate_fetch_message(DeviceType::BeacnMic, firmware)` for the firmware-appropriate getter list, and dispatches those getter requests one at a time.

The module has one device-message dispatch call (`device.handle_message(request)`) and the request variable is sourced from the generated fetch list. Direct `set_value`, `param_set`, hardware-controller connect, and BEACN setter paths are absent from the module and are checked by `scripts/v0.1.21-on-device-profile-contract.sh`.

## Mapping

Supported responses are translated into the existing `SoftwareDspState`:

- mic setup: gain;
- microphone EQ: active Simple/Advanced mode, type, gain, frequency, Q, enabled;
- compressor: active mode, enabled, threshold, ratio, attack, release, makeup gain;
- expander: active mode, enabled, threshold, ratio, attack, release;
- suppressor: enabled, amount, style, sensitivity, adapt time;
- de-esser: enabled and amount;
- exciter: enabled, amount, hardware frequency mapped monotonically to the current local Tone control;
- headphones: level, mic monitor, mic output gain, headphone type, FX enable;
- controls: mono and balance;
- headphone EQ: linked state plus per-ear type/gain/frequency/Q/enabled.

`beacn-lib` v0.4.3 currently enumerates `Band1..Band9`. AetherForge keeps its ten-band local DSP model. Imported hardware bands populate slots 1–9; slot 10 remains its safe local default unless/until the protocol library exposes a corresponding hardware band.

## Cache semantics

A successful live read is serialized into `~/.config/aetherforge-beacn-control/on-device/<serial>.profile`. This cache exists only to avoid throwing away the last known mic-derived configuration when a later read is temporarily unavailable. Cache provenance is preserved in the UI and never labeled as a live device read.

Unknown or unsupported hardware parameters are not preserved as raw blobs because this release never writes a profile back to the mic. Consequently, unsupported fields remain untouched on hardware rather than being round-tripped or zeroed.

## Audio ownership

The BEACN audio interfaces remain with Linux audio. The vendor query interface is separate from the raw capture/playback path. The app keeps its existing fail-closed `HardwareController` write boundary and private-DSP architecture.
