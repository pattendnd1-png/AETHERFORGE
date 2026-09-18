#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)}"
PW="$ROOT/src/pipewire.rs"
PROBE="$ROOT/src/probe.rs"
MAIN="$ROOT/src/main.rs"
grep -Fq 'parse_pactl_cards' "$PW"
grep -Fq 'recover_beacn_output_profile' "$PW"
grep -Fq 'set-card-profile' "$PW"
grep -Fq 'PACTL_CARDS' "$PROBE"
grep -Fq 'BEACN_PULSE_ACTIVE_PROFILE' "$PROBE"
if grep -Fq 'set-default' "$PW"; then
  echo 'forbidden set-default found' >&2
  exit 1
fi
echo 'AETHERFORGE_BEACN_V0_1_15_OUTPUT_PROFILE_CONTRACT=PASS'
