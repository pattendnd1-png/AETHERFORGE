# ForgeHX All-HyperX-Mice Full-Support Design

## Status

Approved architecture, written 2026-08-14.

## Goal

Make ForgeHX the native Linux control application for HyperX gaming mice across current and legacy product generations, using shared protocol-family drivers, exact hardware matching, and capability-by-capability fallback. No mouse may be labeled fully supported unless all capabilities claimed for that model have real working handlers.

## Scope

### Required current/legacy matrix

The initial release program covers the union of HyperX's current NGENUITY and NGENUITY Legacy mouse lists:

Current generation:

- Pulsefire Saga
- Pulsefire Saga Pro Wireless
- Pulsefire Haste 2 wired
- Pulsefire Haste 2 Wireless
- Pulsefire Haste 2 S Wireless
- Pulsefire Haste 2 Pro / 4K Wireless

Legacy/current-overlap generation:

- Pulsefire Surge
- Pulsefire Raid
- Pulsefire Core
- Pulsefire FPS Pro
- Pulsefire Dart
- Pulsefire Haste
- Pulsefire Haste Wireless
- Pulsefire Haste 2 / Haste 2 Wired
- Pulsefire Haste 2 Wireless
- Pulsefire Haste 2 Mini Wireless
- Pulsefire Haste 2 Core Wireless
- Pulsefire Haste 2 S Wireless
- Pulsefire Fuse Wireless
- Pulsefire Haste 2 Pro

Duplicate naming between current and Legacy is normalized to one logical model entry with aliases.

### Historical and future devices

The architecture must also accept additional HyperX/Kingston/HP mouse IDs discovered through verified protocol research. Devices outside the seeded matrix remain safely detected and receive a registry backlog entry, but never receive raw writes until a driver family is verified.

## Full-support capability target

Per model, implement every hardware capability the mouse exposes, including where applicable:

- DPI value
- multiple DPI stages
- active DPI stage
- independent X/Y DPI
- polling/report rate, including 2K/4K/8K models where hardware supports it
- lift-off distance
- debounce/sensor tuning
- motion/sensor options exposed by firmware
- button remapping
- keyboard shortcuts
- media actions
- DPI-cycle actions
- macros
- onboard profiles
- profile persistence/save-to-device
- RGB/aRGB/static/effects/zones/brightness
- battery percentage/state
- charging state
- wireless receiver state
- 2.4 GHz/Bluetooth mode status
- receiver pairing/status where safely supported
- Instant Pair visibility/status when Linux exposes the relevant transport
- firmware/hardware revision readback
- diagnostics

Firmware flashing remains excluded.

## Core design rule

**Protocol family first, model descriptor second.**

A model descriptor contains identity, physical capabilities and quirks. A protocol-family implementation contains packet encoding/decoding and transport rules shared by related mice. This prevents copy-pasting a full driver for every Haste variant.

## Architecture

### `forgehx-device`

Keep the static driver registry but evolve it into model descriptors plus protocol-family IDs.

Proposed shape:

```rust
pub struct MouseModelDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub vendor_id: u16,
    pub product_ids: &'static [u16],
    pub protocol_family: MouseProtocolFamily,
    pub capabilities: &'static [Capability],
    pub complete: bool,
}
```

`complete` only becomes true when the model's declared capability matrix has working handlers and tests.

### New `forgehx-mouse` crate

Move native mouse protocol implementations out of `forgehx-device` so detection and control are separated.

Suggested layout:

```text
crates/forgehx-mouse/
  src/lib.rs
  src/registry.rs
  src/transport.rs
  src/state.rs
  src/protocol/
    mod.rs
    haste_v1.rs
    dart.rs
    legacy_rgb.rs
    haste2.rs
    haste2_wireless.rs
    saga.rs
    fuse.rs
```

Exact module boundaries follow protocol evidence; two named families merge if packet formats prove identical.

The existing Pulsefire Haste Wireless implementation becomes the seed for `haste_v1` rather than remaining a one-off driver.

## Protocol-family program

Initial families and targets:

### Haste v1

Target:

- Pulsefire Haste
- Pulsefire Haste Wireless

Known starting point:

- 64-byte control packets
- verified native DPI and polling implementation already exists for Haste Wireless
- extend to state readback, button assignments, macros, lighting, lift-off and profile persistence where protocol evidence supports them

### Legacy Pulsefire RGB/control

Target candidates:

- Pulsefire Surge
- Pulsefire Core
- Pulsefire FPS Pro
- Pulsefire Raid

OpenRGB has existing HyperX controllers for several of these devices. ForgeHX may use OpenRGB as an interim lighting fallback, while native input/performance control is developed independently from verified public protocol behavior. License boundaries must be respected; do not copy GPL implementation code into an incompatible ForgeHX license.

### Dart family

Target:

- Pulsefire Dart wired/wireless transports

Treat wired and wireless receiver IDs as separate transport identities under one logical model family.

### Haste 2 family

Target:

- Haste 2 wired
- Haste 2 Wireless
- Haste 2 Mini Wireless
- Haste 2 Core Wireless
- Haste 2 S Wireless
- Haste 2 Pro / 4K Wireless

Variant descriptors identify differences such as maximum polling rate, battery, Bluetooth, receiver behavior, sensor controls and lighting.

### Saga family

Target:

- Pulsefire Saga
- Pulsefire Saga Pro Wireless

The architecture must represent modular/extra button layouts without assuming six fixed buttons. Button topology comes from the model descriptor/native state.

### Fuse family

Target:

- Pulsefire Fuse Wireless

Keep separate until protocol evidence proves it is a Haste-family derivative.

## Capability routing

Priority stays:

