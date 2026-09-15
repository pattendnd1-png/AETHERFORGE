#!/usr/bin/env python3
from pathlib import Path
p=Path(__file__).resolve().parents[1]/'crates/forgehx-daemon/src/lib.rs'
s=p.read_text()
need=[
    'fn stable_audio_device_id(',
    'stable_audio_device_id(&node.name)',
    'stable_audio_device_id_is_independent_of_pipewire_node_number',
]
missing=[x for x in need if x not in s]
if 'DeviceId(format!("audio:{}",node.id))' in s.replace(' ', ''):
    missing.append('transient audio:<node.id> identity still present')
if missing:
    raise SystemExit('FAIL: unstable microphone identity markers: '+repr(missing))
print('PASS: ForgeHX 10.0.10 stable microphone identity contract')
