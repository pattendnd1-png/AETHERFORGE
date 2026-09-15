#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -q 'pub const IPC_PROTOCOL_VERSION: u32 = 11;' crates/forgehx-core/src/lib.rs
grep -q 'pub struct MicrophoneDspConfig' crates/forgehx-core/src/lib.rs
grep -q 'pub struct VoicePilotConfig' crates/forgehx-core/src/lib.rs
grep -q 'pub struct ToneControlsConfig' crates/forgehx-core/src/lib.rs
grep -q 'pub struct MultibandEqBand' crates/forgehx-core/src/lib.rs
grep -q 'forgehx_processed_mic' crates/forgehx-dsp/src/lib.rs
grep -q 'VoiceProcessingEngine' crates/forgehx-dsp/src/engine.rs
grep -q 'DspRuntimeRegistry' crates/forgehx-dsp/src/runtime.rs
grep -q 'process_render_f32' crates/forgehx-dsp/src/engine.rs
grep -q 'process_capture_f32' crates/forgehx-dsp/src/engine.rs
grep -q 'Audio/Source' crates/forgehx-dsp/src/direct_pipewire.rs
grep -q 'ForgeHX Mic' crates/forgehx-dsp/src/direct_pipewire.rs
grep -q 'wait_for_source_node_id(raw_source)' crates/forgehx-dsp/src/direct_pipewire.rs
grep -q 'Some(raw_source_node_id)' crates/forgehx-dsp/src/direct_pipewire.rs
if grep -RniE 'Audio/Sink|ForgeHX DSP Internal Injection|node\.hidden|pw-loopback' crates/forgehx-dsp/src --include='*.rs'; then
  echo 'obsolete virtual microphone chain found' >&2
  exit 1
fi
# ForgeHX captures the physical HyperX source directly and publishes exactly one app-facing microphone source.
if grep -RniE 'librnnoise_ladspa|noise_suppressor_mono|lsp-plug\.in/plugins/lv2' crates/forgehx-dsp/src --include='*.rs'; then
  echo 'legacy external microphone DSP path found' >&2
  exit 1
fi
if grep -RniE 'JamesDSP|jamesdsp|EasyEffects|easyeffects' crates/forgehx-dsp crates/forgehx-gui/src/microphone.rs 2>/dev/null; then
  echo 'output-oriented DSP runtime marker found' >&2
  exit 1
fi
if grep -qE "'noise-suppression-for-voice'|'lsp-plugins-lv2'" PKGBUILD; then
  echo 'legacy external microphone DSP package dependency found' >&2
  exit 1
fi
echo 'ForgeHX HyperX microphone realtime input-only invariants passed.'
