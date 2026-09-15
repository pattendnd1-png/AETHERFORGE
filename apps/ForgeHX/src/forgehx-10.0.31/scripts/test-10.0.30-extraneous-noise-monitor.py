#!/usr/bin/env python3
from pathlib import Path
import sys

root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path(__file__).resolve().parents[1]

def read(rel):
    path = root / rel
    return path.read_text() if path.exists() else ''

core = read('crates/forgehx-core/src/lib.rs')
dsp_lib = read('crates/forgehx-dsp/src/lib.rs')
engine = read('crates/forgehx-dsp/src/engine.rs')
monitor = read('crates/forgehx-dsp/src/noise_monitor.rs')
adaptive = read('crates/forgehx-dsp/src/adaptive_noise.rs')
runtime = read('crates/forgehx-dsp/src/runtime.rs')
direct = read('crates/forgehx-dsp/src/direct_pipewire.rs')
gui = read('crates/forgehx-gui/src/microphone.rs')
manager = read('crates/forgehx-dsp/src/lib.rs')
joined = '\n'.join([core, dsp_lib, engine, monitor, adaptive, runtime, direct, gui, manager])

checks = {
    'noise monitor module': (root/'crates/forgehx-dsp/src/noise_monitor.rs').exists() and 'pub mod noise_monitor;' in dsp_lib,
    'adaptive noise module': (root/'crates/forgehx-dsp/src/adaptive_noise.rs').exists() and 'pub mod adaptive_noise;' in dsp_lib,
    'all class scores': all(token in core for token in ['white_like','pink_like','broadband','hum_rumble','narrowband_whine','transient','speaker_leak']),
    'adaptation states': all(token in core for token in ['Learning','Holding','Suppressing','SpeechProtected','ReferenceUnavailable']),
    'single full-system speaker reference': 'get-default-sink' in runtime and '.monitor' in runtime and 'Command::new("parec")' in runtime,
    'monitor consumes render reference': 'render_history' in monitor and 'speaker_leak_score' in monitor,
    'broadband slope excludes sub-180hz hum probes': 'BROADBAND_SLOPE_START: usize = 4' in monitor and 'let values = &probe_db[BROADBAND_SLOPE_START' in monitor,
    'double-talk protection': 'double_talk' in monitor and 'SPEECH_HOLD_FRAMES' in monitor and '6.0' in adaptive,
    'per-band floors': 'NoiseBandLevels' in core and 'update_band_floors' in monitor,
    'bounded adaptive controller': all(token in adaptive for token in ['max_adaptive_suppression_db','background_margin_offset_db','residual_speaker_suppression_db','rate_limit']),
    'engine integration': all(token in engine for token in ['noise_monitor.analyze','adaptive_noise.update','process_adaptive','residual_speaker_suppression_db']),
    'runtime telemetry': 'noise_scene_telemetry' in runtime and 'noise_scene_telemetry' in direct,
    'state telemetry': 'pub noise_scene_telemetry: NoiseSceneTelemetry' in core,
    'gui monitor control': 'Extraneous Noise Monitor' in gui and 'Auto Noise Adapt' in gui,
    'gui speaker leak telemetry': 'Speaker leak' in gui or 'speaker leak' in gui,
    'fail closed silence': 'raw microphone bypass forbidden' in direct and '[0.0f32; FRAME_SAMPLES]' in direct,
    'no app-facing source publisher': 'MEDIA_CLASS => "Audio/Source"' not in direct,
}
for forbidden in ['wpctl set-default', 'pactl set-default-source', 'pactl set-default-sink']:
    checks['no '+forbidden] = forbidden not in joined

failed = [name for name, ok in checks.items() if not ok]
if failed:
    print('FORGEHX_10_0_30_EXTRANEOUS_NOISE_MONITOR=FAIL ' + ','.join(failed))
    raise SystemExit(1)
print('FORGEHX_10_0_30_EXTRANEOUS_NOISE_MONITOR=PASS')
