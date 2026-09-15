#!/usr/bin/env python3
from pathlib import Path

runtime = Path('crates/forgehx-dsp/src/runtime.rs').read_text()
dsp = Path('crates/forgehx-dsp/src/lib.rs').read_text()
daemon = Path('crates/forgehx-daemon/src/lib.rs').read_text()

checks = {
    'direct PipeWire runtime owns capture/output': 'run_direct_pipewire' in runtime,
    'no pw-loopback bridge': 'pw-loopback' not in runtime,
    'manager direct-source restore API': 'pub fn ensure_direct_source(&self, device_id: &DeviceId)' in dsp,
    'remembered raw node identity retained': 'active.raw_source_node_name' in dsp,
    'stable app-facing source retained': 'processed_source_name(device_id)' in dsp,
    'HyperX microphone source eligibility helper': 'fn should_keep_forgehx_mic_source' in daemon,
    'missing-raw-source restore uses direct source': 'ensure_direct_source(&device.id)' in daemon,
    'unit regression for missing raw source': 'forgehx_mic_identity_survives_missing_raw_source' in daemon,
}
failed = [name for name, ok in checks.items() if not ok]
if failed:
    raise SystemExit('FAIL: missing migrated 10.0.10 persistent microphone guarantees: ' + ', '.join(failed))

restore_start = daemon.find('fn restore_always_on_dsp')
restore_end = daemon.find('pub fn handle', restore_start)
restore = daemon[restore_start:restore_end]
start_direct = restore.find('ensure_direct_source(&device.id)')
route = restore.find('ensure_communication_routing(&device.id)')
if start_direct < 0 or route < 0 or start_direct > route:
    raise SystemExit('FAIL: ForgeHX Mic runtime must be restored before communication routing is promoted')

print('PASS: ForgeHX 10.0.10 persistent microphone guarantees migrated to direct ForgeHX Mic runtime')
