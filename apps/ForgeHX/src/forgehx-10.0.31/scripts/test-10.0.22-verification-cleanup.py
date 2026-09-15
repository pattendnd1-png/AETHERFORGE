from pathlib import Path
root = Path(__file__).resolve().parents[1]
core = (root/'crates/forgehx-core/src/lib.rs').read_text()
daemon = (root/'crates/forgehx-daemon/src/lib.rs').read_text()
gui = (root/'crates/forgehx-gui/src/app.rs').read_text()
verify = (root/'scripts/verify-10.0.22.sh').read_text() if (root/'scripts/verify-10.0.22.sh').exists() else ''
checks = {
    'lighting default is derivable': '#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]' in core and '#[default]\n    None,' in core,
    'manual lighting Default removed': 'impl Default for KeyboardLightingTopology' not in core,
    'large mic reply boxed': 'MicDspState { state: Box<MicrophoneDspState> }' in core,
    'daemon boxes mic states': 'Reply::MicDspState { state: Box::new(state) }' in daemon,
    'gui unboxes mic state': 'self.mic_dsp_state = Some(*state);' in gui,
    'format apply before format check': 'run FORMAT_APPLY cargo fmt --all' in verify and 'run FORMAT cargo fmt --all -- --check' in verify,
    'socket is hard runtime health': 'FORGEHX_AETHERSTREAM_OUTPUT_DSP_SOCKET=PASS' in verify and 'fail=1' in verify,
    'inactive unit diagnostic only': 'FORGEHX_AETHERSTREAM_AUDIOD=INFO:inactive_or_different_unit' in verify,
    'solocast visibility diagnostic only': 'FORGEHX_SOLOCAST_ENDPOINT_VISIBLE=INFO:not_observed' in verify,
    'routing mutation forbidden': 'test-10.0.18-solocast-routing-lock.py' in verify,
}
failed=[name for name,ok in checks.items() if not ok]
if failed:
    raise SystemExit('FAIL: '+', '.join(failed))
print('PASS: ForgeHX 10.0.22 verification/runtime cleanup contract')
