#!/usr/bin/env bash
set -euo pipefail

fail() { echo "FAIL: $*" >&2; exit 1; }

python - <<'PYCHECK'
from pathlib import Path
import re
s=Path('crates/forgehx-dsp/src/runtime.rs').read_text()
m=re.search(r'fn run_worker\((.*?)\) \{', s, re.S)
if not m or 'processed_source' not in m.group(1):
    raise SystemExit('FAIL: direct runtime worker must carry processed_source identity')
body=s[m.end():]
if 'run_direct_pipewire' not in body or 'processed_source' not in body[:body.find('fn run_monitor_worker')]:
    raise SystemExit('FAIL: processed_source is not consumed by direct PipeWire runtime')
PYCHECK
! grep -qE '\b(FirmwareSupportLevel|FirmwareTransactionStatus)\b' <(sed -n '1,18p' crates/forgehx-daemon/src/lib.rs) || fail 'unused firmware imports remain'
grep -qx 'pub use forgehx_ipc::send;' crates/forgehx-gui/src/ipc.rs || fail 'GUI IPC re-export still exposes unused socket_path'
! grep -qE '^    (Bypass|EnrollStart|EnrollCancel),$' crates/forgehx-gui/src/microphone.rs || fail 'unconstructed microphone control variants remain'
! grep -qE 'MicControl::(Bypass|EnrollStart|EnrollCancel)' crates/forgehx-gui/src/device_page.rs || fail 'dead microphone control match arms remain'
grep -Fq 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo build --workspace --release' PKGBUILD || fail 'Arch release build does not deny Rust warnings'
grep -Fq 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test --workspace' PKGBUILD || fail 'Arch test gate does not deny Rust warnings'
grep -Fq 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test --workspace' scripts/build-release.sh || fail 'release test gate does not deny Rust warnings'
grep -Fq 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo build --workspace --release' scripts/build-release.sh || fail 'release build does not deny Rust warnings'

echo 'PASS: ForgeHX 10.0.12 warning-clean contract'
