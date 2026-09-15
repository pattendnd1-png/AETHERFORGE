#!/usr/bin/env python3
from pathlib import Path

dsp = Path('crates/forgehx-dsp/src/lib.rs').read_text()

required = {
    'PipeWire registry node parser': 'parse_pw_cli_named_node_id' in dsp,
    'PipeWire registry lookup command': '.args(["ls", "Node"])' in dsp,
    'registry fallback participates in source wait': 'find_source_node_id_via_pw_cli' in dsp,
    'lookup still retains wpctl status compatibility': '.args(["status", "-n"])' in dsp,
}
missing = [name for name, ok in required.items() if not ok]
if missing:
    raise SystemExit('FAIL: 10.0.11 virtual-source node resolution missing: ' + ', '.join(missing))

print('PASS: ForgeHX 10.0.11 virtual-source node resolution contract')
