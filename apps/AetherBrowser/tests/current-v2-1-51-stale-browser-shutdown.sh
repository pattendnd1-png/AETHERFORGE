#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_V2_1_52_STALE_BROWSER_SHUTDOWN=FAIL:$1" >&2; exit 1; }
for f in scripts/install-host.sh scripts/install-current-tree.sh; do
  grep -qF 'browser_live_pids()' "$f" || fail "live-pid-helper-missing:$f"
  grep -qF '$2 !~ /^Z/' "$f" || fail "zombie-filter-missing:$f"
  grep -qF 'ps -u "$UID" -o pid=,stat=,comm=' "$f" || fail "current-user-process-scan-missing:$f"
  if grep -qF 'pgrep -x aether-browser' "$f"; then
    fail "raw-pgrep-still-gates-shutdown:$f"
  fi
done
grep -qF 'AETHER_BROWSER_PREVERIFY_BROWSER_SHUTDOWN=PASS' scripts/install-host.sh || fail preverify-pass-marker
grep -qF 'AETHER_BROWSER_PREVERIFY_BROWSER_SHUTDOWN_ZOMBIES=IGNORED' scripts/install-host.sh || fail zombie-diagnostic-marker
echo 'AETHER_BROWSER_V2_1_52_STALE_BROWSER_SHUTDOWN=PASS'
