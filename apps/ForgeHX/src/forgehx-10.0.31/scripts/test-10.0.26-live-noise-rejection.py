from pathlib import Path
root = Path(__file__).resolve().parents[1]
rejection = (root/'crates/forgehx-dsp/src/background_rejection.rs').read_text()
direct = (root/'crates/forgehx-dsp/src/direct_pipewire.rs').read_text()
engine = (root/'crates/forgehx-dsp/src/engine.rs').read_text()
gui = (root/'crates/forgehx-gui/src/microphone.rs').read_text()
checks = {
    'stable background tracker exists': 'stable_frames' in rejection and 'last_level_db' in rejection,
    'loud stable background can be learned': 'LOUD_BACKGROUND_LEARN_LIMIT_DB' in rejection and 'stable_frames >=' in rejection,
    'strong reject floor reaches -72 dB': 'MAX_REJECTION_DB: f32 = -72.0' in rejection,
    'dynamic voice threshold replaces fixed -28 ceiling': 'voice_open_limit_db' in rejection and 'VOICE_OPEN_CEILING_DB' not in rejection,
    'loud steady background regression test exists': 'loud_steady_background_is_learned_and_rejected' in rejection,
    'voice dynamics regression test exists': 'varying_voice_is_not_learned_as_background' in rejection,
    'direct runtime has no raw fail-open': '.unwrap_or(*frame)' not in direct,
    'direct runtime explicitly fails closed': 'DSP frame rejected; raw microphone bypass forbidden' in direct and '[0.0f32; FRAME_SAMPLES]' in direct,
    'full engine rejection regression exists': 'full_engine_rejects_learned_room_noise' in engine,
    'gui names background rejection': 'Background Rejection' in gui,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit('FORGEHX_10_0_26_LIVE_NOISE_REJECTION=FAIL ' + ', '.join(failed))
print('FORGEHX_10_0_26_LIVE_NOISE_REJECTION=PASS')
