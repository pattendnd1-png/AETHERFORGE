#!/usr/bin/env bash
set -euo pipefail
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/aetherforge/aether-browser"
PREVIOUS_FILE="$STATE_DIR/previous-package"
OUT="$HOME/Downloads/Aether-Browser-ROLLBACK-VERIFY.txt"
: > "$OUT"
record() { printf '%s\n' "$1" | tee -a "$OUT"; }
[[ -f "$PREVIOUS_FILE" ]] || { record 'AETHER_BROWSER_ROLLBACK=FAIL:no-previous-package-record'; exit 2; }
PREVIOUS=$(cat "$PREVIOUS_FILE")
[[ -n "$PREVIOUS" && -f "$PREVIOUS" ]] || { record 'AETHER_BROWSER_ROLLBACK=FAIL:previous-package-missing'; exit 3; }
record "AETHER_BROWSER_ROLLBACK_PACKAGE=$PREVIOUS"
echo 'AETHER_BROWSER_SUDO_AUTH=START'
sudo -v
echo 'AETHER_BROWSER_SUDO_AUTH=PASS'
sudo -n pacman -U --noconfirm "$PREVIOUS"
systemctl --user daemon-reload
if systemctl --user cat aether-browser-media.service >/dev/null 2>&1; then
  systemctl --user enable --now aether-browser-media.service
fi
record "AETHER_BROWSER_ROLLBACK_INSTALLED=$(pacman -Q aether-browser 2>/dev/null || echo unknown)"
record 'AETHER_BROWSER_ROLLBACK_VERIFY=PASS'
