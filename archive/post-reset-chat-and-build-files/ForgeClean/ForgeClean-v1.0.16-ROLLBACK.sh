#!/usr/bin/env bash
set -Eeuo pipefail
BACKUP='/home/benji/.local/state/forgeclean-v1.0.16-20260915-170522'
systemctl --user disable --now forgeclean-pre-rebase.timer >/dev/null 2>&1 || true
for bin in forgeclean forgeclean-gui forgeclean-system; do
  if [[ -f "$BACKUP/bin-before/$bin" ]]; then
    install -m 0755 "$BACKUP/bin-before/$bin" "$HOME/.local/bin/$bin"
  fi
done
for unit in forgeclean-pre-rebase.service forgeclean-pre-rebase.timer; do
  if [[ -f "$BACKUP/systemd-before/$unit" ]]; then
    install -m 0644 "$BACKUP/systemd-before/$unit" "$HOME/.config/systemd/user/$unit"
  else
    rm -f "$HOME/.config/systemd/user/$unit"
  fi
done
systemctl --user daemon-reload
systemctl --user restart forgeclean-organizer.service >/dev/null 2>&1 || true
echo 'FORGECLEAN_V1_0_16_ROLLBACK=PASS'
echo 'ROLLBACK_SCOPE=BINARIES_AND_TIMER_ONLY_PERMANENTLY_DELETED_FILES_ARE_NOT_RECOVERABLE'
