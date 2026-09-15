#!/usr/bin/env python3
from pathlib import Path
import re
root=Path(__file__).resolve().parents[1]
read=lambda p:(root/p).read_text()
cargo=read('Cargo.toml'); pkg=read('PKGBUILD'); core=read('crates/forgehx-core/src/lib.rs')
daemon=read('crates/forgehx-daemon/src/lib.rs'); engine=read('crates/forgehx-dsp/src/engine.rs')
monitor=read('crates/forgehx-dsp/src/noise_monitor.rs'); adaptive=read('crates/forgehx-dsp/src/adaptive_noise.rs')
runtime=read('crates/forgehx-dsp/src/runtime.rs'); direct=read('crates/forgehx-dsp/src/direct_pipewire.rs')
gui=read('crates/forgehx-gui/src/microphone.rs'); manager=read('crates/forgehx-dsp/src/lib.rs'); voice_only=read('crates/forgehx-dsp/src/voice_only.rs')
checks={
 'version':'version = "10.0.31"' in cargo and 'pkgver=10.0.31' in pkg,
 'playback priority helper':'fn should_hard_reject_playback(' in engine,
 'false speech protection regression':'false_speech_protection_cannot_override_confirmed_playback' in engine,
 'confirmed voice regression':'confirmed_enrolled_voice_overrides_playback_hard_block' in engine,
 'weak evidence regression':'weak_playback_evidence_keeps_speech_protection' in engine,
 'old unconditional speech veto removed':'let local_voice_present = confirmed_local_voice || observation.telemetry.speech_protected;' not in engine,
 'monitor config':'ExtraneousNoiseMonitorConfig' in core and 'max_adaptive_suppression_db' in core,
 'telemetry schema':'NoiseSceneTelemetry' in core and 'noise_scene_telemetry' in core,
 'noise monitor':'goertzel_power' in monitor and 'update_band_floors' in monitor,
 'speaker correlation':'normalized_correlation' in monitor and 'double_talk' in monitor,
 'adaptive policy':'AdaptiveNoiseController' in adaptive and 'VoicePilotMode::Locked' in adaptive,
 'engine integration':bool(re.search(r'noise_monitor\s*\.\s*analyze\s*\(',engine,re.S)) and bool(re.search(r'adaptive_noise\s*\.\s*update\s*\(',engine,re.S)),
 'voice only delay aligner':'MAX_REFERENCE_DELAY_FRAMES: usize = 200' in voice_only and 'aligned_reference' in voice_only,
 'voice only subtraction':'subtract_correlated_playback' in voice_only and 'playback_dominant' in voice_only,
 'engine aligned render':'aligned_render' in engine and 'voice_only_subtraction' in engine,
 'effective 2000ms guard':'config.history_ms.max(2000)' in read('crates/forgehx-dsp/src/speaker_lock.rs'),
 'speaker monitor capture':'get-default-sink' in runtime and '.monitor' in runtime and 'Command::new("parec")' in runtime,
 'solocast lock':'HyperX SoloCast 2 Analog Stereo' in core,
 'aetherstream source':'aetherstream.system.microphone' in manager,
 'bridge heartbeat verifier':'forgehx-mic.active' in read('scripts/verify-10.0.31.sh'),
 'raw fail closed':'raw microphone bypass forbidden' in direct and '[0.0f32; FRAME_SAMPLES]' in direct,
}
joined='\n'.join([daemon,engine,monitor,adaptive,runtime,direct,gui,manager,voice_only])
for forbidden in ['wpctl set-default','pactl set-default-source','pactl set-default-sink']:
    checks['no '+forbidden]=forbidden not in joined
failed=[k for k,v in checks.items() if not v]
if failed: raise SystemExit('FORGEHX_10_0_31_SOURCE=FAIL '+','.join(failed))
print('FORGEHX_10_0_31_SOURCE=PASS')