1. ForgeHX Native
2. Linux Standard where appropriate
3. libratbag fallback for capabilities the installed ratbag daemon actually exposes for the device
4. OpenRGB fallback for lighting
5. Diagnostic read-only

Rules:

- A native driver owns only capabilities with implemented handlers.
- A partially native mouse may use ForgeHX Native for DPI and OpenRGB for lighting simultaneously.
- Ratbag is probed at runtime; ForgeHX never assumes a model is supported merely because a future/upstream driver exists.
- One capability has one selected owner.
- Native promotion must not break a working fallback capability.

## Core types

Add typed mouse state rather than overloading generic strings.

Examples:

```rust
pub struct MouseLimits {
    pub dpi_min: u32,
    pub dpi_max: u32,
    pub dpi_step: u32,
    pub max_dpi_stages: u8,
    pub polling_rates_hz: Vec<u32>,
    pub button_count: u16,
}

pub struct WirelessMouseState {
    pub battery_percent: Option<u8>,
    pub charging: Option<bool>,
    pub transport: Option<WirelessTransport>,
    pub receiver_connected: Option<bool>,
}

pub struct MouseHardwareState {
    pub dpi: DpiConfig,
    pub polling_rate_hz: Option<u32>,
    pub active_profile: Option<u8>,
    pub button_assignments: Vec<ButtonAssignment>,
    pub wireless: Option<WirelessMouseState>,
}
```

## Daemon and IPC

The daemon remains sole write authority.

Required command surface grows to include:

```text
MouseCapabilities
MouseState
MouseSetDpi
MouseSetPollingRate
MouseSetProfile
MouseSetButtonAssignment
MouseSetMacro
MouseSetLiftOffDistance
MouseSetSensorOption
MouseLightingMetadata
MouseSetLighting
MouseWirelessState
MouseSaveOnboard
```

Unsupported commands return a typed unsupported-capability error and never fall through to raw HID.

## GUI

Mouse tabs:

- **Overview**: model, transport, firmware, backend ownership.
- **Performance**: polling, sensor/lift-off/debounce options.
- **DPI**: stages, active stage, X/Y linking where supported.
- **Buttons**: visual/logical button list and assignments.
- **Macros**: create/edit/assign macros where firmware supports them.
- **Lighting**: per-device modes/zones/effects.
- **Battery/Wireless**: charge, receiver, transport and pairing status.
- **Profiles**: onboard/software profiles and apply status.
- **Doctor**: exact interfaces, protocol family and backend diagnostics.

The UI derives ranges and button counts from `MouseLimits`; it must not hard-code five stages or six buttons globally.

## Profiles

Profiles remain backend-neutral. A profile stores desired logical settings, then the daemon resolves the best provider at apply time.

Profile application is per capability and returns a partial result. An unavailable lighting backend must not stop DPI/profile/button settings from applying.

## Device grouping

Wireless products often expose receiver, HID control, mouse input, keyboard/media and charging/wired identities. ForgeHX groups them by:

1. serial/unique receiver identity when available
2. exact USB parent/topology
3. verified model-specific receiver relationship
4. deterministic path fallback

Do not merge unrelated HP/HyperX devices solely on VID.

## Permissions

Install exact udev rules for verified HyperX mouse VID/PID/interface combinations. Avoid manufacturer-string assumptions because HP-era HyperX hardware may report `HP, Inc` rather than `HyperX`.

Rules must grant only the minimum hidraw/USB access necessary for the control interface.

## Safety

- no firmware flashing
- no unknown vendor writes
- packet length validation on every transport
- exact model/protocol matching before native write
- receiver/pairing writes disabled unless protocol is verified and recovery is understood
- if hardware readback disagrees with requested state, report failure instead of pretending success

## Testing

Required:

- every seeded model/alias has a registry test
- exact VID/PID/interface matching tests
- protocol packet golden tests
- malformed-packet rejection tests
- ownership tests for mixed native/OpenRGB/ratbag capability sets
- no-write tests for unknown HyperX IDs
- button-count and DPI-range tests per model descriptor
- wireless grouping tests
- profile partial-application tests
- udev exact-match guards
- GUI dynamic-range/button-topology tests

Hardware tests run only when matching hardware is present.

## Delivery sequence

Do not attempt every native family in one unreviewable patch. Deliver incremental packages while keeping the overall spec as the acceptance target:

1. Generalize existing Haste Wireless into `forgehx-mouse` and finish Haste v1.
2. Add legacy Pulsefire families (Surge/Core/FPS Pro/Raid/Dart).
3. Add Haste 2 variants, including high polling-rate/wireless state differences.
4. Add Fuse Wireless.
5. Add Saga/Saga Pro modular topology.
6. Run matrix audit and close remaining capability gaps.

Each package must improve real control without falsely promoting unfinished capabilities.

## Acceptance criteria

The all-mice program is successful when:

1. Every model in the required current/legacy matrix is recognized as its proper HyperX model/family.
2. No seeded model remains `Diagnostic Only` merely because one optional capability is missing.
3. Every capability advertised for a model has a functioning owner and handler.
4. A model is `Fully Supported` only when its declared full capability matrix is implemented and verified.
5. Mixed ownership works: native control can coexist with OpenRGB or ratbag fallback without conflicts.
6. Wired/wireless identities group correctly and retain transport-specific state.
7. Unknown HyperX mouse IDs remain write-protected.
8. No firmware flashing is exposed.

## Reference basis

The seeded model matrix is based on HyperX's current NGENUITY and NGENUITY Legacy compatibility lists as checked on 2026-08-14. Protocol-family work may use public protocol documentation, Linux kernel/HID descriptors, device captures supplied by the user, OpenRGB as a compatibility backend/reference boundary, and libratbag behavior where the installed version actually supports a model.
