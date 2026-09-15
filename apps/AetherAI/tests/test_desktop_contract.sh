#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
BIN="$ROOT/target/release/aetherai-desktop"
[[ -x "$BIN" ]] || { echo AETHERAI_DESKTOP_CONTRACT=FAIL:NO_BINARY; exit 1; }
out=$("$BIN" --data-dir "$ROOT/.aetherai/desktop-contract" --diagnostic)
grep -qx 'AETHERAI_DESKTOP=LIVE' <<<"$out"
grep -qx 'AETHERAI_THEME=DRAGONGLASS' <<<"$out"
grep -qx 'AETHERAI_RUNTIME=LIVE' <<<"$out"
! grep -R --line-number --fixed-strings 'DesktopPage::Work' apps/aetherai-desktop/src >/dev/null 2>&1
echo AETHERAI_DESKTOP_CONTRACT=PASS
