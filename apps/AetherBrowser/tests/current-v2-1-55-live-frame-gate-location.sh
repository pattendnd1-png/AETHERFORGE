#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
SHUTDOWN="$ROOT/crates/aether-engine-servo/tests/shutdown.rs"
INSTALL="$ROOT/scripts/install-current-tree.sh"
fail(){ echo "AETHER_BROWSER_V2_1_56_LIVE_FRAME_GATE_LOCATION=FAIL:$1"; exit 1; }

grep -qF 'const INSTALL_SOURCE: &str = include_str!("../../../scripts/install-current-tree.sh");' "$SHUTDOWN" || fail shutdown-test-not-retargeted
! grep -qF 'const VERIFY_SOURCE: &str = include_str!("../../../scripts/verify.sh");' "$SHUTDOWN" || fail stale-preinstall-verifier-binding

grep -qF 'installed_web_rc=$?' "$INSTALL" || fail installed-web-probe-rc-missing
grep -qF 'if (( installed_web_rc == 0 ))' "$INSTALL" || fail installed-web-probe-zero-only-success-missing
grep -qF "record 'AETHER_BROWSER_POSTINSTALL_VISUAL=FAIL'" "$INSTALL" || fail installed-visual-failure-marker-missing

python3 - "$INSTALL" <<'PY'
from pathlib import Path
import sys
text = Path(sys.argv[1]).read_text()
start = text.find('installed_web_rc=$?')
end = text.find('rm -f /tmp/aether-browser-installed-web-probe.log', start)
if start < 0 or end < 0:
    raise SystemExit('AETHER_BROWSER_V2_1_56_LIVE_FRAME_GATE_LOCATION=FAIL:installed-web-probe-block-missing')
block = text[start:end]
if 'hard_fail=1' not in block:
    raise SystemExit('AETHER_BROWSER_V2_1_56_LIVE_FRAME_GATE_LOCATION=FAIL:installed-web-probe-not-fatal')
for forbidden in ('PASS_WITH_139', 'installed_web_rc == 139', 'installed_web_rc==139'):
    if forbidden in block:
        raise SystemExit('AETHER_BROWSER_V2_1_56_LIVE_FRAME_GATE_LOCATION=FAIL:nonzero-probe-exception-present')
PY

echo 'AETHER_BROWSER_V2_1_56_LIVE_FRAME_GATE_LOCATION=PASS'
