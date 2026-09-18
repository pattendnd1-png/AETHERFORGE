#!/usr/bin/env bash
set -euo pipefail
ROOT="${1:?source root required}"
MAIN="$ROOT/src/main.rs"
PW="$ROOT/src/pipewire.rs"
PROBE="$ROOT/src/probe.rs"
TESTS="$ROOT/tests/core.rs"
PRIVATE="$ROOT/src/private_audio.rs"

grep -Fq 'pub devices: Vec<AudioNode>' "$PW"
grep -Fq 'pub enum BeacnAudioHealth' "$PW"
grep -Fq 'pub fn beacn_audio_health' "$PW"
grep -Fq 'pub fn reselect_node' "$PW"
grep -Fq 'pub fn recover_beacn_nodes' "$PW"
grep -Fq 'pub fn is_private_beacn_node' "$PW"
grep -Fq 'BEACN_PIPEWIRE_HEALTH=' "$PROBE"
grep -Fq 'AETHERFORGE_BEACN_PRIVATE_DSP=APP_ISOLATED_PIPE_SOURCE' "$PROBE"
grep -Fq 'AETHERFORGE_BEACN_SYSTEM_DSP_INTEGRATION=FORBIDDEN' "$PROBE"
grep -Fq 'ARECORD_LIST' "$PROBE"
grep -Fq 'APLAY_LIST' "$PROBE"
grep -Fq 'does_not_fall_through_to_another_source_when_selected_node_disappears' "$TESTS"
grep -Fq 'reports_usb_present_but_beacn_pipewire_nodes_missing' "$TESTS"
grep -Fq 'pipewire_health_ignores_private_processed_source_when_raw_missing' "$TESTS"
grep -Fq 'pub const PRIVATE_SOURCE_NAME' "$PRIVATE"
if grep -Fq 'current.is_some_and(|index| index < nodes.len())' "$MAIN"; then
  echo 'INDEX_REUSE_SELECTION=FAIL'
  exit 1
fi
grep -Fq 'Recover BEACN audio nodes' "$MAIN"
grep -Fq 'selected_source: Option<String>' "$MAIN"
grep -Fq 'selected_sink: Option<String>' "$MAIN"
grep -Fq 'BEACN audio nodes missing' "$MAIN"
if grep -Fq 'set-default' "$PW"; then
  echo 'DEFAULT_DEVICE_MUTATION=FAIL'
  exit 1
fi
echo 'AETHERFORGE_BEACN_V0_1_16_RUNTIME_AUDIO_CONTRACT=PASS'
