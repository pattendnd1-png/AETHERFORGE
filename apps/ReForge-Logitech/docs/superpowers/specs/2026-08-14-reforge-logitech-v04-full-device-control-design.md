# ReForge Logitech v0.4 Full Device Control Design

Status: Approved design; ready for implementation planning after written-spec review.

## Goal

Expand ReForge from capability discovery plus a small set of writable controls into a capability-driven Logitech control center where every positively identified, safely writable setting on a listed device is exposed through a typed backend, RPC, CLI, profile system, and GUI control surface.

The release must preserve the v0.3.1 safety boundary: unsupported or ambiguous proprietary functions remain visible as detected capabilities where useful, but ReForge does not expose arbitrary raw HID/USB report injection or guess undocumented payloads.

## Scope

v0.4 covers five control families:

1. HID++ keyboards, mice, gaming peripherals, receivers, headsets, microphones, speakers, and other Logitech HID++ devices.
2. V4L2/UVC camera controls exposed by the Linux kernel.
3. PipeWire audio endpoint controls exposed by the Linux audio graph.
4. Logitech receiver/pairing state that can be safely enumerated and changed through verified protocol operations.
5. Generic Logitech USB/Bluetooth/HID inventory that has no verified writable backend yet.

A listed physical device may aggregate more than one provider. For example, a headset may combine HID++ battery/EQ/lighting with PipeWire volume/mute. A camera may combine USB identity with one or more V4L2 nodes. Providers must merge into one physical device card rather than creating duplicate cards.

## Non-Goals

v0.4 does not:

- Implement firmware flashing or bootloader recovery.
- Expose arbitrary raw HID/USB report editors.
- Claim control for capabilities ReForge cannot positively identify and validate.
- Reimplement the Linux input, UVC, ALSA, or PipeWire kernel/userspace stacks.
- Bypass device security, pairing authentication, or firmware safeguards.
- Copy Logitech proprietary source code, artwork, trademarks, or application assets.

## Architecture

The existing crate boundaries remain in place.

### `reforge-core`

`reforge-core` defines provider-neutral device and control models. v0.4 extends `DeviceControl` so controls can represent:

- toggle
- integer/ranged value
- choice/menu
- action
- multi-value/vector settings where needed for EQ and similar structured settings
- read-only status/telemetry

Every writable control carries:

- stable logical control ID
- provider
- provider endpoint
- backend feature/control identifier
- value type and limits
- current value when readable
- read/write capability flags
- persistence/profile eligibility
- optional grouping metadata for GUI sections

Structured controls must remain typed. A HID++ EQ band list is not flattened into an arbitrary byte string.

### `reforge-hid`

`reforge-hid` remains the only crate allowed to perform Logitech HID++ device I/O. It gains capability adapters with a common internal interface:

- probe supported controls from the enumerated HID++ feature set
- read a control when the protocol supports reads
- validate a requested value against the device-reported range/choices
- perform the typed write
- optionally read back and verify

Adapters are grouped by responsibility rather than model name. Initial adapter families are:

- pointer/DPI/report-rate
- wheel/scroll/SmartShift
- button and key assignments
- onboard profile mode/profile selection
- keyboard Fn/G-key/function behavior
- brightness/backlight
- zone/per-key/RGB lighting
- battery/power/charging status and writable power settings where exposed
- audio volume/mute/sidetone
- basic/advanced EQ where the device exposes a verified HID++ EQ feature
- microphone controls where exposed
- headset RGB/lighting where exposed
- receiver and pairing controls where safe and verified

Model-specific quirks may specialize an adapter, but the public control model remains feature-driven.

### `reforge-daemon/providers.rs`

Linux-standard providers remain outside `reforge-hid`.

#### V4L2/UVC

ReForge enumerates all writable V4L2 controls reported by `v4l2-ctl --list-ctrls-menus`, including common controls such as:

