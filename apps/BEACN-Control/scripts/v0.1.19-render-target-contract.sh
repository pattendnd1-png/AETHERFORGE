#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)}"
MAIN="$ROOT/src/main.rs"
DSP="$ROOT/src/software_dsp.rs"
PRIVATE="$ROOT/src/private_dsp.rs"
required_main=(
  'Self::Home'
  'Self::Headphones'
  'Self::Recorder'
  'Equalizer & Enhancement'
  'Secondary Processing'
  'Mic Output'
  'LED Control'
  'Real-Time Meters'
  'Quick Presets'
  'Monitor Mix'
  'Broadcast'
  'Streaming'
  'Podcast'
  'Voice Chat'
  'Music'
  'Custom 1'
  'FULL WINDOWS 1.4 PARITY'
  'PRIVATE DSP'
  'DRAGONGLASS UI'
)
for needle in "${required_main[@]}"; do
  grep -Fq "$needle" "$MAIN" || { echo "RENDER_TARGET_CONTRACT=FAIL:missing:$needle"; exit 1; }
done
grep -Fq 'fn apply_quick_preset' "$MAIN" || { echo 'RENDER_TARGET_CONTRACT=FAIL:missing_quick_preset_action'; exit 1; }
grep -Eq 'MIC_EQ_BAND_COUNT:[[:space:]]+usize[[:space:]]*=[[:space:]]*10' "$DSP" || { echo 'RENDER_TARGET_CONTRACT=FAIL:mic_eq_not_10_band'; exit 1; }
grep -Fq 'MIC_EQ_BAND_COUNT' "$PRIVATE" || { echo 'RENDER_TARGET_CONTRACT=FAIL:private_dsp_not_bound_to_mic_eq_count'; exit 1; }
if grep -RniE 'aetherstream|system[-_ ]dsp integration' "$ROOT/src" "$ROOT/Cargo.toml" | grep -v 'SYSTEM DSP ISOLATED' >/dev/null; then
  echo 'RENDER_TARGET_CONTRACT=FAIL:system_dsp_coupling'
  exit 1
fi
echo 'AETHERFORGE_BEACN_V0_1_19_RENDER_TARGET=PASS'
