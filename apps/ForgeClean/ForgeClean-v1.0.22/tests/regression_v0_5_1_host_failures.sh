#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
TEST="$ROOT/tests/system_scan.rs"
COLD="$ROOT/src/coldstore.rs"

grep -Fq 'SYSTEM_MANIFEST_FILENAME' "$TEST"
! grep -Fq 'ForgeClean-v0.4.2-PACMAN-BATCH.txt' "$TEST"
python3 - "$COLD" <<'PY'
from pathlib import Path
import re, sys
text = Path(sys.argv[1]).read_text()
pat = re.compile(r'if let Some\(store_dir\) = verify_store\s*\n\s*&& hash_manifest_content\(&manifest, store_dir\)\? != manifest\.source_sha256\s*\{')
if not pat.search(text):
    raise SystemExit('missing Rust 1.98 collapsed let-chain for verify_store')
if re.search(r'if let Some\(store_dir\) = verify_store\s*\{\s*if hash_manifest_content', text, re.S):
    raise SystemExit('nested verify_store/hash if regressed')
PY
python3 - "$ROOT/hit-it-template.sh" <<'PY'
from pathlib import Path
import re, sys
text = Path(sys.argv[1]).read_text()
run_stage = re.search(r'run_stage\(\) \{(?P<body>.*?)\n\}', text, re.S)
if not run_stage:
    raise SystemExit('missing run_stage')
body = run_stage.group('body')
if not re.search(r'if "\$@" >>"\$VERIFY" 2>&1; then.*?else\s*\n\s*local rc=\$\?', body, re.S):
    raise SystemExit('run_stage does not capture failing command status inside else')
if re.search(r'fi\s*\n\s*local rc=\$\?', body):
    raise SystemExit('run_stage loses failing status by reading $? after fi')
PY
printf '%s\n' 'FORGECLEAN_V0_5_1_HOST_FAILURES=PASS'
