#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 - "$ROOT" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
contract = (root / 'tests/current-obs-websocket-contract.sh').read_text()
old = 'grep -qF "stream.html.contains(\\"$stable\\")" "$PAGES_TEST"'
if old in contract:
    raise SystemExit('AETHER_BROWSER_V2_1_34_OBS_CONTRACT_ROBUSTNESS=FAIL:brittle-exact-rust-expression-still-present')
required = [
    'AETHER_OBS_STABLE_ASSERTION_CHECK=SEMANTIC_FUNCTION_SCOPE',
    'stream_studio_and_vault_are_visible_first_party_surfaces',
    "'Aether Stream Studio'",
    "'OBS WebSocket'",
    "'Aether Studio // BrowserAuthoritative'",
    "'Scenes'",
    "'Mixer'",
]
for token in required:
    if token not in contract:
        raise SystemExit(f'AETHER_BROWSER_V2_1_34_OBS_CONTRACT_ROBUSTNESS=FAIL:missing:{token}')
print('AETHER_BROWSER_V2_1_34_OBS_CONTRACT_ROBUSTNESS=PASS')
PY
