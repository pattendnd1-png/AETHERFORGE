#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_60_RUSTFMT_CLOSURE=FAIL:$1"; exit 1; }
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
LIVE="$ROOT/crates/aether-engine-servo/src/live.rs"
PAGES="$ROOT/crates/aether-native-pages/tests/pages.rs"
grep -qF 'use aether_native_pages::{NativePageRoute, native_page_for_url, native_page_for_url_with_library};' "$LIVE" || fail native-pages-import
python3 - "$PAGES" <<'PY2'
from pathlib import Path
import sys
raw=Path(sys.argv[1]).read_bytes()
if not raw.endswith(b'}\n') or raw.endswith(b'}\n\n'):
    raise SystemExit('AETHER_BROWSER_V2_1_60_RUSTFMT_CLOSURE=FAIL:pages-trailing-blank')
PY2
echo 'AETHER_BROWSER_V2_1_60_RUSTFMT_CLOSURE=PASS'
