from pathlib import Path
import sys
core = Path('crates/forgehx-core/src/lib.rs').read_text()
required = [
    'pub struct EchoCancellationConfig',
    'pub struct ToneControlsConfig',
    'pub struct MultibandEqBand',
    'pub struct DynamicEqBand',
    'pub struct MultibandCompressorBand',
    'pub struct VoiceEnhancerConfig',
    'pub struct SaturationConfig',
    'pub enum VoicePilotMode',
    'pub enum VoicePilotTarget',
    'pub struct VoicePilotConfig',
    'pub struct VoicePilotTelemetry',
    'pub echo_cancellation: EchoCancellationConfig',
    'pub tone: ToneControlsConfig',
    'pub multiband_eq: Vec<MultibandEqBand>',
    'pub dynamic_eq: Vec<DynamicEqBand>',
    'pub multiband_compressor: Vec<MultibandCompressorBand>',
    'pub voice_enhancer: VoiceEnhancerConfig',
    'pub saturation: SaturationConfig',
    'pub voicepilot: VoicePilotConfig',
]
missing = [x for x in required if x not in core]
if missing:
    print('MISSING=' + ';'.join(missing))
    sys.exit(1)
for marker in ['bass_boost_db', 'treble_boost_db', 'adaptation_strength', 'auto_tone', 'auto_dynamics', 'BroadcastFull']:
    if marker not in core:
        print('MISSING_MARKER=' + marker)
        sys.exit(1)
print('ForgeHX 10.0.6 VoicePilot DSP contract invariants passed.')
