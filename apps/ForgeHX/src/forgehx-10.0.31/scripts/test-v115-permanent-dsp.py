from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
dsp=(root/'crates/forgehx-dsp/src/lib.rs').read_text()
runtime=(root/'crates/forgehx-dsp/src/runtime.rs').read_text()
daemon=(root/'crates/forgehx-daemon/src/lib.rs').read_text()
gui=(root/'crates/forgehx-gui/src/microphone.rs').read_text()
checks={
 'permanent dsp invariant': 'FORGEHX_PERMANENT_DSP' in dsp,
 'always-on restore method': 'pub fn ensure_always_on' in dsp,
 'daemon reconnect restore': 'ensure_always_on' in daemon and 'restore_always_on_dsp' in daemon,
 'runtime self-healing direct capture': 'FORGEHX_DSP_RECONNECT_MS' in runtime and 'run_direct_pipewire' in runtime and 'pw-loopback' not in runtime,
 'reference self-healing': 'reconnect_reference_session' in runtime,
 'no user bypass control': 'DSP permanently enabled' in gui and 'ui.button("Bypass")' not in gui,
 'normal apply preserves PipeWire graph': 'restart_pipewire' not in dsp and 'cleanup_legacy_bridge' in dsp,
 'bypass cannot deactivate permanent dsp': 'permanent DSP cannot be bypassed' in dsp,
}
failed=[name for name, ok in checks.items() if not ok]
if failed:
    print('FAIL: '+', '.join(failed))
    sys.exit(1)
print('PASS: ForgeHX permanent DSP/reconnect contract on direct hardware capture runtime')
