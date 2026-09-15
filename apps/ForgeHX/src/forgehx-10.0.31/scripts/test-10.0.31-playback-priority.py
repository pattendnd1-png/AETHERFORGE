#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
engine=(root/'crates/forgehx-dsp/src/engine.rs').read_text()
checks={
 'decision helper':'fn should_hard_reject_playback(' in engine,
 'false speech protection regression':'false_speech_protection_cannot_override_confirmed_playback' in engine,
 'confirmed user voice override regression':'confirmed_enrolled_voice_overrides_playback_hard_block' in engine,
 'weak playback keeps speech protection':'weak_playback_evidence_keeps_speech_protection' in engine,
 'engine uses helper':'should_hard_reject_playback(' in engine and 'let playback_rejected = should_hard_reject_playback(' in engine,
 'old unconditional speech veto removed':'let local_voice_present = confirmed_local_voice || observation.telemetry.speech_protected;' not in engine,
}
failed=[k for k,v in checks.items() if not v]
for k,v in checks.items(): print(f"FORGEHX_10_0_31_{k.upper().replace(' ','_')}={'PASS' if v else 'FAIL'}")
if failed: raise SystemExit('FORGEHX_10_0_31_PLAYBACK_PRIORITY=FAIL '+','.join(failed))
print('FORGEHX_10_0_31_PLAYBACK_PRIORITY=PASS')
