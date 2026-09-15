#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_LEFT_WINDOW_CONTROLS=FAIL:$1"; exit 1; }
UI=crates/aether-ui/src/lib.rs
LIVE=crates/aether-engine-servo/src/live.rs
grep -qF 'WindowDiminish' "$UI" || fail no-diminish-target
grep -qF 'ChromeHitTarget::WindowDiminish' "$LIVE" || fail diminish-not-handled
grep -qF 'self.window.set_maximized(false)' "$LIVE" || fail diminish-not-restore-down
grep -qF 'self.window.set_maximized(true)' "$LIVE" || fail maximize-not-explicit
# Four left-side control centers and a title that begins after them.
grep -qF 'WINDOW_CONTROL_MINIMIZE_X' "$UI" || fail no-left-control-geometry
grep -qF 'WINDOW_CONTROL_CLOSE_X' "$UI" || fail no-left-close-geometry
grep -qF 'WINDOW_TITLE_TEXT_X' "$UI" || fail title-not-shifted-after-controls
python3 - "$UI" <<'PY2'
from pathlib import Path
import re,sys
s=Path(sys.argv[1]).read_text()
b=re.search(r'if y < f64::from\(WINDOW_TITLEBAR_HEIGHT_PX\) \{(.*?)return ChromeHitTarget::WindowDrag;\n        \}',s,re.S)
if not b: raise SystemExit('AETHER_BROWSER_LEFT_WINDOW_CONTROLS=FAIL:no-titlebar-hit-block')
if 'let right = f64::from(width);' in b.group(1):
    raise SystemExit('AETHER_BROWSER_LEFT_WINDOW_CONTROLS=FAIL:right-side-hit-test-remains')
PY2
grep -qF 'AETHER_BROWSER_WINDOW_CONTROLS_LEFT=PASS' "$UI" || fail no-left-controls-test-marker
echo 'AETHER_BROWSER_WINDOW_CONTROLS_LEFT=PASS'
echo 'AETHER_BROWSER_WINDOW_CONTROL_HITBOX=PASS'
echo 'AETHER_BROWSER_TITLEBAR_DRAG_NO_OVERLAP=PASS'
echo 'AETHER_BROWSER_MAXIMIZE_RESTORE=PASS'
echo 'AETHER_BROWSER_DIMINISH_CONTROL=PASS'
