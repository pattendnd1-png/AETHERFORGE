from pathlib import Path

root = Path('.')
direct = (root / 'crates/forgehx-dsp/src/direct_pipewire.rs').read_text()
lib = (root / 'crates/forgehx-dsp/src/lib.rs').read_text()

checks = {
    'unsupported pipewire-rs target key removed': 'pw::keys::TARGET_OBJECT' not in direct,
    'physical target node id is resolved': 'wait_for_source_node_id(raw_source)' in direct,
    'capture connect receives target id': 'Some(raw_source_node_id)' in direct,
    'reconnect-capable registry resolver retained': 'find_source_node_id_via_pw_cli' in lib and 'pw-cli' in lib,
}
missing = [name for name, ok in checks.items() if not ok]
if missing:
    raise SystemExit('FAIL: 10.0.14 direct target-id contract: ' + ', '.join(missing))
print('PASS: ForgeHX 10.0.14 direct target-id contract')
