from pathlib import Path
root = Path(__file__).resolve().parents[1]
core = (root/'crates/forgehx-core/src/lib.rs').read_text()
dsp_lib = (root/'crates/forgehx-dsp/src/lib.rs').read_text()
engine = (root/'crates/forgehx-dsp/src/engine.rs').read_text()
gui = (root/'crates/forgehx-gui/src/microphone.rs').read_text()
transient_path = root/'crates/forgehx-dsp/src/transient.rs'
checks = {
    'config type': 'pub struct ClickSuppressionConfig' in core,
    'config field': 'pub click_suppression: ClickSuppressionConfig' in core,
    'enabled default': 'enabled: true' in core and 'sensitivity_percent: 90.0' in core,
    'module export': 'pub mod transient;' in dsp_lib,
    'processor exists': transient_path.exists() and 'pub struct TransientSuppressor' in transient_path.read_text() if transient_path.exists() else False,
    'engine owns processor': 'click_suppressor: TransientSuppressor' in engine,
    'engine applies processor': 'self.click_suppressor.process(&mut cleaned)' in engine,
    'live ui control': 'Keyboard / Mouse Click Suppression' in gui and 'Suppress keyboard and mouse clicks' in gui,
    'sensitivity control': 'Click suppression sensitivity (%)' in gui,
    'noise-domain ownership': 'before.click_suppression != config.click_suppression' in gui,
    'unit tests': transient_path.exists() and 'isolated_impulse_is_strongly_reduced' in transient_path.read_text() and 'steady_voice_like_wave_is_preserved' in transient_path.read_text() if transient_path.exists() else False,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit('FAIL: ' + ', '.join(failed))
print('PASS: ForgeHX 10.0.6 keyboard/mouse click suppression contract')
