#!/usr/bin/env bash
set -euo pipefail
OUT="$HOME/Downloads/Aether-Browser-UNINSTALL-VERIFY.txt"
: > "$OUT"
record() { printf '%s\n' "$1" | tee -a "$OUT"; }
systemctl --user disable --now aether-browser-media.service >/dev/null 2>&1 || true
echo 'AETHER_BROWSER_SUDO_AUTH=START'
sudo -v
echo 'AETHER_BROWSER_SUDO_AUTH=PASS'
sudo -n pacman -Rns --noconfirm aether-browser
systemctl --user daemon-reload
if pacman -Q aether-browser >/dev/null 2>&1; then
  record 'AETHER_BROWSER_UNINSTALL_VERIFY=FAIL:package-still-installed'
  exit 1
fi
record 'AETHER_BROWSER_UNINSTALL_VERIFY=PASS'
