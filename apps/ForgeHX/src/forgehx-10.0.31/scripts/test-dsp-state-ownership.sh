#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

python - <<'PY'
from pathlib import Path
src = Path('crates/forgehx-dsp/src/lib.rs').read_text()
expected = 'raw_source_node_name: active.as_ref().map(|value| value.raw_source_node_name.clone()),'
if expected not in src:
    raise SystemExit(
        'MicrophoneDspManager::state must borrow ActiveMicState when projecting raw_source_node_name; '
        'consuming Option<ActiveMicState> before active.is_some() causes E0382'
    )
if 'raw_source_node_name: active.map(|value| value.raw_source_node_name),' in src:
    raise SystemExit('consuming active Option reintroduced in MicrophoneDspManager::state')
PY

echo 'ForgeHX microphone DSP state ownership regression passed.'
