#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.11"' Cargo.toml
grep -Fq 'if let Some(asset) = fields.get(4)' src/orbital.rs
grep -Fq '&& !asset.is_empty()' src/orbital.rs
grep -Fq '&& !names.lines().any(|name| name == *asset)' src/orbital.rs

python3 - src/orbital.rs <<'PY'
from pathlib import Path
import re,sys
s=Path(sys.argv[1]).read_text()
bad=re.compile(
    r'if let Some\(asset\) = fields\.get\(4\) \{\s*'
    r'if !asset\.is_empty\(\) && !names\.lines\(\)\.any\(\|name\| name == \*asset\)',
    re.S,
)
if bad.search(s):
    raise SystemExit("old collapsible nested-if still present")
PY

echo 'FORGECLEAN_V1_0_11_CLIPPY_REGRESSION=PASS'
