#!/usr/bin/env bash
set -euo pipefail
VERSION="1.0.1"
CLI_SRC="${HOME}/Downloads/ForgeClean-v${VERSION}-forgeclean"
GUI_SRC="${HOME}/Downloads/ForgeClean-v${VERSION}-forgeclean-gui"
DEST_DIR="${HOME}/.local/bin"
CLI_DEST="${DEST_DIR}/forgeclean"
GUI_DEST="${DEST_DIR}/forgeclean-gui"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
SERVICE_SRC="${SCRIPT_DIR}/forgeclean-organizer.service.in"
SERVICE_DIR="${HOME}/.config/systemd/user"
SERVICE_DEST="${SERVICE_DIR}/forgeclean-organizer.service"
DESKTOP_SRC="${SCRIPT_DIR}/forgeclean.desktop.in"
DESKTOP_DIR="${HOME}/.local/share/applications"
DESKTOP_DEST="${DESKTOP_DIR}/org.aetherforge.forgeclean.desktop"
VERIFY="${HOME}/Downloads/ForgeClean-v${VERSION}-VERIFY.txt"
log() { printf '%s\n' "$*" | tee -a "$VERIFY"; }

[[ -x "$CLI_SRC" ]] || { echo "FORGECLEAN_INSTALL=FAIL:BINARY_NOT_FOUND:$CLI_SRC"; exit 1; }
[[ -x "$GUI_SRC" ]] || { echo "FORGECLEAN_INSTALL=FAIL:GUI_BINARY_NOT_FOUND:$GUI_SRC"; exit 1; }
[[ -f "$SERVICE_SRC" ]] || { echo "FORGECLEAN_INSTALL=FAIL:SERVICE_TEMPLATE_NOT_FOUND:$SERVICE_SRC"; exit 1; }
[[ -f "$DESKTOP_SRC" ]] || { echo "FORGECLEAN_INSTALL=FAIL:DESKTOP_TEMPLATE_NOT_FOUND:$DESKTOP_SRC"; exit 1; }
command -v systemctl >/dev/null 2>&1 || { echo "FORGECLEAN_INSTALL=FAIL:SYSTEMCTL_NOT_FOUND"; exit 127; }
command -v loginctl >/dev/null 2>&1 || { echo "FORGECLEAN_INSTALL=FAIL:LOGINCTL_NOT_FOUND"; exit 127; }

mkdir -p "$DEST_DIR" "$SERVICE_DIR" "$DESKTOP_DIR" "${HOME}/Downloads/ForgeClean"
install -m 0755 "$CLI_SRC" "$CLI_DEST"
install -m 0755 "$GUI_SRC" "$GUI_DEST"
install -m 0644 "$SERVICE_SRC" "$SERVICE_DEST"
sed "s|@HOME@|${HOME}|g" "$DESKTOP_SRC" > "$DESKTOP_DEST"
chmod 0644 "$DESKTOP_DEST"

systemctl --user daemon-reload
systemctl --user enable forgeclean-organizer.service
systemctl --user restart forgeclean-organizer.service
systemctl --user is-enabled --quiet forgeclean-organizer.service
systemctl --user is-active --quiet forgeclean-organizer.service

log "FORGECLEAN_PERSISTENT_SERVICE=PASS"
log "FORGECLEAN_SERVICE_RESTARTED=PASS"
log "FORGECLEAN_SERVICE=forgeclean-organizer.service"

if [[ "$(loginctl show-user "$USER" -p Linger --value 2>/dev/null || true)" != "yes" ]]; then
  if loginctl enable-linger "$USER" >/dev/null 2>&1; then
    :
  elif command -v sudo >/dev/null 2>&1 && sudo loginctl enable-linger "$USER"; then
    :
  else
    log "FORGECLEAN_LINGER=FAIL"
    exit 1
  fi
fi

[[ "$(loginctl show-user "$USER" -p Linger --value 2>/dev/null || true)" == "yes" ]] || {
  log "FORGECLEAN_LINGER=FAIL:NOT_ENABLED"
  exit 1
}

"$GUI_DEST" --self-test >>"$VERIFY" 2>&1
log "FORGECLEAN_LINGER=PASS"
log "FORGECLEAN_GUI_INSTALL=PASS"
log "FORGECLEAN_DESKTOP_ENTRY=PASS"
log "FORGECLEAN_INSTALL=PASS"
log "FORGECLEAN_BINARY=$CLI_DEST"
log "FORGECLEAN_GUI_BINARY=$GUI_DEST"
log "FORGECLEAN_DESKTOP_FILE=$DESKTOP_DEST"
log "FORGECLEAN_ORGANIZER_ROOT=${HOME}/Downloads/ForgeClean"
