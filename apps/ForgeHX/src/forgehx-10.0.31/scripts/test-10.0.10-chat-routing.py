from pathlib import Path
root=Path(__file__).resolve().parents[1]
dsp=(root/'crates/forgehx-dsp/src/lib.rs').read_text()
daemon=(root/'crates/forgehx-daemon/src/lib.rs').read_text()
pkg=(root/'PKGBUILD').read_text()
assert 'ensure_communication_routing' in dsp
assert 'run_wpctl_checked(&["set-default", &node_id.to_string()])' in dsp
assert 'set_pipewire_default_source_metadata' in dsp
assert 'default.configured.audio.source' in dsp
assert 'default.audio.source' in dsp
assert 'set-default-source' in dsp
assert 'verify_default_source' in dsp
assert 'verify_pulse_default_source' in dsp
assert 'linking.allow-moving-streams' in dsp
assert 'linking.follow-default-target' in dsp
assert 'let _ = run_wpctl_checked(&["settings", "linking.allow-moving-streams", "true"]);' in dsp
assert "'pipewire-pulse'" in pkg
assert "'libpulse'" in pkg
assert 'ensure_communication_routing' in daemon
assert 'systemctl --user restart wireplumber' not in dsp
assert 'restart_pipewire' not in dsp[dsp.find('ensure_communication_routing'):dsp.find('ensure_communication_routing')+7000]
print('FORGEHX_10_0_7_CHAT_ROUTING=PASS')
