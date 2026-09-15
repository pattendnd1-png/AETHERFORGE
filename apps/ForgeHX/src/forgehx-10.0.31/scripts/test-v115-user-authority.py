from pathlib import Path
root=Path(__file__).resolve().parents[1]
app=(root/'crates/forgehx-gui/src/app.rs').read_text()
dsp=(root/'crates/forgehx-dsp/src/lib.rs').read_text()
mic=(root/'crates/forgehx-gui/src/microphone.rs').read_text()
checks = {
    'editor authority marker': 'FORGEHX_USER_CONFIG_AUTHORITY' in app,
    'daemon snapshot cannot overwrite editor blindly': 'self.mic_dsp_config = state.config.clone(); self.mic_dsp_state = Some(state)' not in app,
    'reply preserves user editor': 'state.config = self.mic_dsp_config.clone()' in app,
    'live edits unconditional': 'let live_mic_dsp_edits = true;' in app,
    'active profile follows live config': 'active.profile_name = config.name.clone()' in dsp,
    'active mapping persisted': 'self.write_active(device_id, &active)' in dsp,
    'manual domain ownership remains': 'claim_user_control_domains(&before, config);' in mic,
}
failed=[name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit('FAIL: '+', '.join(failed))
print('PASS: ForgeHX 10.0.6 user-authority contract')
