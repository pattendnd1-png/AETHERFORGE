#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -q '^version = "10.0.10"$' Cargo.toml || { echo 'workspace version 10.0.10 missing' >&2; exit 1; }
grep -q '^pkgver=10.0.10$' PKGBUILD || { echo 'PKGBUILD pkgver 10.0.10 missing' >&2; exit 1; }
grep -q '^pkgrel=1$' PKGBUILD || { echo 'PKGBUILD pkgrel 1 missing' >&2; exit 1; }

grep -q 'pub const IPC_PROTOCOL_VERSION: u32 = 11;' crates/forgehx-core/src/lib.rs || { echo 'IPC v11 missing' >&2; exit 1; }
grep -q 'KeyboardCapabilities { protocol_version: u32, device_id: DeviceId }' crates/forgehx-core/src/lib.rs || { echo 'keyboard command missing' >&2; exit 1; }
grep -q 'KeyboardCapabilities { model: Option<KeyboardModelInfo> }' crates/forgehx-core/src/lib.rs || { echo 'keyboard reply missing' >&2; exit 1; }
grep -q 'Reply::KeyboardCapabilities' crates/forgehx-cli/src/main.rs || { echo 'CLI reply arm missing' >&2; exit 1; }
grep -q 'TopCommand::Keyboard' crates/forgehx-cli/src/main.rs || { echo 'keyboard CLI command missing' >&2; exit 1; }
python - <<'PYCLI'
from pathlib import Path
import re
src = Path('crates/forgehx-cli/src/main.rs').read_text()
nested = sorted(set(re.findall(r'#\[command\(subcommand\)\]\s+command:\s*([A-Za-z0-9_]+)', src)))
missing = []
for enum_name in nested:
    pattern = rf'#\[derive\([^\]]*\bDebug\b[^\]]*\bSubcommand\b[^\]]*\)\]\s*enum\s+{re.escape(enum_name)}\b'
    if not re.search(pattern, src, re.S):
        missing.append(enum_name)
if missing:
    raise SystemExit('nested clap subcommand enums missing Debug + Subcommand derive: ' + ', '.join(missing))
PYCLI
for variant in FirmwareIdentity FirmwarePackage FirmwareTransaction; do
  grep -q "Reply::${variant}" crates/forgehx-cli/src/main.rs || { echo "missing firmware reply arm $variant" >&2; exit 1; }
done
./scripts/test-dsp-state-ownership.sh

if grep -nE 'Stroke::new\((3|5|6|7|8|13)\.0,' crates/forgehx-gui/src/device_page.rs >/dev/null; then
  echo 'future-incompatible inferred f64 Stroke width reintroduced' >&2
  exit 1
fi
echo 'ForgeHX 10.0.10 compile regression guards passed.'
