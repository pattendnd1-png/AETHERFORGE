# ForgeHX HyperX Microphone Full-Stack Design

## Status

Approved architecture, written 2026-08-14.

## Goal

Make ForgeHX the native Linux control application for HyperX microphones, with the widest safe control surface available per model. ForgeHX must combine verified HyperX hardware controls with a ForgeHX-managed Linux audio/DSP path. It must never claim that a software DSP feature is stored in microphone firmware when it is not.

## Scope

### Target microphone matrix

The initial support matrix is the union of HyperX's current NGENUITY and NGENUITY Legacy microphone lists:

- FlipCast
- SoloCast 2
- QuadCast 2 S
- SoloCast
- DuoCast
- QuadCast S
- QuadCast 2

QuadCast 2 S appears in both generations and is one physical product family entry.

Future HyperX microphone models enter through the same registry and capability system. Unknown future devices remain safely detected but are not promoted to writable native support until their protocol is verified.

### Full-stack capability target

ForgeHX exposes the following controls when the selected model and transport support them:

Hardware/native layer:

- microphone gain/input level
- hardware mute and mute-state synchronization
- headphone/playback level
- direct-monitor/sidetone level or monitor mix
- selectable polar pattern
- hardware high-pass filter
- hardware presence/voice enhancement switch
- hardware auto-level/limiter controls where verified
- sample-rate and bit-depth mode where a writable hardware/UAC control is actually exposed
- RGB/aRGB/status lighting
- LED behavior for muted/live state
- firmware/hardware version readback
- physical control state synchronization
- hardware status and diagnostic reports

ForgeHX DSP layer:

- input trim
- high-pass filter
- parametric EQ
- noise suppression
- expander/noise gate
- compressor
- de-esser
- limiter
- output/makeup gain
- complete chain bypass
- dry/processed monitor selection
- processed virtual microphone source
- per-microphone presets/profiles
- level/peak metering

Routing/monitoring layer:

- raw HyperX microphone source
- processed ForgeHX virtual source
- monitor-to-headphones where the device exposes a headphone sink
- explicit source/sink target selection within the logical HyperX device
- latency-aware direct monitoring preference when hardware supports it

## Non-goals

- Firmware updates are governed exclusively by `2026-08-23-forgehx-hyperx-mic-firmware-manager-design.md`; generic/raw firmware flashing remains prohibited.
- No raw writes to unidentified vendor endpoints.
- No generic microphone DSP page for non-HyperX microphones.
- No claim that XLR-only FlipCast operation can be controlled through USB when the USB control path is not present.
- No browser-source or external web UI dependency.
- No system playback/output DSP, output equalizer, speaker enhancer, or JamesDSP integration.
- No EasyEffects runtime dependency; ForgeHX owns its microphone graph directly.

## Source-of-truth behavior

ForgeHX treats three classes of state separately:

1. **Hardware state**: read/written through verified HyperX HID/UAC/vendor controls.
2. **Linux audio state**: PipeWire/WirePlumber source/sink volume, mute, routing and stream graph.
3. **ForgeHX DSP state**: filter graph configuration owned by ForgeHX.

The UI labels the backend for each control so a user can see whether a setting is `ForgeHX Native`, `Linux Standard`, or `ForgeHX DSP`.

## Architecture

### New `forgehx-mic` crate

Responsibilities:

- HyperX microphone registry keyed by exact VID/PID plus interface/usage when required.
- Per-family protocol drivers.
- Model capability descriptors.
- Hardware state readback and writes.
- Safe transport selection.
- Translation between model-specific controls and common ForgeHX microphone types.

Suggested module layout:

```text
crates/forgehx-mic/
  src/lib.rs
  src/registry.rs
  src/transport.rs
  src/models/
    mod.rs
    quadcast2.rs
    quadcast2s.rs
    flipcast.rs
    solocast2.rs
    legacy_quadcast.rs
    duocast.rs
    solocast.rs
```

A model module may share a protocol-family implementation when packet layouts are identical. Model names do not force one-file-per-device if the protocol is shared.

### New `forgehx-dsp` crate

Responsibilities:

- ForgeHX microphone processing profile schema.
- Generation and lifecycle of PipeWire/WirePlumber processing nodes.
- One processed virtual microphone source per active HyperX mic.
- Filter ordering and parameter validation.
- Apply/bypass/rebuild operations.
- Metering interface.

The DSP crate does not write microphone firmware.