- brightness
- contrast
- saturation
- hue
- sharpness
- gain
- exposure mode/value
- focus automatic/manual
- zoom
- pan/tilt
- white-balance mode/temperature
- privacy
- power-line frequency
- any other kernel-reported writable integer, boolean, menu, integer-menu, or button control

The parser must honor disabled/read-only flags instead of blindly marking every supported type writable.

#### PipeWire

ReForge exposes safe endpoint controls available through `wpctl`, initially:

- volume
- mute
- default-route actions where a Logitech endpoint is a valid target

HID++ audio controls supplement rather than replace PipeWire transport controls.

### Daemon control router

`SetControl` becomes provider-aware for all supported providers. The daemon resolves the selected device, looks up the exact advertised `DeviceControl`, revalidates the requested value, and dispatches to:

- `reforge_hid::set_control()` for HID++
- V4L2 backend for V4L2
- PipeWire backend for PipeWire

The GUI and CLI never send raw provider commands.

## Physical Device Identity and Deduplication

One physical device must appear once.

The deduplication layer merges provider records using the strongest available identifiers in this order:

1. shared stable serial/unique ID plus vendor/product identity
2. shared USB sysfs ancestor
3. known HID++ receiver-child identity plus capability fingerprint
4. matching product/vendor identity only when an additional stable transport/endpoint relationship proves they are the same physical device

Receiver slot mirrors that return the same identity and capability fingerprint as the direct device are suppressed. Distinct paired devices must never be collapsed merely because they share a model name or receiver path.

The daemon retains internal routing data required to address a paired receiver child even when only one device card is shown.

## HID++ Controls

### Mice and pointing devices

Where the feature set supports them, expose:

- adjustable DPI and device-reported DPI choices/ranges
- pointer speed
- report rate and extended report rate
- SmartShift threshold/mode
- ratchet/free-spin controls
- high-resolution wheel mode and direction
- thumb-wheel mode and direction
- button assignment/remapping
- G-Shift or equivalent modifier behavior where exposed
- onboard profile enable/selection
- battery and charging telemetry
- RGB/lighting controls

### Keyboards

Where exposed, support:

- brightness and backlight mode/level/timeouts
- zone lighting and per-key RGB
- supported firmware effects
- Fn inversion / function-key mode
- G/M key diversion or assignments
- report rate
- onboard profile mode/selection
- supported key remapping/assignments
- analog-key actuation/rapid-trigger/haptic tuning where the feature exists
- battery and charging telemetry

The G515 LS TKL remains a first-class validation device: it must appear as one keyboard and retain 95-zone per-key RGB plus brightness control.

### Headsets, microphones, speakers, and audio devices

Where exposed through HID++, support:

- device volume and mute controls
- sidetone
- microphone mute/gain or verified microphone settings
- basic/graphic/advanced EQ controls with explicit validated band data
- lighting/RGB where available
- battery/charging status
- power/auto-sleep settings where safely writable

PipeWire volume/mute remain separate controls when the same hardware also exposes a PipeWire endpoint.

### Receivers and pairing

Receiver controls are intentionally narrower than device controls. ReForge may expose:

- receiver identity
- paired slot/device status
- safe unpair/pair operations only when the verified protocol implementation can positively identify the receiver type and required operation

ReForge must not send generic pairing writes to an unknown receiver family.

## Control Readback and State

A control has one of three readback classes:

1. **Readable**: ReForge reads the current device state and updates the UI after writes.
2. **Write-only canonical state**: the protocol has no read operation; ReForge persists the last successfully written typed state as canonical, as already done for per-key RGB.
3. **Telemetry-only**: current status is readable but not writable.

When a write can be verified by readback, failure to observe the requested state is reported as an error rather than silently treated as success.

## Profiles

Profiles expand from DPI, lighting, and generic provider values to a complete typed control snapshot.

A profile stores only controls marked profile-eligible. Transient actions such as “pair” or momentary camera buttons are not persisted.

Profile application order is deterministic:

1. device mode/onboard profile prerequisites
2. performance/input controls
3. lighting control/claim state
4. lighting values/per-key frame
5. audio/camera/provider controls

