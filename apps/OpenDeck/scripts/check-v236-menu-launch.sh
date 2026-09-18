#!/usr/bin/env bash
set -Eeuo pipefail

EXPECTED_BIN="${1:?candidate binary path required}"
EXPECTED_REAL="$(readlink -f "$EXPECTED_BIN" 2>/dev/null || true)"
[[ -n "$EXPECTED_REAL" && -x "$EXPECTED_REAL" ]] || { echo 'OPENDECK_V236_MENU_LAUNCH=FAIL:EXPECTED_BINARY'; exit 2; }
command -v gtk-launch >/dev/null 2>&1 || { echo 'OPENDECK_V236_MENU_LAUNCH=FAIL:GTK_LAUNCH_MISSING'; exit 3; }

DESKTOP_DIR="$HOME/.local/share/applications"
DESKTOP_ID="opendeck-v236-menu-smoke"
DESKTOP_FILE="$DESKTOP_DIR/$DESKTOP_ID.desktop"
LOG_FILE="/tmp/opendeck-v236-menu-launch.log"
PROBE_FILE="/tmp/opendeck-v236-menu-visible-$$.json"
mkdir -p "$DESKTOP_DIR"
new_pid=""
cleanup(){
  if [[ -n "$new_pid" ]]; then
    kill "$new_pid" >/dev/null 2>&1 || true
    for _ in {1..40}; do kill -0 "$new_pid" >/dev/null 2>&1 || break; sleep .05; done
    kill -9 "$new_pid" >/dev/null 2>&1 || true
    wait "$new_pid" 2>/dev/null || true
  fi
  rm -f -- "$DESKTOP_FILE" "$PROBE_FILE"
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
  if command -v kbuildsycoca6 >/dev/null 2>&1; then kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
  elif command -v kbuildsycoca5 >/dev/null 2>&1; then kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=OpenDeck+ 2.0.36 Menu Smoke
Exec=env OPENDECK_STARTUP_PROBE_FILE=$PROBE_FILE $EXPECTED_REAL
Icon=opendeck-studio
Terminal=false
Categories=Utility;
NoDisplay=true
StartupNotify=true
EOF
if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$DESKTOP_FILE" || { echo 'OPENDECK_V236_MENU_LAUNCH=FAIL:TEMP_DESKTOP_INVALID'; exit 4; }
fi
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
if command -v kbuildsycoca6 >/dev/null 2>&1; then kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

before="$(pgrep -x opendeck-studio 2>/dev/null || true)"
: > "$LOG_FILE"
gtk-launch "$DESKTOP_ID" >"$LOG_FILE" 2>&1 || {
  cat "$LOG_FILE" || true
  echo 'OPENDECK_V236_MENU_LAUNCH=FAIL:GTK_LAUNCH'
  exit 5
}

is_old(){ local p="$1"; grep -qx "$p" <<<"$before"; }
for _ in {1..160}; do
  while read -r p; do
    [[ -n "$p" ]] || continue
    if ! is_old "$p"; then new_pid="$p"; break; fi
  done < <(pgrep -x opendeck-studio 2>/dev/null || true)
  [[ -n "$new_pid" ]] && break
  sleep .05
done
[[ -n "$new_pid" ]] || { cat "$LOG_FILE" || true; echo 'OPENDECK_V236_MENU_LAUNCH=FAIL:NO_NEW_PROCESS'; exit 6; }

actual_real="$(readlink -f "/proc/$new_pid/exe" 2>/dev/null || true)"
[[ "$actual_real" == "$EXPECTED_REAL" ]] || {
  echo "OPENDECK_V236_MENU_LAUNCH=FAIL:WRONG_BINARY:$actual_real:$EXPECTED_REAL"
  exit 7
}

for _ in {1..240}; do
  if [[ -s "$PROBE_FILE" ]] && python3 - "$PROBE_FILE" "$new_pid" <<'PY' >/dev/null 2>&1
import json, sys
p=sys.argv[1]; pid=int(sys.argv[2])
d=json.load(open(p))
assert d.get('release') == '2.0.36'
assert d.get('pid') == pid
assert d.get('nativeVisible') is True
assert d.get('frontendVisible') is True
PY
  then
    echo "OPENDECK_V236_MENU_LAUNCH=PASS:pid=$new_pid:probe=$PROBE_FILE:binary=$actual_real"
    exit 0
  fi
  kill -0 "$new_pid" >/dev/null 2>&1 || { cat "$LOG_FILE" || true; echo 'OPENDECK_V236_MENU_LAUNCH=FAIL:PROCESS_EXITED'; exit 8; }
  sleep .05
done
cat "$LOG_FILE" || true
echo "OPENDECK_V236_MENU_LAUNCH=FAIL:NO_VISIBLE_STARTUP_ACK:pid=$new_pid"
exit 9