### Input-only processing invariant

ForgeHX microphone DSP is an **input-only** subsystem. It never owns, inserts into, processes, or redirects normal system playback/output streams. JamesDSP is not part of the ForgeHX microphone architecture. EasyEffects is not a ForgeHX runtime dependency.

The only supported software signal direction is:

```text
physical microphone capture
  -> ForgeHX input processing graph
  -> ForgeHX Processed Mic virtual source
  -> recording/voice applications
```

Optional monitoring is a tap of the microphone path for audition/sidetone only; it is not a general playback-effects pipeline. Any hardware direct-monitor path remains device-native and bypasses the software graph unless the user explicitly selects processed audition.

The initial implementation uses PipeWire's filter-chain facilities to construct the virtual source and processing graph. ForgeHX may load bounded built-in, LADSPA, or LV2 processors for microphone functions, but no processor may attach to arbitrary output streams.

### Existing `forgehx-audio`

Extend, do not replace, the existing PipeWire/WirePlumber adapter. It remains responsible for discovery, basic source/sink volume/mute, and stable node identity. Add helpers for identifying a microphone source and headphone/monitor sink that belong to the same physical HyperX USB device.

### `forgehx-core`

Add typed microphone capabilities and state rather than using generic `Microphone` alone.

Proposed capability additions:

```rust
MicGain,
MicHardwareMute,
MicHeadphoneVolume,
MicMonitorMix,
MicPolarPattern,
MicHardwareFilter,
MicLighting,
MicDsp,
MicRouting,
MicMetering,
```

Add common types such as:

```rust
pub enum PolarPattern {
    Cardioid,
    Omnidirectional,
    Bidirectional,
    Stereo,
}

pub struct MicrophoneHardwareState {
    pub gain: Option<f32>,
    pub muted: Option<bool>,
    pub headphone_volume: Option<f32>,
    pub monitor_mix: Option<f32>,
    pub polar_pattern: Option<PolarPattern>,
    pub high_pass_enabled: Option<bool>,
    pub presence_enabled: Option<bool>,
    pub auto_level_enabled: Option<bool>,
}

pub struct MicrophoneDspConfig {
    pub enabled: bool,
    pub input_gain_db: f32,
    pub high_pass_hz: Option<f32>,
    pub eq: Vec<ParametricEqBand>,
    pub noise_suppression: NoiseSuppressionConfig,
    pub gate: GateConfig,
    pub compressor: CompressorConfig,
    pub de_esser: DeEsserConfig,
    pub limiter: LimiterConfig,
    pub output_gain_db: f32,
}
```

Exact schemas are finalized during implementation planning, but all DSP parameters must be typed, bounded, and serializable.

## DSP chain

Default signal order:

```text
HyperX raw source
  -> input trim
  -> high-pass
  -> noise suppression
  -> expander/gate
  -> parametric EQ
  -> compressor
  -> de-esser
  -> limiter
  -> output gain
  -> ForgeHX processed source
```

The chain is rebuilt atomically. A failed rebuild leaves the previous working graph active or falls back to raw capture; it must not leave applications with a silently dead microphone.

Noise suppression may use a packaged Linux DSP provider (for example an RNNoise/WebRTC-capable PipeWire/LV2/LADSPA component) selected at package/build time. ForgeHX must detect availability and report it; it must not silently expose a control whose processor is absent.

## Hardware capability examples

These examples guide the registry, not marketing assumptions:

- **QuadCast 2 / QuadCast 2 S**: gain, playback/headphone level, monitoring level, mute, four polar patterns, lighting/status; 2 S additionally has large aRGB lighting capability.
- **FlipCast USB mode**: gain, headphone volume, monitor mix, tap mute, onboard high-pass, presence boost, LED/status controls, and software-level auto-level/EQ/limiter features where protocol support is verified.
- **SoloCast 2**: mute/status and its documented adjustable audio filter, plus Linux/DSP stack.
- **Legacy SoloCast / DuoCast / QuadCast S**: exact capability set is derived from verified USB/HID descriptors and protocol work; missing hardware features do not block the ForgeHX DSP path.

No feature is assigned to `ForgeHxNative` based only on a product page. A native capability requires a verified read/write path and regression tests.

## Device association

A logical HyperX microphone may expose multiple interfaces:

- USB audio source
- USB audio headphone sink
- HID/vendor control interface
- lighting interface

