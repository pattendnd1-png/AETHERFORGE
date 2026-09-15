#!/usr/bin/env bash
set -uo pipefail

VERSION='2.1.10'
TAG='V2_1_10'
DOWNLOADS="$HOME/Downloads"
PREFIX="Aether-Browser-v${VERSION}"
SRC="$DOWNLOADS/$PREFIX"
BOOTSTRAP_VERIFY="$DOWNLOADS/${PREFIX}-INSTALL-BOOTSTRAP.txt"
VERIFY_FILE="$DOWNLOADS/${PREFIX}-VERIFY.txt"
INSTALL_VERIFY="$DOWNLOADS/${PREFIX}-INSTALL-VERIFY.txt"
CURRENT_STAGE='bootstrap'
AETHER_BROWSER_INSTALL_LOCK="${XDG_RUNTIME_DIR:-/tmp}/aether-browser-v${VERSION}-install.lock"
command -v flock >/dev/null 2>&1 || { printf 'AETHER_BROWSER_INSTALL_LOCK=FAIL:missing-flock\n'; exit 72; }
exec 9>"$AETHER_BROWSER_INSTALL_LOCK"
if ! flock -n 9; then
  printf 'AETHER_BROWSER_INSTALL_LOCK=BUSY:%s\n' "$AETHER_BROWSER_INSTALL_LOCK"
  exit 73
fi
printf 'AETHER_BROWSER_INSTALL_LOCK=ACQUIRED:%s\n' "$AETHER_BROWSER_INSTALL_LOCK"
mkdir -p "$DOWNLOADS"
: > "$BOOTSTRAP_VERIFY"
printf 'AETHER_BROWSER_VERSION=%s\nAETHER_BROWSER_VERIFY=START\n' "$VERSION" > "$VERIFY_FILE"
printf 'AETHER_BROWSER_VERSION=%s\nAETHER_BROWSER_INSTALL_VERIFY=START\n' "$VERSION" > "$INSTALL_VERIFY"
record(){ printf '%s\n' "$1" | tee -a "$BOOTSTRAP_VERIFY"; }
fail(){ record "$1"; exit "${2:-1}"; }
append_diag(){ local target="$1"; printf 'AETHER_BROWSER_DIAG_BEGIN=%s\n' "$CURRENT_STAGE" >> "$target"; tail -n 120 "$BOOTSTRAP_VERIFY" >> "$target" 2>/dev/null || true; printf 'AETHER_BROWSER_DIAG_END=%s\n' "$CURRENT_STAGE" >> "$target"; }
on_exit(){
  local rc=$?
  if (( rc != 0 )); then
    if ! grep -Eq '^AETHER_BROWSER_V2_1_10_VERIFY=(PASS|FAIL)' "$VERIFY_FILE" 2>/dev/null; then
      printf 'AETHER_BROWSER_VERSION=%s\nAETHER_BROWSER_V2_1_10_VERIFY=FAIL_EARLY:%s:exit=%s\nAETHER_BROWSER_FAILURE_STAGE=%s\n' "$VERSION" "$CURRENT_STAGE" "$rc" "$CURRENT_STAGE" >> "$VERIFY_FILE"
    fi
    if ! grep -Eq '^AETHER_BROWSER_V2_1_10_INSTALL_VERIFY=(PASS|FAIL)' "$INSTALL_VERIFY" 2>/dev/null; then
      printf 'AETHER_BROWSER_VERSION=%s\nAETHER_BROWSER_V2_1_10_INSTALL_VERIFY=FAIL_EARLY:%s:exit=%s\nAETHER_BROWSER_FAILURE_STAGE=%s\n' "$VERSION" "$CURRENT_STAGE" "$rc" "$CURRENT_STAGE" >> "$INSTALL_VERIFY"
    fi
    append_diag "$VERIFY_FILE"; append_diag "$INSTALL_VERIFY"
    printf 'AETHER_BROWSER_VERIFY_FILE=%s\nAETHER_BROWSER_INSTALL_VERIFY_FILE=%s\n' "$VERIFY_FILE" "$INSTALL_VERIFY"
  fi
}
trap on_exit EXIT
record "AETHER_BROWSER_VERSION=${VERSION}"
record 'AETHER_BROWSER_INSTALL_BOOTSTRAP=START'

