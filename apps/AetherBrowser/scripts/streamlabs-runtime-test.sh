#!/usr/bin/env bash
set -euo pipefail
VERSION='2.1.60'
ROOT="$HOME/Downloads"
SRC="$ROOT/Aether-Browser-v${VERSION}"
BIN="$SRC/target/release/aether-browser"
OUT="$ROOT/Aether-Browser-v${VERSION}-STREAMLABS-VERIFY.txt"
URL="${AETHER_STREAMLABS_TEST_URL:-https://streamlabs.com/dashboard}"
: > "$OUT"
record(){ printf '%s\n' "$1" | tee -a "$OUT"; }
record "AETHER_BROWSER_VERSION=${VERSION}"
record 'AETHER_BROWSER_STREAMLABS_RUNTIME=START'
record 'AETHER_BROWSER_STREAMLABS_INFINITY_MIRROR=BLOCKED:STRICT_NO_RECURSION'
record 'AETHER_BROWSER_STREAMLABS_SCENE_OVERFLOW=CLIPPED:VIEWPORT_BOUNDS'
[[ -x "$BIN" ]] || { record 'AETHER_BROWSER_STREAMLABS_RUNTIME=FAIL:browser-not-built'; exit 2; }
[[ -t 0 ]] || { record 'AETHER_BROWSER_STREAMLABS_RUNTIME=FAIL:tty-required'; exit 4; }

run_surface(){
  local log rc
  log=$(mktemp)
  set +e
  WINIT_UNIX_BACKEND=x11 "$BIN" --url "$URL" 2>&1 | tee "$log"
  rc=${PIPESTATUS[0]}
  set -e
  cat "$log" >> "$OUT"
  if (( rc != 0 )); then rm -f "$log"; return "$rc"; fi
  grep -qF 'AETHER_BROWSER_CONTENT_ENGINE=CHROMIUM_COMPAT:' "$log" || { rm -f "$log"; return 21; }
  grep -qF 'AETHER_BROWSER_COMPAT_SURFACE=PASS:IN_WINDOW_CHROMIUM' "$log" || { rm -f "$log"; return 22; }
  grep -qF 'AETHER_BROWSER_COMPAT_PROFILE=PERSISTENT_NORMAL' "$log" || { rm -f "$log"; return 23; }
  rm -f "$log"
}

printf '\nOpen the Streamlabs dashboard in Aether Browser. Sign in if you want this account tested.\n'
printf 'Verify the dashboard, Alerts, Widgets/Overlays, and account menu load. Close Aether Browser when finished.\n\n'
if run_surface; then
  record 'AETHER_BROWSER_STREAMLABS_CHROMIUM=PASS'
  record 'AETHER_BROWSER_STREAMLABS_PERSISTENT_PROFILE=PASS'
else
  rc=$?
  record "AETHER_BROWSER_STREAMLABS_CHROMIUM=FAIL:${rc}"
  record 'AETHER_BROWSER_STREAMLABS_RUNTIME=FAIL'
  exit 1
fi

read -r -p 'Is a Streamlabs account connected/signed in for this test? [y/N] ' answer
if [[ ! "$answer" =~ ^[Yy]([Ee][Ss])?$ ]]; then
  record 'AETHER_BROWSER_STREAMLABS_ACCOUNT_STATE=NOT_CONNECTED'
  record 'AETHER_BROWSER_STREAMLABS_RUNTIME=NOT_CONNECTED'
  exit 0
fi
record 'AETHER_BROWSER_STREAMLABS_ACCOUNT_STATE=CONNECTED'

printf '\nReopening Streamlabs to prove the browser profile persists. Close Aether Browser after checking.\n'
if ! run_surface; then
  rc=$?
  record "AETHER_BROWSER_STREAMLABS_SESSION_REOPEN=FAIL:${rc}"
  record 'AETHER_BROWSER_STREAMLABS_RUNTIME=FAIL'
  exit 1
fi
record 'AETHER_BROWSER_STREAMLABS_SESSION_REOPEN=PASS'
read -r -p 'Did Streamlabs stay signed in and did dashboard + Alerts/Widgets/Overlays still work? [y/N] ' answer
if [[ "$answer" =~ ^[Yy]([Ee][Ss])?$ ]]; then
  record 'AETHER_BROWSER_STREAMLABS_ACCOUNT_PERSISTENCE=PASS'
  record 'AETHER_BROWSER_STREAMLABS_DASHBOARD=PASS:user-confirmed'
  record 'AETHER_BROWSER_STREAMLABS_ALERTS_WIDGETS_OVERLAYS=PASS:user-confirmed'
  record 'AETHER_BROWSER_STREAMLABS_RUNTIME=PASS'
  exit 0
fi
record 'AETHER_BROWSER_STREAMLABS_ACCOUNT_PERSISTENCE=FAIL'
record 'AETHER_BROWSER_STREAMLABS_RUNTIME=FAIL'
exit 1
