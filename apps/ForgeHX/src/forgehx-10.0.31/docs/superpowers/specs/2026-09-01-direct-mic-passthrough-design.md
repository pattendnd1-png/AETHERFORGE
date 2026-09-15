# ForgeHX 10.0.12 Direct Microphone Pass-Through Design

## Goal

Replace the ForgeHX 10.0.11 `pw-loopback`/hidden-injection-sink microphone graph with an application-owned PipeWire capture/output pair:

`physical HyperX source -> ForgeHX capture stream -> in-process DSP -> ForgeHX Mic Audio/Source -> Discord/OBS/games/browser/etc.`

## Required behavior

- ForgeHX captures directly from the selected physical HyperX PipeWire source by stable `node.name`.
- ForgeHX DSP remains in-process at 48 kHz mono, processing exact 10 ms / 480-sample frames.
- ForgeHX publishes exactly one stable app-facing source: `ForgeHX Mic` (`media.class=Audio/Source`).
- No `pw-loopback` process, no hidden injection sink, no sink-monitor-source bridge, and no PipeWire configuration drop-in are used for the primary microphone path.
- The app-facing source identity remains stable across physical mic reconnects.
- ForgeHX Mic remains the native PipeWire/WirePlumber and PipeWire-Pulse default microphone.
- Applications that explicitly choose another input are not forcibly rewritten.
- The raw HyperX source may remain discoverable as an emergency/manual bypass, but ForgeHX never routes normal communication clients to it.
- Wired sidetone/monitoring and the existing AEC playback reference remain auxiliary consumers and must not become part of the primary communication path.
- The runtime reconnects the physical capture stream without restarting PipeWire or WirePlumber.
- The previous 10.0.10 persistent identity, 10.0.11 default routing, Haste, firmware safety, GUI, and DSP regression suites remain enforced.
- Arch release build and tests run with `-D warnings`.

## Implementation

Use `pipewire` Rust bindings 0.10.1. A dedicated `direct_pipewire` module owns one PipeWire main loop with two streams: an Input stream targeted directly at the physical HyperX source and an Output stream classified as `Audio/Source` with the stable ForgeHX node name. Capture callbacks collect F32LE mono samples into 480-sample frames, run the existing `VoiceProcessingEngine`, and enqueue processed samples for the source callback. The processed queue is bounded so a disconnected/slow consumer cannot accumulate unbounded latency.

`runtime.rs` retains lifecycle/config/telemetry/control ownership and auxiliary monitoring/AEC behavior, but it no longer creates or writes to an injection sink. The direct PipeWire thread is the only producer of the ForgeHX Mic source.

## Failure/reconnect behavior

If the physical source disappears, the direct capture stream is allowed to reconnect through PipeWire using the stable target name while the ForgeHX source identity remains owned by the ForgeHX runtime. If the PipeWire runtime exits unexpectedly, the daemon refresh loop starts it again. DSP errors fall back to the raw frame for that frame, preserving audio continuity as in 10.0.11.

## Verification

A new 10.0.12 contract test must fail on 10.0.11 and assert:

- `pw-loopback` is absent from ForgeHX DSP runtime code.
- no processed injection sink is rendered or required.
- `pipewire = "0.10.1"` is pinned in the workspace.
- direct capture targets the raw physical `node.name`.
- the published stream is `Audio/Source`, 48 kHz, mono, F32LE, and has the stable ForgeHX node name.
- the 10.0.11 default-source routing and stable-identity tests remain present.
- the host verification script reports `DIRECT_HARDWARE_CAPTURE=PASS`, `APP_OWNED_MIC_SOURCE=PASS`, `NO_VIRTUAL_LOOPBACK_CHAIN=PASS`, and final `FORGEHX_VERIFY=PASS`.
