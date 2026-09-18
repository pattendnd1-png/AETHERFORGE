#!/usr/bin/env bash
set -Eeuo pipefail

MODE="${1:?mode required: staged|canonical}"
EXPECTED_BIN="${2:-/opt/opendeck-plus/2.0.45/bin/opendeck-studio}"
EXPECTED_REAL="$(readlink -f "$EXPECTED_BIN" 2>/dev/null || true)"
SYSTEM_APP_DIR="${OPENDECK_SYSTEM_APP_DIR:-/usr/share/applications}"
TARGET_VERSION="2.0.45"

[[ "$MODE" == "staged" || "$MODE" == "canonical" ]] || { echo "OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:MODE:$MODE"; exit 2; }
[[ -n "$EXPECTED_REAL" && -x "$EXPECTED_REAL" ]] || { echo 'OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:EXPECTED_BINARY'; exit 2; }
command -v gtk-launch >/dev/null 2>&1 || { echo 'OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:GTK_LAUNCH_MISSING'; exit 3; }
command -v desktop-file-validate >/dev/null 2>&1 || { echo 'OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:DESKTOP_VALIDATE_MISSING'; exit 3; }

DESKTOP_ID="opendeck-studio"
TEMP_SYSTEM_FILE=""
LOCAL_TEMP=""
if [[ "$MODE" == "staged" ]]; then
  DESKTOP_ID="opendeck-v245-system-smoke"
  TEMP_SYSTEM_FILE="$SYSTEM_APP_DIR/$DESKTOP_ID.desktop"
  LOCAL_TEMP="/tmp/$DESKTOP_ID-$$.desktop"
  cat > "$LOCAL_TEMP" <<EOF
[Desktop Entry]
Type=Application
Name=OpenDeck+ 2.0.45 System Install Smoke
Exec=$EXPECTED_REAL
Icon=opendeck-studio
Terminal=false
Categories=Utility;
NoDisplay=true
StartupNotify=true
EOF
  desktop-file-validate "$LOCAL_TEMP" || { echo 'OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:TEMP_DESKTOP_INVALID'; exit 4; }
  sudo install -m 0644 "$LOCAL_TEMP" "$TEMP_SYSTEM_FILE"
  command -v update-desktop-database >/dev/null 2>&1 && sudo update-desktop-database "$SYSTEM_APP_DIR" >/dev/null 2>&1 || true
fi

LOG_FILE="/tmp/opendeck-v245-system-menu-$MODE.log"
PROBE_FILE="/tmp/opendeck-v245-system-menu-$MODE-$$.json"
new_pid=""
cleanup(){
  if [[ -n "$new_pid" ]]; then
    kill "$new_pid" >/dev/null 2>&1 || true
    for _ in {1..40}; do kill -0 "$new_pid" >/dev/null 2>&1 || break; sleep .05; done
    kill -9 "$new_pid" >/dev/null 2>&1 || true
    wait "$new_pid" 2>/dev/null || true
  fi
  rm -f -- "$PROBE_FILE" "$LOCAL_TEMP"
  if [[ -n "$TEMP_SYSTEM_FILE" ]]; then
    sudo rm -f -- "$TEMP_SYSTEM_FILE" || true
    command -v update-desktop-database >/dev/null 2>&1 && sudo update-desktop-database "$SYSTEM_APP_DIR" >/dev/null 2>&1 || true
  fi
  if command -v kbuildsycoca6 >/dev/null 2>&1; then kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
  elif command -v kbuildsycoca5 >/dev/null 2>&1; then kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if command -v kbuildsycoca6 >/dev/null 2>&1; then kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

before="$(pgrep -x opendeck-studio 2>/dev/null || true)"
: > "$LOG_FILE"
OPENDECK_STARTUP_PROBE_FILE="$PROBE_FILE" gtk-launch "$DESKTOP_ID" >"$LOG_FILE" 2>&1 || {
  cat "$LOG_FILE" || true
  echo "OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:GTK_LAUNCH:$MODE"
  exit 5
}

is_old(){ local p="$1"; grep -qx "$p" <<<"$before"; }
for _ in {1..200}; do
  while read -r p; do
    [[ -n "$p" ]] || continue
    if ! is_old "$p"; then new_pid="$p"; break; fi
  done < <(pgrep -x opendeck-studio 2>/dev/null || true)
  [[ -n "$new_pid" ]] && break
  sleep .05
done
[[ -n "$new_pid" ]] || { cat "$LOG_FILE" || true; echo "OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:NO_NEW_PROCESS:$MODE"; exit 6; }

actual_real="$(readlink -f "/proc/$new_pid/exe" 2>/dev/null || true)"
[[ "$actual_real" == "$EXPECTED_REAL" ]] || {
  echo "OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:WRONG_BINARY:$actual_real:$EXPECTED_REAL:$MODE"
  exit 7
}

for _ in {1..300}; do
  if [[ -s "$PROBE_FILE" ]] && python3 - "$PROBE_FILE" "$new_pid" <<'PY' >/dev/null 2>&1
import json, sys
p=sys.argv[1]; pid=int(sys.argv[2])
d=json.load(open(p))
assert d.get('release') == '2.0.45'
assert d.get('pid') == pid
assert d.get('nativeVisible') is True
assert d.get('frontendVisible') is True
PY
  then
    echo "OPENDECK_V245_SYSTEM_MENU_LAUNCH=PASS:mode=$MODE:pid=$new_pid:binary=$actual_real"
    exit 0
  fi
  kill -0 "$new_pid" >/dev/null 2>&1 || { cat "$LOG_FILE" || true; echo "OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:PROCESS_EXITED:$MODE"; exit 8; }
  sleep .05
done
cat "$LOG_FILE" || true
echo "OPENDECK_V245_SYSTEM_MENU_LAUNCH=FAIL:NO_VISIBLE_STARTUP_ACK:mode=$MODE:pid=$new_pid"
exit 9
