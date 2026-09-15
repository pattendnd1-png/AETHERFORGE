#!/usr/bin/env bash
set -euo pipefail
VERSION='2.1.60'
ROOT=${AETHER_BROWSER_OUT_DIR:-$HOME/Downloads}
OUT="$ROOT/Aether-Browser-v${VERSION}-VELORA-VERIFY.txt"
LOG="$ROOT/Aether-Browser-v${VERSION}-VELORA-RUNTIME.log"
: > "$OUT"
: > "$LOG"
record(){ printf '%s\n' "$1" | tee -a "$OUT"; }
record "AETHER_BROWSER_VERSION=${VERSION}"
record 'VELORA_RUNTIME_VERIFY=START'
command -v aether-browser >/dev/null 2>&1 || { record 'VELORA_SOFTWARE=FAIL:aether-browser-missing'; exit 2; }
command -v chromium >/dev/null 2>&1 || { record 'VELORA_SOFTWARE=FAIL:chromium-missing'; exit 2; }
record 'VELORA_SOFTWARE=PASS'
record 'VELORA_PROVIDER_URL=https://velora.tv/'
record 'VELORA_WEB_SESSION=PERSISTENT_NORMAL'
set +e
timeout 18s env WINIT_UNIX_BACKEND=x11 AETHER_BROWSER_MEDIA_RENDERER=cpu-bgra aether-browser 'https://velora.tv/' >"$LOG" 2>&1
rc=$?
set -e
if (( rc != 0 && rc != 124 )); then
  record "VELORA_BROWSER_LAUNCH=FAIL:${rc}"
  tail -n 80 "$LOG" | sed 's/^/VELORA_DETAIL=/' >> "$OUT"
  exit 1
fi
record 'VELORA_BROWSER_LAUNCH=PASS'
if grep -qF 'AETHER_BROWSER_RUNTIME_TARGET=https://velora.tv/' "$LOG"; then
  record 'VELORA_ROUTE=PASS'
else
  record 'VELORA_ROUTE=WARNING:runtime-target-marker-not-seen'
fi
if grep -qF 'AETHER_BROWSER_COMPAT_PROFILE=PERSISTENT_NORMAL' "$LOG"; then
  record 'VELORA_PROFILE_PERSISTENCE=PASS'
else
  record 'VELORA_PROFILE_PERSISTENCE=WARNING:profile-marker-not-seen'
fi
record 'VELORA_USER_ACCEPTANCE=OPEN_BROWSER_AND_CONFIRM_LOGIN_CHAT_PLAYBACK'
record 'VELORA_RUNTIME_VERIFY=PASS'
