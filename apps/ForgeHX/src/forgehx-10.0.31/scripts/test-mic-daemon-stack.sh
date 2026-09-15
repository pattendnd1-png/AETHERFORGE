#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
daemon="$root/crates/forgehx-daemon/src/lib.rs"
grep -q 'BackendKind::ForgeHxDsp' "$daemon"
grep -q 'Capability::MicDsp' "$daemon"
grep -q 'MicDspManager' "$daemon"
grep -q 'MicDspLiveUpdate' "$daemon"
grep -q 'mic_dsp_live_update' "$daemon"
grep -q 'project_device_for_protocol' "$daemon"
grep -q 'project_backends_for_protocol' "$daemon"
grep -q 'mic_dsp_apply_rejects_a_stale_pipewire_source_before_touching_the_graph' "$daemon"
echo 'ForgeHX HyperX microphone daemon invariants passed.'
