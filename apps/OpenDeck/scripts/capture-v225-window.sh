#!/usr/bin/env bash
set -euo pipefail
OUT="${1:?usage: capture-v225-window.sh OUTPUT.png EXPECTED_PID EXPECTED_RELEASE FOCUS_ACK.json [--diagnostic]}"
EXPECTED_PID="${2:-0}"
EXPECTED_RELEASE="${3:-2.0.25}"
FOCUS_ACK="${4:-}"
MODE="${5:-}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
command -v spectacle >/dev/null 2>&1 || { echo 'OPENDECK_V225_CAPTURE=FAIL:SPECTACLE_MISSING'; exit 2; }
RAW="${OUT%.png}.raw.png"
rm -f "$RAW" "$OUT"

activate_candidate_window(){
  local method="tauri-focus-ack"
  if command -v kdotool >/dev/null 2>&1; then
    local wid active active_pid
    wid="$(kdotool search --pid "$EXPECTED_PID" 2>/dev/null | head -n1 || true)"
    if [[ -n "$wid" ]]; then
      kdotool windowactivate "$wid" >/dev/null 2>&1 || true
      sleep .20
      active="$(kdotool getactivewindow 2>/dev/null | tail -n1 || true)"
      active_pid="$(kdotool getwindowpid "$active" 2>/dev/null | tail -n1 || true)"
      [[ "$active_pid" == "$EXPECTED_PID" ]] || { echo "OPENDECK_V225_CAPTURE=FAIL:ACTIVE_PID_KDOTOOL:$active_pid:$EXPECTED_PID"; return 1; }
      method="kdotool"
    fi
  elif command -v xdotool >/dev/null 2>&1; then
    local wid active active_pid
    wid="$(xdotool search --pid "$EXPECTED_PID" 2>/dev/null | head -n1 || true)"
    if [[ -n "$wid" ]]; then
      xdotool windowactivate --sync "$wid" >/dev/null 2>&1 || true
      sleep .20
      active="$(xdotool getactivewindow 2>/dev/null || true)"
      active_pid="$(xdotool getwindowpid "$active" 2>/dev/null || true)"
      [[ "$active_pid" == "$EXPECTED_PID" ]] || { echo "OPENDECK_V225_CAPTURE=FAIL:ACTIVE_PID_XDOTOOL:$active_pid:$EXPECTED_PID"; return 1; }
      method="xdotool"
    fi
  fi
  echo "OPENDECK_V225_CAPTURE_FOCUS_METHOD=$method"
}

if [[ "$MODE" == "--diagnostic" && "$EXPECTED_PID" =~ ^[0-9]+$ && "$EXPECTED_PID" -gt 1 ]]; then
  activate_candidate_window || true
  sleep .20
fi

if [[ "$MODE" != "--diagnostic" ]]; then
  [[ "$EXPECTED_PID" =~ ^[0-9]+$ && "$EXPECTED_PID" -gt 1 ]] || { echo 'OPENDECK_V225_CAPTURE=FAIL:EXPECTED_PID'; exit 5; }
  [[ -s "$FOCUS_ACK" ]] || { echo 'OPENDECK_V225_CAPTURE=FAIL:FOCUS_ACK_MISSING'; exit 6; }
  python3 - "$FOCUS_ACK" "$EXPECTED_PID" "$EXPECTED_RELEASE" <<'PY'
import json,sys
p,pid,release=sys.argv[1],int(sys.argv[2]),sys.argv[3]
d=json.load(open(p))
assert d.get('release')==release,(d,release)
assert int(d.get('pid',-1))==pid,(d,pid)
print(f'OPENDECK_V225_FOCUS_ACK=PASS:release={release}:pid={pid}')
PY
  activate_candidate_window || exit 7
  sleep .25
fi

spectacle -b -n -a -o "$RAW"
[[ -s "$RAW" ]] || { echo 'OPENDECK_V225_CAPTURE=FAIL:EMPTY'; exit 3; }
python3 "$ROOT/scripts/v225-normalize-capture.py" "$RAW" "$OUT"
rm -f "$RAW"
[[ -s "$OUT" ]] || { echo 'OPENDECK_V225_CAPTURE=FAIL:NORMALIZE_EMPTY'; exit 4; }
if [[ "$MODE" != "--diagnostic" ]]; then
  python3 "$ROOT/scripts/v225-validate-capture-identity.py" "$OUT" "$FOCUS_ACK" "$EXPECTED_PID" "$EXPECTED_RELEASE"
fi
echo "OPENDECK_V225_CAPTURE=PASS:$OUT"
