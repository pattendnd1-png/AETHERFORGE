# ForgeHX 10.0.10 target hardware

## HyperX Pulsefire Haste Wireless

ForgeHX 10.0.10 has a native Rust driver for the first-generation HyperX Pulsefire Haste Wireless family.

Verified transport identities:

- `03f0:028e` — 2.4 GHz wireless receiver transport
- `03f0:048e` — wired USB transport of the same Haste Wireless family
- configuration HID interface: interface `02`, vendor usage page `0xff00`, 64-byte reports

Native ForgeHX capabilities:

- current DPI stage/readback
- up to five DPI stages, 200–16000 DPI in 100-DPI increments
- 125/250/500/1000 Hz polling rate
- 1 mm / 2 mm lift-off distance
- six button assignments
- bounded keyboard/mouse macro assignments
- RGB static/breathing/spectrum controls
- onboard settings save
- connection-state readback
- battery percentage and charging/wired state telemetry

All writes are model/VID/PID/interface gated. Future or name-only variants do not inherit this native write path.

## HyperX SoloCast 2

Verified identity:

- `03f0:0fbf`
- HID control interface `02`
- exact HP/HyperX udev permission is installed because the device reports manufacturer `HP, Inc.`

ForgeHX owns the complete safe Linux control path for the SoloCast 2:

- PipeWire capture volume and mute
- persistent input routing
- permanent ForgeHX Processed Mic graph
- live user-authoritative DSP
- acoustic echo cancellation
- noise reduction, gate, dynamics, EQ and limiter
- voice isolation / anti-loop
- keyboard and mouse click suppression
- automatic DSP reattachment across microphone, speaker/Bluetooth, daemon and PipeWire reconnects
- firmware identity/inventory for exact `03f0:0fbf`

The SoloCast 2 exposes a vendor HID control interface, but its vendor report meanings are not documented by the evidence qualified for this release. ForgeHX 10.0.10 therefore does not send guessed vendor packets for onboard filters or LEDs. Those controls remain write-protected until packet meanings are captured and verified against this exact device.

## Full-support classification contract

ForgeHX support level describes whether ForgeHX has a verified owner for every capability in the model's application support contract. It is not synonymous with raw-vendor-HID ownership.

- Pulsefire Haste Wireless is a complete native device: its verified ForgeHX native driver owns every required mouse capability. Compatibility backends such as OpenRGB or libratbag may be present, but they can no longer downgrade this complete native model to Partial Support.
- SoloCast 2 is a complete verified composite device: Linux Standard owns UAC/PipeWire capture, gain and mute; ForgeHX DSP owns the persistent processed-microphone chain; ForgeHX Native owns exact firmware identity/inventory. When all required owners are present and verified, the device is Fully Supported by ForgeHX.
- SoloCast 2 vendor-HID writes remain independently write-protected. Full ForgeHX support does not authorize undocumented onboard-filter, LED, bootloader, or firmware-write packets.

## 10.0.10 Haste HID permission repair

For `03f0:028e` and `03f0:048e`, ForgeHX grants deterministic read/write access only to USB HID interface `02`, the verified 64-byte vendor control interface. Interfaces `00`, `01`, and `03` are not granted this direct ForgeHX permission. The package post-install/post-upgrade hook reloads udev and repairs the currently connected interface-02 node immediately.
