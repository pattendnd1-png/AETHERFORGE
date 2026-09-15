#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
direct=(root/'crates/forgehx-dsp/src/direct_pipewire.rs').read_text()
bridge=(root/'crates/forgehx-dsp/src/aetherstream_bridge.rs').read_text()
manager=(root/'crates/forgehx-dsp/src/lib.rs').read_text()
service=(root/'packaging/systemd/forgehx-daemon.service').read_text()
cargo=(root/'Cargo.toml').read_text()
pkgbuild=(root/'PKGBUILD').read_text()
checks={
 'bridge socket': 'forgehx-mic.sock' in bridge,
 'bridge magic': 'AFXHXM01' in bridge,
 'nonblocking datagram': 'UnixDatagram' in bridge and 'set_nonblocking(true)' in bridge,
 'post-dsp bridge': 'send_processed_frame' in direct and 'processed' in direct,
 'no app-facing source class': 'MEDIA_CLASS => "Audio/Source"' not in direct,
 'no default-source ownership': 'ensure_communication_routing' not in manager,
 'aetherstream service dependency': 'aetherstream-audiod.service' in service,
 'release version': 'version = "10.0.30"' in cargo and 'pkgver=10.0.30' in pkgbuild,
}
failed=[name for name,ok in checks.items() if not ok]
if failed:
    print('FORGEHX_AETHERSTREAM_BRIDGE=FAIL:' + ','.join(failed))
    raise SystemExit(1)
print('FORGEHX_AETHERSTREAM_BRIDGE=PASS')
