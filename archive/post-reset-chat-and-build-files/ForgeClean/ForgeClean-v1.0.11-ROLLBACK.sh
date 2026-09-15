#!/usr/bin/env bash
set -Eeuo pipefail
BACKUP='/home/benji/.local/state/forgeclean-v1.0.11-20260915-125601/bin-before'
for bin in forgeclean forgeclean-gui forgeclean-system; do
  if [[ -f "$BACKUP/$bin" ]]; then
    install -m 0755 "$BACKUP/$bin" "$HOME/.local/bin/$bin"
  fi
done
echo 'FORGECLEAN_V1_0_11_ROLLBACK=PASS'
