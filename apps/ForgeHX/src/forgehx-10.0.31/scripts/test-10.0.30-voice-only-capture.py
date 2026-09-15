from pathlib import Path
import sys
root = Path(__file__).resolve().parents[1]

def read(p): return (root/p).read_text()
checks = {
    'reference aligner module': (root/'crates/forgehx-dsp/src/voice_only.rs').exists(),
    'voice only module exported': 'pub mod voice_only;' in read('crates/forgehx-dsp/src/lib.rs'),
    'two second reference history': 'MAX_REFERENCE_DELAY_FRAMES: usize = 200' in read('crates/forgehx-dsp/src/voice_only.rs') if (root/'crates/forgehx-dsp/src/voice_only.rs').exists() else False,
    'delay alignment test': 'finds_nine_hundred_millisecond_playback_delay' in read('crates/forgehx-dsp/src/voice_only.rs') if (root/'crates/forgehx-dsp/src/voice_only.rs').exists() else False,
    'double talk preservation test': 'subtraction_preserves_uncorrelated_local_voice' in read('crates/forgehx-dsp/src/voice_only.rs') if (root/'crates/forgehx-dsp/src/voice_only.rs').exists() else False,
    'playback only rejection test': 'playback_only_is_hard_rejected_after_alignment' in read('crates/forgehx-dsp/src/engine.rs'),
    'no six db cap': '.min(6.0)' not in read('crates/forgehx-dsp/src/adaptive_noise.rs'),
    'engine uses aligned render': 'aligned_render' in read('crates/forgehx-dsp/src/engine.rs'),
    'default playback history 2000ms': 'history_ms: 2000' in read('crates/forgehx-core/src/lib.rs'),
}
failed=[k for k,v in checks.items() if not v]
for k,v in checks.items(): print(f'FORGEHX_VOICE_ONLY_{k.upper().replace(" ","_")}={"PASS" if v else "FAIL"}')
if failed:
    print('FORGEHX_10_0_30_VOICE_ONLY_CAPTURE=FAIL:' + ','.join(failed))
    sys.exit(1)
print('FORGEHX_10_0_30_VOICE_ONLY_CAPTURE=PASS')
