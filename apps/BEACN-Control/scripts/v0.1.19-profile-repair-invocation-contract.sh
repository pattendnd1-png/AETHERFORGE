#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-.}"
MAIN="$ROOT/src/main.rs"
INSTALL="$ROOT/INSTALL-AND-VERIFY.sh"

require() {
  local pattern="$1" file="$2" message="$3"
  if ! grep -Fq -- "$pattern" "$file"; then
    echo "CONTRACT_FAIL: $message" >&2
    exit 1
  fi
}

require '"--repair-output-profile"' "$MAIN" 'missing repair CLI mode'
require 'pipewire::repair_beacn_output_profile_only' "$MAIN" 'repair CLI does not invoke profile recovery'
require 'repair_output_profile_at_startup' "$MAIN" 'startup profile repair hook missing'
require '--repair-output-profile' "$INSTALL" 'installer does not run safe profile repair'

repair_line=$(grep -n -- '--repair-output-profile' "$INSTALL" | head -1 | cut -d: -f1)
probe_line=$(grep -n -- '--probe' "$INSTALL" | tail -1 | cut -d: -f1)
if [[ -z "$repair_line" || -z "$probe_line" || "$repair_line" -ge "$probe_line" ]]; then
  echo 'CONTRACT_FAIL: repair must run before post-install probe' >&2
  exit 1
fi

if grep -Fq -- 'wpctl set-default' "$MAIN" "$ROOT/src/pipewire.rs" "$INSTALL"; then
  echo 'CONTRACT_FAIL: repair must not change system defaults' >&2
  exit 1
fi

echo 'AETHERFORGE_BEACN_V0_1_19_PROFILE_REPAIR_INVOCATION_CONTRACT=PASS'
