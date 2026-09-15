#!/usr/bin/env python3
from pathlib import Path

root = Path(__file__).resolve().parents[1]
runtime = (root / 'crates/forgehx-dsp/src/runtime.rs').read_text()
dsp_lib = (root / 'crates/forgehx-dsp/src/lib.rs').read_text()
direct_path = root / 'crates/forgehx-dsp/src/direct_pipewire.rs'
workspace = (root / 'Cargo.toml').read_text()
dsp_cargo = (root / 'crates/forgehx-dsp/Cargo.toml').read_text()
pkgbuild = (root / 'PKGBUILD').read_text()

errors = []

def require(condition: bool, message: str) -> None:
    if not condition:
        errors.append(message)

require('pw-loopback' not in runtime, 'primary DSP runtime still invokes pw-loopback')
require('spawn_bridge' not in runtime, 'old processed-mic bridge launcher remains')
require('processed_injection_name' not in dsp_lib, 'processed injection sink API remains')
require('render_runtime_bridge' not in dsp_lib, 'rendered loopback bridge config remains')
require(direct_path.exists(), 'direct_pipewire runtime module is missing')
require('pipewire = { version = "0.10.1", features = ["v0_3_41"] }' in workspace, 'workspace does not pin pipewire-rs 0.10.1 with safe trigger API feature')
require('pipewire.workspace = true' in dsp_cargo, 'forgehx-dsp does not consume workspace pipewire dependency')
require('tracing.workspace = true' in dsp_cargo, 'forgehx-dsp does not expose direct-path runtime evidence through tracing')
require("'libpipewire'" in pkgbuild, 'Arch package does not explicitly depend on libpipewire for the direct Rust runtime')

if direct_path.exists():
    direct = direct_path.read_text()
    require('wait_for_source_node_id(raw_source)' in direct and 'Some(raw_source_node_id)' in direct, 'physical capture does not resolve and connect directly to raw PipeWire node ID')
    require('_direct_capture' in direct, 'direct physical capture stream lacks a stable diagnostic node name')
    require('ForgeHX Direct HyperX Capture' in direct, 'direct capture stream lacks a clear diagnostic description')
    require('ForgeHX direct mic path connected' in direct, 'direct runtime lacks host-verifiable connection evidence')
    require('Audio/Source' in direct, 'app-facing stream is not classified Audio/Source')
    require('ForgeHX Mic' in direct, 'app-facing source description is not ForgeHX Mic')
    require('AudioFormat::F32LE' in direct, 'direct path is not F32LE')
    require('set_rate(48_000)' in direct or 'set_rate(48000)' in direct, 'direct path is not 48 kHz')
    require('set_channels(1)' in direct, 'direct path is not mono')
    require('FRAME_SAMPLES' in direct, 'direct path does not preserve 10 ms DSP framing')
    require('MAX_QUEUED_FRAMES' in direct, 'direct output queue is not explicitly bounded')
    require('Direction::Input' in direct, 'direct hardware capture input stream is missing')
    require('Direction::Output' in direct, 'ForgeHX source output stream is missing')
    require('AUTOCONNECT' in direct, 'direct capture stream is not explicitly autoconnected to its physical target')
    require('StreamFlags::TRIGGER' in direct, 'app-facing source is not trigger-driven by completed DSP frames')
    require('.trigger_process()' in direct, 'direct runtime does not use safe PipeWire stream trigger API')
    require('pw::sys::pw_stream_trigger_process' not in direct, 'direct runtime still reaches through raw PipeWire sys trigger API')
    require('from_bits_retain(1 << 9)' not in direct, 'direct runtime hardcodes PW_STREAM_FLAG_TRIGGER bit')
    require('.ok_or_else(' in direct, 'PipeWire Pod::from_bytes Option is not handled with Option semantics')

require((root / 'scripts/test-10.0.11-virtual-source-node-resolution.py').exists(), '10.0.11 routing regression was removed')
require((root / 'scripts/test-10.0.10-stable-mic-identity.py').exists(), 'stable mic identity regression was removed')

if errors:
    for error in errors:
        print(f'FAIL: {error}')
    raise SystemExit(1)
print('PASS: ForgeHX 10.0.12 direct hardware microphone path contract')
