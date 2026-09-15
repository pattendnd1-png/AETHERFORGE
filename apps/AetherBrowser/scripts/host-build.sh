#!/usr/bin/env bash
set -uo pipefail

VERSION='2.1.60'
TAG='V2_1_60'
ROOT="$HOME/Downloads"
PREFIX="Aether-Browser-v${VERSION}"
SRC="$ROOT/$PREFIX"
VERIFY_FILE="$ROOT/${PREFIX}-VERIFY.txt"
mkdir -p "$ROOT"
: > "$VERIFY_FILE"
record(){ printf '%s\n' "$1" | tee -a "$VERIFY_FILE"; }
finish(){ local rc=$?; if (( rc != 0 )) && ! grep -q "^AETHER_BROWSER_${TAG}_VERIFY=" "$VERIFY_FILE" 2>/dev/null; then record "AETHER_BROWSER_${TAG}_VERIFY=FAIL"; fi; }
trap finish EXIT

record "AETHER_BROWSER_VERSION=${VERSION}"
record 'AETHER_BROWSER_HOST_LAUNCHER=START'
ZIP=$(python3 - "$ROOT" "$PREFIX" <<'PY'
from pathlib import Path
import re,sys
root=Path(sys.argv[1]); prefix=sys.argv[2]
pat=re.compile(rf'^{re.escape(prefix)}-source(?: \(\d+\))?\.zip$')
c=[p for p in root.iterdir() if p.is_file() and pat.match(p.name)] if root.exists() else []
if c:
    exact=root/f'{prefix}-source.zip'
    print(exact if exact in c else max(c,key=lambda p:p.stat().st_mtime_ns))
PY
)
[[ -n "$ZIP" && -f "$ZIP" ]] || { record 'AETHER_BROWSER_SOURCE_ZIP=FAIL:not-found'; exit 2; }
record "AETHER_BROWSER_SOURCE_ZIP=PASS:$(basename "$ZIP")"
rm -rf "$SRC"
python3 - "$ZIP" "$ROOT" "$PREFIX" <<'PY' || { record 'AETHER_BROWSER_SOURCE_EXTRACT=FAIL'; exit 3; }
from pathlib import Path
import os,sys,zipfile
z=Path(sys.argv[1]); out=Path(sys.argv[2]).resolve(); prefix=sys.argv[3]+'/'
with zipfile.ZipFile(z) as arc:
    infos=arc.infolist()
    if not infos or any(not i.filename.startswith(prefix) for i in infos): raise SystemExit('archive root mismatch')
    for i in infos:
        d=(out/i.filename).resolve()
        if out != d and out not in d.parents: raise SystemExit('unsafe archive path')
    arc.extractall(out)
    for i in infos:
        if i.is_dir(): continue
        mode=(i.external_attr>>16)&0o7777
        if mode: os.chmod(out/i.filename,mode)
PY
record 'AETHER_BROWSER_SOURCE_EXTRACT=PASS:unix-modes-restored'

cd "$SRC" || { record 'AETHER_BROWSER_SOURCE_CD=FAIL'; exit 4; }
record 'AETHER_BROWSER_TWITCH_RESOLVER=DEFERRED:streamlink-installed-during-promotion'

record 'AETHER_BROWSER_MEDIA_SAFE_FETCH=START'
cargo fetch 2>&1 | tee -a "$VERIFY_FILE" || { record 'AETHER_BROWSER_MEDIA_SAFE_FETCH=FAIL'; exit 6; }
record 'AETHER_BROWSER_MEDIA_SAFE_FETCH=PASS'
record 'AETHER_BROWSER_MEDIA_SAFE_PREPARE=START'
AETHER_BROWSER_MEDIA_RENDERER=cpu-bgra bash scripts/prepare-media-safe-renderer.sh 2>&1 | tee -a "$VERIFY_FILE" || { record 'AETHER_BROWSER_MEDIA_SAFE_PREPARE=FAIL'; exit 7; }
record 'AETHER_BROWSER_MEDIA_SAFE_PREPARE=PASS'
record 'AETHER_BROWSER_HOST_VERIFY=START'
AETHER_BROWSER_MEDIA_RENDERER=cpu-bgra AETHER_BROWSER_OUT_DIR="$ROOT" bash scripts/verify.sh
rc=$?
if (( rc == 0 )); then record 'AETHER_BROWSER_HOST_LAUNCHER=PASS'; else record "AETHER_BROWSER_HOST_LAUNCHER=FAIL:${rc}"; fi
exit "$rc"
