#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 - "$ROOT" <<'PY'
from pathlib import Path
import re, sys
root = Path(sys.argv[1])
live = (root / 'crates/aether-engine-servo/src/live.rs').read_text()
# Rust 1.98 -D warnings rejected all of these nested compatibility-surface cleanup forms.
patterns = [
    re.compile(r"if engine != ContentEngine::Compatibility \{\s*if let Some\(mut surface\) = self\.active_tab_mut\(\)\.compatibility\.take\(\) \{", re.S),
    re.compile(r"if let Some\(mut surface\) = self\.active_tab_mut\(\)\.compatibility\.take\(\) \{\s*if let Some\(host\) = self\.compatibility_host\.as_ref\(\) \{", re.S),
    re.compile(r"if let Some\(mut surface\) = tab\.compatibility\.take\(\) \{\s*if let Some\(host\) = self\.compatibility_host\.as_ref\(\) \{", re.S),
    re.compile(r"if let Some\(mut surface\) = self\.tabs\[index\]\.compatibility\.take\(\) \{\s*if let Some\(host\) = self\.compatibility_host\.as_ref\(\)", re.S),
    re.compile(r"if let Some\(mut surface\) = self\.tabs\[self\.active_index\]\.compatibility\.take\(\) \{\s*if let Some\(host\) = self\.compatibility_host\.as_ref\(\)", re.S),
]
for i, pattern in enumerate(patterns, 1):
    if pattern.search(live):
        raise SystemExit(f'AETHER_BROWSER_V2_1_34_ENGINE_CLIPPY_CLEAN=FAIL:nested-compat-cleanup-{i}')
print('AETHER_BROWSER_V2_1_34_ENGINE_CLIPPY_CLEAN=PASS')
PY
