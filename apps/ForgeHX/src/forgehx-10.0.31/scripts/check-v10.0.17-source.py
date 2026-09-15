#!/usr/bin/env python3
from pathlib import Path
import re
root = Path(__file__).resolve().parents[1]
errors=[]
def read(rel): return (root/rel).read_text()

cargo=read('Cargo.toml'); pkg=read('PKGBUILD'); bridge=read('crates/forgehx-dsp/src/aetherstream_bridge.rs')
direct=read('crates/forgehx-dsp/src/direct_pipewire.rs'); manager=read('crates/forgehx-dsp/src/lib.rs')
daemon=read('crates/forgehx-daemon/src/lib.rs'); service=read('packaging/systemd/forgehx-daemon.service')
runtime=read('crates/forgehx-dsp/src/runtime.rs'); readme=read('README.md')

checks={
 'workspace version': 'version = "10.0.17"' in cargo,
 'package version': 'pkgver=10.0.17' in pkg,
 'bridge magic': '*b"AFXHXM01"' in bridge,
 'bridge version': 'BRIDGE_VERSION: u16 = 1' in bridge,
 '48k mono contract': 'BRIDGE_RATE: u32 = 48_000' in bridge and 'BRIDGE_CHANNELS: u16 = 1' in bridge,
 '10ms frame': 'FRAME_SAMPLES, 480' in bridge or 'FRAME_SAMPLES *' in bridge,
 'fixed packet': 'BRIDGE_PACKET_BYTES' in bridge and '1_952' in bridge,
 'nonblocking datagram': 'UnixDatagram::unbound()' in bridge and 'set_nonblocking(true)' in bridge,
 'post dsp sender': 'send_processed_frame(&processed)' in direct,
 'no ForgeHX app source': 'MEDIA_CLASS => "Audio/Source"' not in direct,
 'no default-source owner': 'ensure_communication_routing' not in manager and 'set-default' not in manager,
 'AetherStream app source': 'aetherstream.system.microphone' in bridge and 'AETHERSTREAM_SYSTEM_SOURCE_NAME' in runtime,
 'AetherStream service dependency': 'aetherstream-audiod.service' in service and 'Wants=aetherstream-audiod.service' in service,
 'direct capture preserved': 'Some(raw_source_node_id)' in direct and 'direct_capture' in direct,
 'daemon bridge restore': 'ForgeHX→AetherStream' in daemon or 'AetherStream' in daemon,
 'release notes': readme.startswith('# ForgeHX 10.0.17'),
}
for name, ok in checks.items():
    if not ok: errors.append(name)
required=[
 'crates/forgehx-dsp/src/aetherstream_bridge.rs',
 'scripts/test-10.0.17-aetherstream-bridge.py',
 'docs/superpowers/specs/2026-09-03-aetherstream-bridge-design.md',
 'docs/superpowers/plans/2026-09-03-aetherstream-bridge.md',
]
for rel in required:
    if not (root/rel).is_file(): errors.append(f'missing {rel}')
if errors:
    print('FORGEHX_10_0_17_SOURCE_CONTRACT=FAIL:' + ','.join(errors))
    raise SystemExit(1)
print('FORGEHX_10_0_17_SOURCE_CONTRACT=PASS')
