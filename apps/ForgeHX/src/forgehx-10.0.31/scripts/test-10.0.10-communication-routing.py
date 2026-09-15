from pathlib import Path
root = Path(__file__).resolve().parents[1]
dsp = (root/'crates/forgehx-dsp/src/lib.rs').read_text()
pkg = (root/'PKGBUILD').read_text()
daemon_unit = (root/'packaging/systemd/forgehx-daemon.service').read_text()
assert 'set-default-source' in dsp, 'missing pactl fallback for virtual source defaulting'
assert 'verify_default_source' in dsp, 'routing must verify the resulting default source'
assert "'pipewire-pulse'" in pkg, 'pipewire-pulse runtime dependency missing'
assert "'libpulse'" in pkg, 'libpulse/pactl runtime dependency missing'
assert 'Wants=pipewire-pulse.socket' in daemon_unit, 'daemon must ensure PipeWire-Pulse compatibility socket is available'
assert 'let _ = run_wpctl_checked(&["settings", "linking.allow-moving-streams", "true"]);' in dsp, 'optional WirePlumber setting must not abort routing'
assert 'run_wpctl_checked(&["set-default", &node_id.to_string()])' in dsp, 'native wpctl default attempt missing'
print('FORGEHX_10_0_7_COMMUNICATION_ROUTING=PASS')
