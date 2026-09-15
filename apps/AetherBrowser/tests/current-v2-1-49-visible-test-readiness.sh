#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
LIVE="$ROOT/crates/aether-engine-servo/src/live.rs"
INSTALL="$ROOT/scripts/install-current-tree.sh"
fail(){ echo "AETHER_BROWSER_V2_1_52_VISIBLE_TEST_READINESS=FAIL:$1"; exit 1; }

grep -qF 'AETHER_BROWSER_VISIBLE_TEST_SCREEN=PASS' "$LIVE" || fail visible-test-marker-missing
grep -qF 'AETHER_BROWSER_LIVE_FRAME_CAPTURE_SCOPE=WINDOW_CHROME_EXTERNAL_CHILD' "$LIVE" || fail external-child-capture-scope-missing
grep -qF 'AETHER_BROWSER_LIVE_FRAME_PROBE_REVEALED=PASS' "$LIVE" || fail probe-reveal-marker-missing

grep -qF "record 'AETHER_BROWSER_POSTINSTALL_VISUAL=FAIL'" "$INSTALL" || fail visual-fail-marker-missing
grep -qF 'hard_fail=1' "$INSTALL" || fail hard-fail-support-missing
python3 - "$INSTALL" <<'PY'
from pathlib import Path
import sys
text=Path(sys.argv[1]).read_text()
for marker in ('AETHER_BROWSER_POSTINSTALL_VISUAL_WARNING=1:external-web-render','AETHER_BROWSER_POSTINSTALL_VISUAL_WARNING=1:custom-resolution-render'):
    if marker in text:
        raise SystemExit('FAIL:warning-only-visual-probe-still-present')
print('INSTALL_VISUAL_HARD_FAIL=PASS')
PY


echo 'AETHER_BROWSER_V2_1_52_VISIBLE_TEST_READINESS=PASS'
