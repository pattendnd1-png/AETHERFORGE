# ForgeHX Pulsefire Haste Wireless Support Design

## Goal

Promote the HyperX Pulsefire Haste Wireless (`03f0:028e`, wired transport `03f0:048e`) out of Diagnostic Only only when ForgeHX has a real control path. Do not fake support by changing labels or capability flags without working handlers.

## Device identity

- Vendor: HP/HyperX `0x03f0`.
- Wireless product ID: `0x028e`.
- Wired product ID used by the same Haste wireless family: `0x048e`.
- Configuration transport: HID interface 2, vendor usage page `0xff00`, 64-byte reports when that interface is available.
- Product string can identify HyperX even when manufacturer is `HP, Inc`.

## Control strategy

ForgeHX uses a hybrid capability model.

1. A verified native Pulsefire Haste Wireless driver owns only capabilities whose native read/write implementation exists.
2. ratbagd/libratbag remains eligible for DPI, polling rate, profiles, and button bindings when it exposes this mouse.
3. OpenRGB remains eligible for lighting only when it deterministically exposes the exact physical mouse.
4. Diagnostic remains the fallback for unsupported capabilities.
5. Native ownership must never shadow a working fallback with an unimplemented native handler.

The first native slice implements Haste-family identification plus direct polling-rate and DPI packet generation/write support. Lighting and bindings stay on compatibility backends until their native handlers are explicitly implemented and verified.

## Driver boundary

`forgehx-device` owns the device-specific native transport. It exposes a small `PulsefireHasteWireless` controller that:

- locates the configuration HID interface by VID/PID plus interface/usage metadata;
- opens only the matched hidraw path;
- validates DPI and polling-rate values before generating packets;
- sends exactly 64-byte reports;
- never performs firmware operations;
- exposes pure packet builders for unit testing.

`forgehx-daemon` calls that controller only after capability ownership selected `ForgeHxNative` for this registered driver.

## Capabilities and support level

The registered native driver advertises:

- `Diagnostics`
- `Dpi`
- `PollingRate`

If ratbag or OpenRGB adds additional working capabilities, the mouse is `PartiallySupported`. If only native DPI/polling plus diagnostics are available, it is still `PartiallySupported`, never Diagnostic Only.

The driver ID is `hyperx-pulsefire-haste-wireless-v1`.

## Permissions

The udev rules explicitly grant `uaccess` to hidraw nodes for:

- `03f0:028e`
- `03f0:048e`

This avoids depending on an HP manufacturer string matching `HyperX*`.

## Safety

- No firmware flashing.
- No undocumented generic writes to arbitrary HP devices.
- Exact VID/PID and interface matching is required.
- The daemon remains the authorization boundary.
- Packet builders reject unsupported DPI/rate values before any HID write.

## Verification

Tests cover exact device registration, correct support-level promotion, interface selection, polling-rate packet generation, DPI packet generation, and udev rule presence. Arch-side `cargo test --workspace` and package build remain the authoritative compile/runtime checks when Cargo is unavailable in the build sandbox.
