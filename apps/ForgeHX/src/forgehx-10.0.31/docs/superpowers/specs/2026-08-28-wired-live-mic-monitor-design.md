# ForgeHX Wired Live Mic Monitor Design

## Goal
Add a live monitor for the exact post-DSP ForgeHX Processed Mic signal and route it only to a wired ALSA/analog/headphone playback sink, never to a Bluetooth/BlueZ sink.

## Scope
- ForgeHX 10.0.5 only.
- Monitor is a tee from `ForgeHX Processed Mic`; OBS/stream routing is not changed.
- GUI exposes Live Headphone Monitor on/off and monitor level.
- Daemon chooses a currently available wired sink; Bluetooth sinks are always rejected.
- Monitoring is runtime-only and defaults off after daemon restart.
- Existing permanent DSP, voice learning, AEC, click suppression, tray background behavior, and mouse support remain unchanged.

## Routing rules
A sink is monitor-eligible only when it is a PipeWire sink and its node name is not BlueZ/Bluetooth. Prefer, in order: names containing `headphone`, names containing `analog`, then other `alsa_output` nodes. Never fall back to `bluez_output`, `bluez`, or names containing `bluetooth`.

## Runtime architecture
`DspRuntimeRegistry` owns a per-device monitor configuration and a monitor worker. When enabled, the monitor worker records from the device-specific `ForgeHX Processed Mic` source and plays that stream to the selected wired sink using `pw-cat`. The main DSP worker is untouched, so monitoring cannot alter the stream signal.

## Safety/error behavior
If no eligible wired sink exists, enabling monitor returns `audio_unavailable`; Bluetooth is never selected as fallback. If the wired sink disappears, the monitor worker reconnects when an eligible target is configured again. Disabling monitor tears down only the monitor worker.
