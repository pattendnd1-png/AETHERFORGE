#!/usr/bin/env python3
from pathlib import Path
root = Path(__file__).resolve().parents[1]
engine = (root/'crates/forgehx-dsp/src/engine.rs').read_text()
mod = root/'crates/forgehx-dsp/src/background_rejection.rs'
core = (root/'crates/forgehx-core/src/lib.rs').read_text()
checks = {
    'stateful rejector module': mod.exists(),
    'engine imports rejector': 'use crate::background_rejection::BackgroundRejector;' in engine,
    'engine owns rejector': 'background_rejector: BackgroundRejector' in engine,
    'rejector runs after sonora cleanup': 'self.background_rejector.process(&mut cleaned, &self.config.noise_suppression);' in engine,
    'existing VAD control preserved': 'pub vad_threshold_percent: f32' in core,
    'existing grace control preserved': 'pub grace_ms: u32' in core,
}
if mod.exists():
    text = mod.read_text()
    checks.update({
        'adaptive noise floor': 'noise_floor_db' in text and 'update_noise_floor' in text,
        'strength controls margin': 'strength_percent' in text and 'decision_margin_db' in text,
        'vad controls decision': 'vad_threshold_percent' in text,
        'speech hangover uses grace': 'grace_ms' in text and 'hangover_frames' in text,
        'aggressive rejection gain': 'rejection_gain_db' in text and '-60.0' in text,
        'speech safety ceiling': 'VOICE_OPEN_CEILING_DB' in text,
        'unit test background attenuation': 'high_strength_rejects_steady_background' in text,
        'unit test speech hangover': 'speech_hangover_preserves_short_gap' in text,
    })
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit('FORGEHX_10_0_21_BACKGROUND_REJECTION=FAIL:' + ','.join(failed))
print('FORGEHX_10_0_21_BACKGROUND_REJECTION=PASS')