Application continues only where independent writes are safe; failures are reported with the specific control that failed. The daemon must not partially commit a per-key lighting frame.

Automatic game/application profiles continue to use the existing process matcher and reapply all eligible controls through the same typed control router.

## GUI

The system-themed G HUB-style UI keeps one device-centric navigation model. A device shows only sections for capabilities it actually has:

- Overview
- Assignments
- Performance
- Lighting
- Audio
- Camera
- Power
- Profiles
- Receiver

The generic `Device Controls` page remains as a diagnostic/fallback view but is no longer the primary experience for common device families.

UI controls are generated from typed metadata:

- Toggle → switch
- Range → slider/spin field using min/max/step
- Choice → combo box
- Action → button with confirmation for destructive operations
- Structured EQ → band editor
- Telemetry → read-only status row

No disabled placeholder control is shown merely to resemble G HUB.

## CLI

The CLI gains provider-neutral commands to:

- list all controls for a selected device
- read a control
- set a writable control by typed logical value
- apply a profile

Existing specialized commands such as DPI may remain as convenience aliases, but they route through the same typed control implementation where practical.

## Error Handling

Every provider returns actionable errors containing:

- device name/key
- control label/ID
- provider
- rejected value or operation
- backend/protocol failure reason

Transient HID++ BUSY/timeout behavior uses bounded retry logic already present for lighting operations. Permanent protocol errors are not retried indefinitely.

Missing Linux helper tools (`v4l2-ctl`, `wpctl`) make that provider unavailable without breaking HID++ discovery or other providers.

If a device disconnects during a write, ReForge invalidates its cached record and refreshes discovery on the next operation.

## Testing

### Core model tests

- serialization round trips for every new control value type
- profile persistence/migration from v0.3.1 profiles
- profile filtering excludes actions/read-only telemetry

### HID++ adapter tests

Use captured/synthetic protocol vectors and fake transports to test:

- feature-to-control probing
- range/choice validation
- payload encoding
- readback parsing
- error handling
- receiver child routing

No hardware is required for these unit tests.

### Provider tests

- V4L2 parser handles ranges, booleans, menus, disabled/read-only flags, buttons, and multiple video nodes
- PipeWire parser resolves Logitech endpoints without duplicating a physical device
- merged provider records preserve all controls and strongest device identity

### Regression tests

- G515 direct device plus six mirrored receiver slot responses collapses to one device
- two genuinely distinct paired devices of the same model remain two devices
- per-key RGB still commits a complete frame atomically
- v0.3.1 profile files load successfully after the v0.4 model changes

### Build gate

The Arch installer runs, in order:

1. `cargo test --workspace`
2. `cargo build --release --workspace`
3. package creation/installation only after both commands succeed

A failed test or compile leaves the currently installed ReForge version untouched.

## Compatibility and Migration

v0.4 reads existing v0.3.1 profiles. Newly added control metadata uses serde defaults where needed so older saved data remains valid.

The daemon socket path, binary names, systemd user service, udev access model, and desktop launcher remain compatible with the existing installation.

## Release Acceptance Criteria

v0.4 is ready to ship when all of the following are true:

- G515 LS TKL appears once in device discovery.
- G515 per-key RGB and brightness continue to work.
- Every writable V4L2 control reported for a Logitech camera is rendered and dispatches through typed validation.
- Logitech PipeWire endpoints expose working volume/mute controls.
- HID++ control adapters expose every implemented feature only when the feature is present on that device.
- Mice/keyboard performance, assignment, backlight/RGB, battery/power, and onboard-profile controls are capability-gated.
- HID++ audio controls are capability-gated and do not conflict with PipeWire endpoint controls.
- Profiles can persist and reapply every profile-eligible control.
- No arbitrary raw-report write API is exposed through RPC, CLI, or GUI.
- `cargo test --workspace` passes on the release source.
- `cargo build --release --workspace` passes on the release source.
