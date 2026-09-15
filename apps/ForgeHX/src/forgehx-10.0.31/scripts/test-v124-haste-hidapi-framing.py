#!/usr/bin/env python3
from pathlib import Path
p=Path('crates/forgehx-mouse/src/protocol/haste_v1.rs')
s=p.read_text()
required=[
    'const HIDAPI_OUTPUT_SIZE: usize = PACKET_SIZE + 1;',
    'frame[0] = 0;',
    'frame[1..].copy_from_slice(packet);',
    '.write(&frame)',
    'written != HIDAPI_OUTPUT_SIZE',
    'hidapi_output_frame',
]
missing=[x for x in required if x not in s]
if missing:
    raise SystemExit('FAIL: missing HIDAPI report-ID framing markers: '+repr(missing))
print('PASS: ForgeHX 10.0.6 Haste HIDAPI report-ID framing contract')
