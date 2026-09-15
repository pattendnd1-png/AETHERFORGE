#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -q '^version = "10.0.13"$' Cargo.toml || { echo 'workspace version 10.0.13 missing' >&2; exit 1; }
grep -q '^pkgver=10.0.13$' PKGBUILD || { echo 'PKGBUILD pkgver 10.0.13 missing' >&2; exit 1; }
grep -q '^pkgrel=1$' PKGBUILD || { echo 'PKGBUILD pkgrel 1 missing' >&2; exit 1; }
grep -q 'pub const IPC_PROTOCOL_VERSION: u32 = 11;' crates/forgehx-core/src/lib.rs || { echo 'IPC v11 missing' >&2; exit 1; }
grep -q '^sonora = "0.2"$' Cargo.toml || { echo 'Sonora 0.2 workspace dependency missing' >&2; exit 1; }
grep -q '^sonora.workspace = true$' crates/forgehx-dsp/Cargo.toml || { echo 'forgehx-dsp Sonora dependency missing' >&2; exit 1; }
for file in crates/forgehx-dsp/src/engine.rs crates/forgehx-dsp/src/voicepilot.rs crates/forgehx-dsp/src/runtime.rs; do
  [[ -f "$file" ]] || { echo "missing realtime DSP file: $file" >&2; exit 1; }
done
grep -q 'MicDspLiveUpdate' crates/forgehx-core/src/lib.rs
grep -q 'MicDspLiveUpdate' crates/forgehx-daemon/src/lib.rs
grep -q 'MicDspLiveUpdate' crates/forgehx-gui/src/app.rs
grep -q 'pub struct ClickSuppressionConfig' crates/forgehx-core/src/lib.rs
grep -q 'pub mod transient;' crates/forgehx-dsp/src/lib.rs
grep -q 'click_suppressor: TransientSuppressor' crates/forgehx-dsp/src/engine.rs
python scripts/test-v116-click-suppression.py
python scripts/test-10.0.12-direct-mic-path.py
bash scripts/test-10.0.12-warning-clean.sh
if grep -qE "'noise-suppression-for-voice'|'lsp-plugins-lv2'" PKGBUILD; then
  echo 'legacy external microphone DSP dependency remains in PKGBUILD' >&2
  exit 1
fi
if grep -RniE 'librnnoise_ladspa|noise_suppressor_mono|lsp-plug\.in/plugins/lv2' crates/forgehx-dsp/src --include='*.rs' >/dev/null; then
  echo 'legacy external microphone DSP implementation remains' >&2
  exit 1
fi
if grep -RniE 'JamesDSP|jamesdsp|EasyEffects|easyeffects' crates/forgehx-dsp crates/forgehx-gui/src/microphone.rs >/dev/null; then
  echo 'external playback DSP marker found' >&2
  exit 1
fi

echo 'ForgeHX 10.0.13 release identity invariants passed.'
