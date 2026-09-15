#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ENABLE_SERVICE=1
[[ "${1:-}" == "--no-enable-service" ]] && ENABLE_SERVICE=0
cd "$ROOT"

WAS_ACTIVE=0
if systemctl --user is-active --quiet forgehx-daemon.service 2>/dev/null; then
  WAS_ACTIVE=1
fi

if [[ ! -x target/release/forgehx || ! -x target/release/forgehx-gui || ! -x target/release/forgehx-daemon ]]; then
  command -v cargo >/dev/null || { echo 'cargo is required to build ForgeHX.' >&2; exit 1; }
  cargo build --workspace --release
fi

sudo install -Dm755 target/release/forgehx /usr/bin/forgehx
sudo install -Dm755 target/release/forgehx-gui /usr/bin/forgehx-gui
sudo install -Dm755 target/release/forgehx-daemon /usr/bin/forgehx-daemon
sudo install -Dm644 packaging/udev/70-forgehx.rules /usr/lib/udev/rules.d/70-forgehx.rules
sudo install -Dm644 packaging/systemd/forgehx-daemon.service /usr/lib/systemd/user/forgehx-daemon.service
sudo install -Dm644 packaging/systemd/forgehx-openrgb.service /usr/lib/systemd/user/forgehx-openrgb.service
sudo install -Dm644 packaging/desktop/io.forgehx.ForgeHX.desktop /usr/share/applications/io.forgehx.ForgeHX.desktop
sudo install -Dm755 scripts/refresh-haste-hid-access.sh /usr/lib/forgehx/refresh-haste-hid-access.sh
sudo /usr/lib/forgehx/refresh-haste-hid-access.sh || true
systemctl --user daemon-reload

if (( ENABLE_SERVICE )); then
  systemctl --user enable --now forgehx-daemon.service
  echo 'ForgeHX daemon enabled and started.'
elif (( WAS_ACTIVE )); then
  systemctl --user restart forgehx-daemon.service
  echo 'ForgeHX daemon was already running and has been restarted on the new binary.'
else
  echo 'Installed without enabling the daemon (--no-enable-service was requested).'
fi
