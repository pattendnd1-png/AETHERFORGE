#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
LIVE="$ROOT/crates/aether-engine-servo/src/live.rs"
fail(){ echo "AETHER_BROWSER_V2_1_47_ENGINE_CLIPPY_CLEAN=FAIL:$1"; exit 1; }

grep -q 'fn active_external_app_surface' "$LIVE" && fail dead-helper-active-external-app-surface
python3 - "$LIVE" <<'PY' || exit 1
from pathlib import Path
import sys
text=Path(sys.argv[1]).read_text()
old = '''if let (Some(host), Some(surface)) = (self.external_app_host.as_ref(), tab.external_app.as_mut()) {
                if surface.attached() {
                    if index == active_index {'''
if old in text:
    print('AETHER_BROWSER_V2_1_47_ENGINE_CLIPPY_CLEAN=FAIL:collapsible-external-app-if')
    raise SystemExit(1)
if '&& surface.attached()' not in text or 'AETHER_BROWSER_EXTERNAL_APP_RESIZE=FAIL' not in text:
    print('AETHER_BROWSER_V2_1_47_ENGINE_CLIPPY_CLEAN=FAIL:collapsed-external-app-guard-missing')
    raise SystemExit(1)
PY
echo 'AETHER_BROWSER_V2_1_47_ENGINE_CLIPPY_CLEAN=PASS'