ForgeHX groups these into one logical device using serial where available, USB parent/path, VID/PID and interface topology. The daemon never presents each interface as a separate microphone when deterministic grouping is possible.

## Capability ownership

Priority for microphone controls:

1. ForgeHX Native hardware driver
2. ForgeHX DSP for processing-only features
3. Linux Standard for PipeWire volume/mute/routing
4. OpenRGB only as a lighting fallback when deterministic matching is available
5. Diagnostic read-only

Each capability has exactly one selected owner at a time.

## IPC and daemon

Add microphone-specific commands. The daemon remains the sole writer.

Minimum command set:

```text
MicState
MicSetGain
MicSetMute
MicSetHeadphoneVolume
MicSetMonitorMix
MicSetPolarPattern
MicSetHardwareFilter
MicLightingMetadata
MicSetLighting
MicDspGet
MicDspSave
MicDspApply
MicDspBypass
MicRoutingGet
MicRoutingSet
MicMeterState
```

IPC version increments only if the wire schema requires it. Clients must continue to perform the existing compatibility negotiation before sending a command.

## Profiles

Profiles become logical and backend-neutral. A HyperX microphone profile contains:

- hardware control values that the model supports
- DSP config
- routing preference
- lighting config when supported

Applying a profile reports per-section success/failure. A missing lighting backend, for example, does not prevent DSP or gain from applying.

## GUI

HyperX microphone pages:

- **Microphone**: gain, mute, hardware status, polar pattern, headphone level, monitor mix.
- **Processing**: high-pass, suppression, gate, compressor, de-esser, limiter, input/output gain, bypass.
- **EQ**: parametric EQ tailored to the microphone path.
- **Monitoring**: direct/hardware monitor controls and processed/raw audition selection.
- **Routing**: raw and ForgeHX processed source status, target sink/source mapping.
- **Lighting**: RGB/aRGB and live/muted state behavior where supported.
- **Profiles**: save/apply/delete mic profiles.
- **Doctor**: protocol/interface/backend diagnostics.

The GUI hides unsupported controls instead of disabling a wall of irrelevant widgets.

## Error handling

- Permission denied on control HID: show actionable udev/device detail; keep Linux/DSP controls alive.
- DSP provider missing: keep raw mic working and mark that processor unavailable.
- PipeWire restart/node renumber: resolve stable node name/physical association and rebuild processed source.
- Hardware command failure: fail only that capability and preserve the rest of the device.
- Device disconnect: tear down only the affected ForgeHX processing graph; reconnect should restore the last active profile when configured.
- XLR-only FlipCast: treat as external analog source unless its USB control interface is also connected.

## Packaging

Arch package must install:

- exact HyperX microphone udev rules as devices are verified
- PipeWire/WirePlumber dependencies
- any selected DSP provider dependencies
- ForgeHX-managed config under a namespaced path only

ForgeHX must not overwrite unrelated user PipeWire/WirePlumber configuration.

## Testing

Required layers:

- registry VID/PID/interface tests
- packet encoder/decoder tests for each native mic protocol family
- no-write tests for unknown devices
- hardware-capability ownership tests
- logical USB-audio/HID grouping tests
- DSP config validation tests
- PipeWire graph generation tests
- profile partial-failure tests
- GUI action/visibility tests
- udev exact-match package guards

Hardware integration tests run only when a matching VID/PID is present.

## Acceptance criteria

This release family is successful when:

1. Every microphone in the target matrix is identified as a HyperX microphone and is not collapsed into a generic USB-audio device when deterministic identity exists.
2. Every verified hardware capability is controllable from ForgeHX and shows `ForgeHX Native` ownership.
3. Every target microphone can use the ForgeHX DSP path even if its hardware protocol exposes few controls.
4. A ForgeHX processed virtual microphone source can be selected by normal Linux applications.
5. Hardware monitoring is preferred when available; software monitoring is clearly distinguished.
6. Missing RGB, hardware filters, or other optional controls never disable capture, DSP, or unrelated capabilities.
7. No unknown vendor write is issued.
8. No firmware flashing is exposed.

## Reference basis

The model matrix is based on HyperX's current NGENUITY and NGENUITY Legacy compatibility lists as checked on 2026-08-14. Hardware-control examples are grounded in current HyperX product documentation for FlipCast, QuadCast 2, QuadCast 2 S, and SoloCast 2; implementation still requires protocol verification before native write ownership is granted.