CURRENT_STAGE='source-discovery'
ZIP=$(python3 - "$DOWNLOADS" "$PREFIX" <<'PY'
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
[[ -n "$ZIP" && -f "$ZIP" ]] || fail 'AETHER_BROWSER_SOURCE_ZIP=FAIL:not-found' 2
record "AETHER_BROWSER_SOURCE_ZIP=PASS:$(basename "$ZIP")"

CURRENT_STAGE='source-extract'
rm -rf "$SRC"
python3 - "$ZIP" "$DOWNLOADS" "$PREFIX" <<'PY' || fail 'AETHER_BROWSER_SOURCE_EXTRACT=FAIL' 3
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

CURRENT_STAGE='source-cd'
cd "$SRC" || fail 'AETHER_BROWSER_SOURCE_CD=FAIL' 4
TOOLS_ROOT="$SRC/.aether-tools"
TWITCH_HELPER="$TOOLS_ROOT/bin/twitch-hls-client"
mkdir -p "$TOOLS_ROOT/bin"
CURRENT_STAGE='twitch-helper'
record 'AETHER_BROWSER_TWITCH_NATIVE_HELPER=START:fresh-install'
cargo install twitch-hls-client --version 1.8.0 --locked --root "$TOOLS_ROOT" 2>&1 | tee -a "$BOOTSTRAP_VERIFY" || fail 'AETHER_BROWSER_TWITCH_NATIVE_HELPER=FAIL:install' 5
[[ -x "$TWITCH_HELPER" ]] || fail 'AETHER_BROWSER_TWITCH_NATIVE_HELPER=FAIL:missing-binary' 5
record 'AETHER_BROWSER_TWITCH_NATIVE_HELPER=PASS:1.8.0:fresh'
export AETHER_TWITCH_HLS_CLIENT="$TWITCH_HELPER"

CURRENT_STAGE='cargo-fetch'
cargo fetch 2>&1 | tee -a "$BOOTSTRAP_VERIFY" || fail 'AETHER_BROWSER_FETCH=FAIL' 6
CURRENT_STAGE='media-prepare'
AETHER_BROWSER_MEDIA_RENDERER=cpu-bgra bash scripts/prepare-media-safe-renderer.sh 2>&1 | tee -a "$BOOTSTRAP_VERIFY" || fail 'AETHER_BROWSER_MEDIA_SAFE_PREPARE=FAIL' 7
CURRENT_STAGE='aetherai-service-build'
record 'AETHER_BROWSER_AETHERAI_SERVICE_BUILD=START'
AETHER_BROWSER_OUT_DIR="$DOWNLOADS" bash scripts/build-aetherai-browser-service.sh 2>&1 | tee -a "$BOOTSTRAP_VERIFY" || fail 'AETHER_BROWSER_AETHERAI_SERVICE_BUILD=FAIL' 71
record 'AETHER_BROWSER_AETHERAI_SERVICE_BUILD=PASS'

CURRENT_STAGE='host-verify'
record 'AETHER_BROWSER_HOST_VERIFY=START'
AETHER_BROWSER_MEDIA_RENDERER=cpu-bgra AETHER_BROWSER_OUT_DIR="$DOWNLOADS" bash scripts/verify.sh || fail "AETHER_BROWSER_${TAG}_VERIFY=FAIL" 8
record "AETHER_BROWSER_${TAG}_VERIFY=PASS"

CURRENT_STAGE='native-install'
record 'AETHER_BROWSER_NATIVE_INSTALL=START'
AETHER_BROWSER_OUT_DIR="$DOWNLOADS" bash scripts/install-current-tree.sh 2>&1 | tee -a "$BOOTSTRAP_VERIFY"
native_install_rc=${PIPESTATUS[0]}
(( native_install_rc == 0 )) || fail "AETHER_BROWSER_NATIVE_INSTALL=FAIL:exit=${native_install_rc}" 9
CURRENT_STAGE='complete'
record 'AETHER_BROWSER_INSTALL_BOOTSTRAP=PASS'
exit 0
