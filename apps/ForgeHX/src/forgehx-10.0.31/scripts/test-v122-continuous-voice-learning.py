from pathlib import Path
root = Path(__file__).resolve().parents[1]
core = (root/'crates/forgehx-core/src/lib.rs').read_text()
speaker = (root/'crates/forgehx-dsp/src/speaker_lock.rs').read_text()
engine = (root/'crates/forgehx-dsp/src/engine.rs').read_text()
gui = (root/'crates/forgehx-gui/src/microphone.rs').read_text()
checks = {
    'continuous learning config': 'pub continuous_learning: bool' in core,
    'continuous learning default on': 'continuous_learning: true' in core,
    'verifier owns continuous mode': 'continuous_learning: bool' in speaker,
    'auto enrollment without button': 'FORGEHX_CONTINUOUS_VOICE_LEARNING' in speaker and 'self.begin_enrollment()' in speaker,
    'playback rejected audio cannot train': 'learning_allowed' in speaker and 'let playback_learning_blocked = playback_leak_score >=' in engine and 'evaluate(&cleaned, !playback_learning_blocked)' in engine,
    'playback learning guard survives rejection toggle': 'self.enabled && self.hard_block && score >= self.threshold' in speaker and 'if !self.enabled || self.history.is_empty()' not in speaker,
    'high confidence refinement': 'CONTINUOUS_LEARNING_RATE' in speaker and 'continuous_learning_checkpoint' in speaker,
    'derived voiceprint only': 'completed_voiceprint' in speaker and 'std::fs' not in speaker,
    'gui continuous control': 'Continuous voice learning' in gui,
    'gui no record button': 'Train My Voice' not in gui,
    'gui waiting status': 'waiting for your voice' in gui.lower(),
    'unit auto enrollment': 'continuous_learning_starts_enrollment_without_record_button' in speaker,
    'unit ongoing refinement': 'continuous_learning_refines_high_confidence_voiceprint' in speaker,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit('FAIL: ' + ', '.join(failed))
print('PASS: ForgeHX 10.0.6 continuous voice-learning contract')
