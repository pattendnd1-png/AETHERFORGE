from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
mic=(root/'crates/forgehx-gui/src/microphone.rs').read_text()
app=(root/'crates/forgehx-gui/src/app.rs').read_text()
runtime=(root/'crates/forgehx-dsp/src/runtime.rs').read_text()
engine=(root/'crates/forgehx-dsp/src/engine.rs').read_text()
required={
 'manual domain claim helper': 'fn claim_user_control_domains(' in mic,
 'noise becomes manual': 'auto_noise' in mic and 'before.noise_suppression != config.noise_suppression' in mic,
 'aec becomes manual': 'auto_aec' in mic and 'before.echo_cancellation != config.echo_cancellation' in mic,
 'eq becomes manual': 'auto_eq' in mic and 'before.eq' in mic,
 'tone becomes manual': 'auto_tone' in mic and 'before.tone' in mic,
 'dynamics becomes manual': 'auto_dynamics' in mic and 'before.compressor' in mic,
 'deesser becomes manual': 'auto_de_esser' in mic and 'before.de_esser' in mic,
 'loudness becomes manual': 'auto_loudness' in mic and 'before.output_gain_db' in mic,
 'user controls always live': 'let live_mic_dsp_edits = true;' in app,
 'runtime hot update': 'pub fn update(&self, key: &str, config: MicrophoneDspConfig)' in runtime,
 'engine hot update': 'pub fn update_config(&mut self, config: MicrophoneDspConfig)' in engine,
 'live override label': 'LIVE / USER OVERRIDE' in mic,
}
failed=[name for name,ok in required.items() if not ok]
if failed:
 print('FAIL='+';'.join(failed))
 sys.exit(1)
print('PASS: ForgeHX 10.0.6 live-active user-control contract')
