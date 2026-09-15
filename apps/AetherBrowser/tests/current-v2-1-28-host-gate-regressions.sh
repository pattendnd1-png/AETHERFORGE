#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 - "$ROOT" <<'PY'
from pathlib import Path
import re, sys
root = Path(sys.argv[1])
compat = (root / 'crates/aether-compat/src/lib.rs').read_text()
pages_test = (root / 'crates/aether-native-pages/tests/pages.rs').read_text()
pages_src = (root / 'crates/aether-native-pages/src/lib.rs').read_text()
obs_contract = (root / 'tests/current-obs-websocket-contract.sh').read_text()

nested = re.compile(
    r"if let \(Some\(port\), Some\(path\)\) = \(lines\.next\(\), lines\.next\(\)\) \{\s*"
    r"if !port\.trim\(\)\.is_empty\(\) && !path\.trim\(\)\.is_empty\(\) \{",
    re.S,
)
if nested.search(compat):
    raise SystemExit('AETHER_BROWSER_V2_1_34_HOST_GATE_REGRESSIONS=FAIL:collapsible-if-still-present')
if 'AetherStream service' in pages_test:
    raise SystemExit('AETHER_BROWSER_V2_1_34_HOST_GATE_REGRESSIONS=FAIL:stale-native-page-assertion')
if 'Aether Studio // BrowserAuthoritative' not in pages_test:
    raise SystemExit('AETHER_BROWSER_V2_1_34_HOST_GATE_REGRESSIONS=FAIL:new-studio-contract-not-asserted')
if 'Aether Studio // BrowserAuthoritative' not in pages_src:
    raise SystemExit('AETHER_BROWSER_V2_1_34_HOST_GATE_REGRESSIONS=FAIL:new-studio-surface-missing')
if 'AetherStream service' in obs_contract:
    raise SystemExit('AETHER_BROWSER_V2_1_34_HOST_GATE_REGRESSIONS=FAIL:stale-obs-static-contract')
if 'Aether Studio // BrowserAuthoritative' not in obs_contract:
    raise SystemExit('AETHER_BROWSER_V2_1_34_HOST_GATE_REGRESSIONS=FAIL:new-obs-static-contract-not-asserted')
print('AETHER_BROWSER_V2_1_34_HOST_GATE_REGRESSIONS=PASS')
PY
