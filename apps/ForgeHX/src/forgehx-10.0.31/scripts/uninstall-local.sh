#!/usr/bin/env bash
set -euo pipefail
systemctl --user disable --now forgehx-daemon.service forgehx-openrgb.service 2>/dev/null || true
sudo rm -f /usr/bin/forgehx /usr/bin/forgehx-gui /usr/bin/forgehx-daemon
sudo rm -f /usr/lib/udev/rules.d/70-forgehx.rules
sudo rm -f /usr/lib/systemd/user/forgehx-daemon.service /usr/lib/systemd/user/forgehx-openrgb.service
sudo rm -f /usr/share/applications/io.forgehx.ForgeHX.desktop
sudo udevadm control --reload-rules || true
systemctl --user daemon-reload || true
echo 'ForgeHX removed. User profiles under ~/.config/forgehx were preserved.'
