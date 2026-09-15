from pathlib import Path
import sys
engine = Path('crates/forgehx-dsp/src/engine.rs')
vp = Path('crates/forgehx-dsp/src/voicepilot.rs')
cargo = Path('crates/forgehx-dsp/Cargo.toml').read_text()
root = Path('Cargo.toml').read_text()
if not engine.exists() or not vp.exists():
    print('MISSING_ENGINE_MODULES')
    sys.exit(1)
e = engine.read_text(); v = vp.read_text()
required_engine = [
    'pub const FRAME_SAMPLES: usize = 480',
    'pub struct VoiceProcessingEngine',
    'AudioProcessing::builder()',
    'process_render_f32',
    'process_capture_f32',
    'NoiseSuppressionLevel',
    'noise_level_from_percent',
    'NOISE_RECONFIGURE_FRAMES',
    'set_capture_pre_gain',
    'cleanup_signature',
    'pub fn process_10ms',
    'apply_bass_treble',
    'apply_parametric_eq',
    'apply_multiband_eq',
    'apply_multiband_compressor',
    'apply_de_esser',
    'apply_compressor',
    'apply_saturation',
    'apply_limiter',
]
required_vp = [
    'pub struct VoicePilotAnalyzer',
    'pub struct AdaptiveTargets',
    'bass_boost_db',
    'treble_boost_db',
    'presence_db',
    'noise_strength_percent',
    'de_esser_reduction_db',
    'compressor_reduction_db',
    '.clamp(',
]
missing=[x for x in required_engine if x not in e]+[x for x in required_vp if x not in v]
if missing:
    print('MISSING='+';'.join(missing)); sys.exit(1)
if 'sonora = "0.2"' not in root or 'sonora.workspace = true' not in cargo:
    print('SONORA_DEPENDENCY_MISSING'); sys.exit(1)
if 'JamesDSP' in e or 'EasyEffects' in e:
    print('FORBIDDEN_PLAYBACK_DSP'); sys.exit(1)
print('ForgeHX 10.0.6 realtime VoicePilot DSP invariants passed.')
