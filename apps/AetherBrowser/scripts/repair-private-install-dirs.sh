#!/usr/bin/env bash
set -euo pipefail

for d in \
  /usr/lib/aetherforge/aether-browser \
  /usr/lib/aetherforge/aether-browser/bin \
  /usr/lib/aetherforge/aether-browser/tools \
  /usr/share/licenses/aether-browser; do
  if [[ -d "$d" ]]; then
    sudo -n chown 0:0 "$d"
    sudo -n chmod 0755 "$d"
  fi
done
