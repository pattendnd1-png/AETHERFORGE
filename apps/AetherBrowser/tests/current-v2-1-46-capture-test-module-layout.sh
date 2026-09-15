#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
FILE="$ROOT/crates/aether-capture/src/lib.rs"
python3 - "$FILE" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1])
lines = p.read_text().splitlines()
test_line = next((i for i,l in enumerate(lines,1) if l.strip() == '#[cfg(test)]'), None)
if test_line is None:
    raise SystemExit('FAIL:capture-test-module-missing')
production_after=[]
for i,l in enumerate(lines,1):
    if i <= test_line:
        continue
    s=l.strip()
    if s.startswith('pub fn ') or s.startswith('fn downloads_root('):
        production_after.append((i,s))
if production_after:
    print('FAIL:production-items-after-test-module:' + ','.join(f'{i}:{s}' for i,s in production_after))
    raise SystemExit(1)
print('AETHER_BROWSER_V2_1_47_CAPTURE_TEST_MODULE_LAYOUT=PASS')
PY
