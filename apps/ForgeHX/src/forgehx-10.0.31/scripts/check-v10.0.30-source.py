#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
read=lambda p:(root/p).read_text()
cargo=read('Cargo.toml'); pkg=read('PKGBUILD'); core=read('crates/forgehx-core/src/lib.rs')
daemon=read('crates/forgehx-daemon/src/lib.rs'); engine=read('crates/forgehx-dsp/src/engine.rs')
monitor=read('crates/forgehx-dsp/src/noise_monitor.rs'); adaptive=read('crates/forgehx-dsp/src/adaptive_noise.rs')
runtime=read('crates/forgehx-dsp/src/runtime.rs'); direct=read('crates/forgehx-dsp/src/direct_pipewire.rs')
gui=read('crates/forgehx-gui/src/microphone.rs'); manager=read('crates/forgehx-dsp/src/lib.rs'); voice_only=read('crates/forgehx-dsp/src/voice_only.rs')
checks={
 'version':'version = "10.0.30"' in cargo and 'pkgver=10.0.30' in pkg,
 'monitor config':'ExtraneousNoiseMonitorConfig' in core and 'max_adaptive_suppression_db' in core,
 'telemetry schema':'NoiseSceneTelemetry' in core and 'noise_scene_telemetry' in core,
 'noise monitor':'goertzel_power' in monitor and 'update_band_floors' in monitor,
 'speaker correlation':'normalized_correlation' in monitor and 'double_talk' in monitor,
 'adaptive policy':'AdaptiveNoiseController' in adaptive and 'VoicePilotMode::Locked' in adaptive,
 'engine integration':'noise_monitor.analyze' in engine and 'adaptive_noise.update' in engine,
 'background rejector adaptive':'process_adaptive' in read('crates/forgehx-dsp/src/background_rejection.rs'),
 'runtime telemetry':'noise_scene_telemetry' in runtime and 'noise_scene_telemetry' in direct,
 'speaker monitor capture':'get-default-sink' in runtime and '.monitor' in runtime and 'Command::new("parec")' in runtime,
 'voice only delay aligner':'MAX_REFERENCE_DELAY_FRAMES: usize = 200' in voice_only and 'aligned_reference' in voice_only,
 'voice only subtraction':'subtract_correlated_playback' in voice_only and 'playback_dominant' in voice_only,
 'engine aligned render':'aligned_render' in engine and 'voice_only_subtraction' in engine,
 'no six db double talk cap':'.min(6.0)' not in adaptive,
 'post subtraction local speech':'last_local_probe_db' in monitor and 'speaker_leak < 0.35' in monitor,
 'gui':'Noise Environment' in gui and 'Extraneous Noise Monitor' in gui and 'DOUBLE-TALK' in gui,
 'solocast lock':'HyperX SoloCast 2 Analog Stereo' in core,
 'aetherstream source':'aetherstream.system.microphone' in manager,
 'raw fail closed':'raw microphone bypass forbidden' in direct and '[0.0f32; FRAME_SAMPLES]' in direct,
 'correct verifier parser':'test-10.0.30-runtime-state.py' in read('scripts/verify-10.0.30.sh') if (root/'scripts/verify-10.0.30.sh').exists() else False,
}
joined='\n'.join([daemon,engine,monitor,adaptive,runtime,direct,gui,manager,voice_only])
for forbidden in ['wpctl set-default','pactl set-default-source','pactl set-default-sink']:
    checks['no '+forbidden]=forbidden not in joined
failed=[k for k,v in checks.items() if not v]
if failed: raise SystemExit('FORGEHX_10_0_30_SOURCE=FAIL '+','.join(failed))
print('FORGEHX_10_0_30_SOURCE=PASS')
