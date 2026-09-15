#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_HARD_FAILURE_ROLLBACK=FAIL:$1"; exit 1; }
INSTALL=scripts/install-current-tree.sh
grep -qF 'bash "$ROOT/scripts/rollback-host.sh"' "$INSTALL" || fail no-hard-failure-rollback
grep -qF 'AETHER_BROWSER_INSTALL_HARD_FAILURE_ROLLBACK=PASS' "$INSTALL" || fail no-rollback-pass-marker
python3 - "$INSTALL" <<'PY'
from pathlib import Path
import sys
s=Path(sys.argv[1]).read_text()
for marker in ['AETHER_BROWSER_INSTALLED_TWITCH_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE','AETHER_BROWSER_INSTALLED_YOUTUBE_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE','AETHER_BROWSER_INSTALLED_YOUTUBE_MUSIC_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE']:
    if marker not in s: raise SystemExit('AETHER_BROWSER_HARD_FAILURE_ROLLBACK=FAIL:missing-focused-skip:'+marker)
r=s.find('bash "$ROOT/scripts/rollback-host.sh"'); final_fail=s.rfind('INSTALL_VERIFY=FAIL')
if r < 0 or final_fail < 0 or r > final_fail: raise SystemExit('AETHER_BROWSER_HARD_FAILURE_ROLLBACK=FAIL:rollback-not-before-final-fail')
print('AETHER_BROWSER_HARD_FAILURE_ROLLBACK=PASS')
PY
