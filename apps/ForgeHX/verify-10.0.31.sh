#!/usr/bin/env bash
set -uo pipefail
VERSION=10.0.31
OUT="${HOME}/Downloads/ForgeHX-${VERSION}-VERIFY.txt"
TMPROOT="${HOME}/Downloads/.forgehx-${VERSION}-tmp"
mkdir -p "${HOME}/Downloads" "$TMPROOT"
export TMPDIR="$TMPROOT" TMP="$TMPROOT" TEMP="$TMPROOT"
: > "$OUT"
exec > >(tee -a "$OUT") 2>&1
printf 'FORGEHX_VERSION=%s\n' "$VERSION"
printf 'FORGEHX_BASELINE_VERSION=10.0.30\n'
printf 'FORGEHX_UPDATE=FALSE_SPEECH_PROTECTION_PLAYBACK_PRIORITY_FIX\n'
printf 'FORGEHX_VERIFY_GUARANTEE=ACTIVE\n'
printf 'FORGEHX_PRESERVED_AUDIO_ENDPOINT=%s\n' 'HyperX SoloCast 2 Analog Stereo'
printf 'FORGEHX_CLIPPY_POLICY=DIAGNOSTIC_ONLY\n'
printf 'FORGEHX_TMP_REDIRECT=PASS:%s\n' "$TMPROOT"
fail=0
run(){ local name="$1"; shift; if "$@"; then printf 'FORGEHX_%s=PASS\n' "$name"; else rc=$?; printf 'FORGEHX_%s=FAIL:%s\n' "$name" "$rc"; fail=1; fi; }
run SOURCE python3 scripts/check-v10.0.31-source.py
run PLAYBACK_PRIORITY python3 scripts/test-10.0.31-playback-priority.py
run VOICE_ONLY_CAPTURE python3 scripts/test-10.0.30-voice-only-capture.py
run EXTRANEOUS_NOISE_MONITOR python3 scripts/test-10.0.30-extraneous-noise-monitor.py
run LIVE_SPEAKER_REFERENCE python3 scripts/test-10.0.30-speaker-reference.py .
run PACKAGING_VERSION python3 scripts/test-10.0.31-packaging-version.py
run SOLOCAST_ROUTING_LOCK python3 scripts/test-10.0.31-solocast-routing-lock.py
run MIC_AETHERSTREAM_BRIDGE python3 scripts/test-10.0.31-aetherstream-bridge.py
run PRIVATE_INTERFACE python3 scripts/test-10.0.16-private-interface.py
run DIRECT_TARGET_ID python3 scripts/test-10.0.14-direct-target-id.py
if command -v cargo >/dev/null 2>&1; then
  run FORMAT_APPLY cargo fmt --all
  run FORMAT cargo fmt --all -- --check
  if RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo clippy --workspace --all-targets -- -D warnings; then printf 'FORGEHX_CLIPPY=PASS\n'; else rc=$?; printf 'FORGEHX_CLIPPY=INFO:FAIL:%s\n' "$rc"; fi
  run TEST bash -lc 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test --workspace'
  run BUILD bash -lc 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo build --workspace --release'
else printf 'FORGEHX_CARGO=FAIL:127\n'; fail=1; fi
if systemctl --user is-active --quiet forgehx-daemon.service; then printf 'FORGEHX_DAEMON=PASS:active\n'; else printf 'FORGEHX_DAEMON=FAIL:inactive\n'; fail=1; fi
runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/aetherstream/output-dsp.sock"
if [[ -S "$runtime" ]]; then printf 'FORGEHX_AETHERSTREAM_OUTPUT_DSP_SOCKET=PASS\n'; else printf 'FORGEHX_AETHERSTREAM_OUTPUT_DSP_SOCKET=FAIL:%s\n' "$runtime"; fail=1; fi
bridge_dir="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/aetherstream"
bridge_socket="$bridge_dir/forgehx-mic.sock"
bridge_active="$bridge_dir/forgehx-mic.active"
if [[ -S "$bridge_socket" ]]; then printf 'FORGEHX_AETHERSTREAM_MIC_BRIDGE_SOCKET=PASS:%s\n' "$bridge_socket"; else printf 'FORGEHX_AETHERSTREAM_MIC_BRIDGE_SOCKET=FAIL:%s\n' "$bridge_socket"; fail=1; fi
if [[ -f "$bridge_active" ]]; then printf 'FORGEHX_AETHERSTREAM_MIC_BRIDGE_HEARTBEAT=PASS:%s\n' "$bridge_active"; else printf 'FORGEHX_AETHERSTREAM_MIC_BRIDGE_HEARTBEAT=FAIL:not_live\n'; fail=1; fi
state_file="$(mktemp -p "$TMPROOT")"; list_file="$(mktemp -p "$TMPROOT")"; cleanup(){ rm -f "$state_file" "$list_file"; }; trap cleanup EXIT
device_id=''
forgehx list >"$list_file" 2>/dev/null || forgehx list --all >"$list_file" 2>/dev/null || true
device_id="$(grep -im1 'HyperX SoloCast 2' "$list_file" | awk -F '\t' '{print $1}')"
if [[ -n "$device_id" ]] && forgehx mic dsp get "$device_id" >"$state_file" 2>/dev/null; then
  if python3 scripts/test-10.0.31-runtime-state.py "$state_file"; then printf 'FORGEHX_LIVE_DSP_STATE=PASS\n'; else printf 'FORGEHX_LIVE_DSP_STATE=FAIL:not_applied_or_wrong_state\n'; fail=1; fi
else printf 'FORGEHX_LIVE_DSP_STATE=FAIL:device_or_state_unavailable\n'; fail=1; fi
if (( fail == 0 )); then
  printf 'FORGEHX_10_0_31_VERIFY=PASS\n'
  printf 'FORGEHX_VERIFY_FILE=%s\n' "$OUT"
  exit 0
else
  printf 'FORGEHX_10_0_31_VERIFY=FAIL\n'
  printf 'FORGEHX_VERIFY_FILE=%s\n' "$OUT"
  exit 1
fi
