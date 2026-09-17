#!/usr/bin/env bash
set -Eeuo pipefail

EXPECTED_BIN="${1:?candidate binary path required}"
EXPECTED_REAL="$(readlink -f "$EXPECTED_BIN" 2>/dev/null || true)"
[[ -n "$EXPECTED_REAL" && -x "$EXPECTED_REAL" ]] || { echo 'OPENDECK_V234_MENU_LAUNCH=FAIL:EXPECTED_BINARY'; exit 2; }
command -v gtk-launch >/dev/null 2>&1 || { echo 'OPENDECK_V234_MENU_LAUNCH=FAIL:GTK_LAUNCH_MISSING'; exit 3; }

DESKTOP_DIR="$HOME/.local/share/applications"
DESKTOP_ID="opendeck-v234-menu-smoke"
DESKTOP_FILE="$DESKTOP_DIR/$DESKTOP_ID.desktop"
LOG_FILE="/tmp/opendeck-v234-menu-launch.log"
mkdir -p "$DESKTOP_DIR"
new_pid=""
cleanup(){
  if [[ -n "$new_pid" ]]; then
    kill "$new_pid" >/dev/null 2>&1 || true
    for _ in {1..40}; do kill -0 "$new_pid" >/dev/null 2>&1 || break; sleep .05; done
    kill -9 "$new_pid" >/dev/null 2>&1 || true
    wait "$new_pid" 2>/dev/null || true
  fi
  rm -f -- "$DESKTOP_FILE"
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
  if command -v kbuildsycoca6 >/dev/null 2>&1; then kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
  elif command -v kbuildsycoca5 >/dev/null 2>&1; then kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=OpenDeck+ 2.0.34 Menu Smoke
Exec=$EXPECTED_REAL
Icon=opendeck-studio
Terminal=false
Categories=Utility;
NoDisplay=true
StartupNotify=true
EOF
if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$DESKTOP_FILE" || { echo 'OPENDECK_V234_MENU_LAUNCH=FAIL:TEMP_DESKTOP_INVALID'; exit 4; }
fi
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DESKTOP_DIR" >/dev/null 2>&1 || true
if command -v kbuildsycoca6 >/dev/null 2>&1; then kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif command -v kbuildsycoca5 >/dev/null 2>&1; then kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

before="$(pgrep -x opendeck-studio 2>/dev/null || true)"
: > "$LOG_FILE"
gtk-launch "$DESKTOP_ID" >"$LOG_FILE" 2>&1 || {
  cat "$LOG_FILE" || true
  echo 'OPENDECK_V234_MENU_LAUNCH=FAIL:GTK_LAUNCH'
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
[[ -n "$new_pid" ]] || { cat "$LOG_FILE" || true; echo 'OPENDECK_V234_MENU_LAUNCH=FAIL:NO_NEW_PROCESS'; exit 6; }

actual_real="$(readlink -f "/proc/$new_pid/exe" 2>/dev/null || true)"
[[ "$actual_real" == "$EXPECTED_REAL" ]] || {
  echo "OPENDECK_V234_MENU_LAUNCH=FAIL:WRONG_BINARY:$actual_real:$EXPECTED_REAL"
  exit 7
}

visible=""
if command -v kdotool >/dev/null 2>&1; then
  for _ in {1..160}; do
    visible="$(kdotool search --pid "$new_pid" 2>/dev/null | head -n1 || true)"
    [[ -n "$visible" ]] && break
    sleep .05
  done
elif command -v xdotool >/dev/null 2>&1; then
  for _ in {1..160}; do
    visible="$(xdotool search --onlyvisible --pid "$new_pid" 2>/dev/null | head -n1 || true)"
    [[ -n "$visible" ]] && break
    sleep .05
  done
else
  echo 'OPENDECK_V234_MENU_LAUNCH=FAIL:WINDOW_VISIBILITY_TOOL_MISSING'
  exit 8
fi

[[ -n "$visible" ]] || {
  cat "$LOG_FILE" || true
  echo "OPENDECK_V234_MENU_LAUNCH=FAIL:NO_VISIBLE_WINDOW:pid=$new_pid"
  exit 9
}

echo "OPENDECK_V234_MENU_LAUNCH=PASS:pid=$new_pid:window=$visible:binary=$actual_real"
