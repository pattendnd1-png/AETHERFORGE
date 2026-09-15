#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
TAKE="$ROOT/scripts/browser-takeover.sh"
ROLL="$ROOT/scripts/browser-takeover-rollback.sh"
fail(){ echo "AETHER_BROWSER_V2_1_60_BROWSER_TAKEOVER=FAIL:$1"; exit 1; }
for f in "$TAKE" "$ROLL"; do grep -qF "VERSION='2.1.60'" "$f" || fail version-missing; done
for assoc in 'x-scheme-handler/http' 'x-scheme-handler/https' 'text/html' 'application/xhtml+xml' 'application/pdf' 'x-scheme-handler/ftp'; do
  grep -qF "$assoc" "$TAKE" || fail "takeover-association-missing:$assoc"
  grep -qF "$assoc" "$ROLL" || fail "rollback-association-missing:$assoc"
done
grep -qF 'AETHER_BROWSER_TAKEOVER_ASSOCIATIONS=PASS' "$TAKE" || fail association-pass-marker-missing
grep -qF 'AETHER_BROWSER_TAKEOVER_DEPENDENCY_SAFE=PASS' "$TAKE" || fail dependency-safe-marker-missing
grep -qF 'AETHER_BROWSER_TAKEOVER_PURGE=SKIP:not-requested' "$TAKE" || fail purge-default-not-safe
grep -qF 'AETHER_BROWSER_TAKEOVER_ROLLBACK_ASSOCIATIONS=PASS' "$ROLL" || fail rollback-pass-marker-missing
if grep -qE 'pacman -Rns' "$TAKE" && ! grep -qF 'AETHER_BROWSER_TAKEOVER_PURGE_ACK' "$TAKE"; then
  fail purge-not-explicit
fi

HOST_GATE="$ROOT/scripts/package-consolidated.sh"
grep -qF 'browser-takeover.sh" --activate' "$HOST_GATE" || fail host-gate-does-not-activate-takeover
grep -qF 'AETHER_BROWSER_HOST_TAKEOVER=PASS' "$HOST_GATE" || fail host-takeover-pass-marker-missing

echo 'AETHER_BROWSER_V2_1_60_BROWSER_TAKEOVER=PASS'
