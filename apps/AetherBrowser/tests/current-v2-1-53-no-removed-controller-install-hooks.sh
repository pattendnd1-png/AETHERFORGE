#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
INSTALL="$ROOT/scripts/install-current-tree.sh"

fail() { echo "AETHER_BROWSER_V2_1_53_NO_REMOVED_CONTROLLER_INSTALL_HOOKS=FAIL:$1"; exit 1; }

[[ -f "$INSTALL" ]] || fail 'missing-install-script'

if grep -Eq 'prewarm_elgato_751_cache|streamdeck|aether-deck|elgato-streamdeck' "$INSTALL"; then
  fail 'removed-controller-hook-still-referenced'
fi

echo 'AETHER_BROWSER_V2_1_53_NO_REMOVED_CONTROLLER_INSTALL_HOOKS=PASS'
