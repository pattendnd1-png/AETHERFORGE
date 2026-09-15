#!/usr/bin/env bash
set -uo pipefail
VERSION=10.0.28
OUT="${HOME}/Downloads/ForgeHX-${VERSION}-VERIFY.txt"
mkdir -p "${HOME}/Downloads"
: > "$OUT"
exec > >(tee -a "$OUT") 2>&1
printf 'FORGEHX_VERSION=%s\n' "$VERSION"
printf 'FORGEHX_BASELINE_VERSION=10.0.27\n'
printf 'FORGEHX_UPDATE=EXTRANEOUS_NOISE_MONITOR\n'
printf 'FORGEHX_VERIFY_GUARANTEE=ACTIVE\n'
printf 'FORGEHX_PRESERVED_AUDIO_ENDPOINT=%s\n' 'HyperX SoloCast 2 Analog Stereo'
printf 'FORGEHX_CLIPPY_POLICY=DIAGNOSTIC_ONLY\n'
fail=0
run(){ local name="$1"; shift; if "$@"; then printf 'FORGEHX_%s=PASS\n' "$name"; else rc=$?; printf 'FORGEHX_%s=FAIL:%s\n' "$name" "$rc"; fail=1; fi; }

run SOURCE python3 scripts/check-v10.0.28-source.py
run EXTRANEOUS_NOISE_MONITOR python3 scripts/test-10.0.28-extraneous-noise-monitor.py
run PACKAGING_VERSION python3 scripts/test-10.0.28-packaging-version.py
run PWCLI_REGISTRY_FIXTURE python3 scripts/test-10.0.20-pw-cli-registry-fixture.py
run NATIVE_BUILD_REGRESSIONS python3 scripts/test-10.0.19-native-build-regressions.py
run OUTPUT_DSP_SCHEMA python3 scripts/test-10.0.18-output-dsp-schema.py
run AETHERSTREAM_OUTPUT_CLIENT python3 scripts/test-10.0.18-aetherstream-output-client.py
run OUTPUT_DSP_DAEMON python3 scripts/test-10.0.18-output-daemon.py
run OUTPUT_DSP_GUI python3 scripts/test-10.0.18-output-gui.py
run SOLOCAST_ROUTING_LOCK python3 scripts/test-10.0.28-solocast-routing-lock.py
run MIC_AETHERSTREAM_BRIDGE python3 scripts/test-10.0.28-aetherstream-bridge.py
run PRIVATE_INTERFACE python3 scripts/test-10.0.16-private-interface.py
run DIRECT_TARGET_ID python3 scripts/test-10.0.14-direct-target-id.py

if command -v cargo >/dev/null 2>&1; then
  run FORMAT_APPLY cargo fmt --all
  run FORMAT cargo fmt --all -- --check
  if RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo clippy --workspace --all-targets -- -D warnings; then
    printf 'FORGEHX_CLIPPY=PASS\n'
  else
    rc=$?
    printf 'FORGEHX_CLIPPY=INFO:FAIL:%s\n' "$rc"
  fi
  run TEST bash -lc 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo test --workspace'
  run BUILD bash -lc 'RUSTFLAGS="${RUSTFLAGS:-} -D warnings" cargo build --workspace --release'
else
  printf 'FORGEHX_CARGO=FAIL:127\n'; fail=1
fi

if command -v systemctl >/dev/null 2>&1 && systemctl --user is-active --quiet forgehx-daemon.service; then
  printf 'FORGEHX_DAEMON=PASS:active\n'
else
  printf 'FORGEHX_DAEMON=FAIL:inactive\n'; fail=1
fi

runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/aetherstream/output-dsp.sock"
if [[ -S "$runtime" ]]; then
  printf 'FORGEHX_AETHERSTREAM_OUTPUT_DSP_SOCKET=PASS\n'
else
  printf 'FORGEHX_AETHERSTREAM_OUTPUT_DSP_SOCKET=FAIL:%s\n' "$runtime"; fail=1
fi

if command -v wpctl >/dev/null 2>&1 && wpctl status 2>/dev/null | grep -Fq 'HyperX SoloCast 2 Analog Stereo'; then
  printf 'FORGEHX_SOLOCAST_ENDPOINT_VISIBLE=PASS\n'
else
  printf 'FORGEHX_SOLOCAST_ENDPOINT_VISIBLE=INFO:not_observed\n'
fi
if command -v pw-cli >/dev/null 2>&1 && pw-cli ls Node 2>/dev/null | grep -Fq 'ForgeHX Direct HyperX Capture'; then
  printf 'FORGEHX_DIRECT_CAPTURE_NODE=PASS\n'
else
  printf 'FORGEHX_DIRECT_CAPTURE_NODE=INFO:not_observed_during_verify\n'
fi

state_file="$(mktemp)"
list_file="$(mktemp)"
tone_file="$(mktemp)"
cleanup(){ rm -f "$state_file" "$list_file" "$tone_file"; }
trap cleanup EXIT

device_id=''
if command -v forgehx >/dev/null 2>&1; then
  forgehx list >"$list_file" 2>/dev/null || forgehx list --all >"$list_file" 2>/dev/null || true
  device_id="$(grep -im1 'HyperX SoloCast 2' "$list_file" | awk -F '\t' '{print $1}')"
  if [[ -n "$device_id" ]] && forgehx mic dsp get "$device_id" >"$state_file" 2>/dev/null; then
    if python3 scripts/test-10.0.28-runtime-state.py "$state_file"; then
      printf 'FORGEHX_LIVE_DSP_STATE=PASS\n'
    else
      printf 'FORGEHX_LIVE_DSP_STATE=FAIL:not_applied_or_wrong_state\n'; fail=1
    fi
  else
    printf 'FORGEHX_LIVE_DSP_STATE=FAIL:device_or_state_unavailable\n'; fail=1
  fi
else
  printf 'FORGEHX_LIVE_DSP_STATE=FAIL:cli_missing\n'; fail=1
fi

# Active playback proves the existing full-system render reference feeds the monitor.
# This does not change the default source or sink and does not record raw samples.
playback_testable=0
if command -v pw-cat >/dev/null 2>&1 && command -v wpctl >/dev/null 2>&1 && wpctl status 2>/dev/null | grep -Eq 'Audio|Sinks'; then
  playback_testable=1
fi
if (( playback_testable == 1 )) && [[ -n "$device_id" ]]; then
  python3 - "$tone_file" <<'PYTONE'
import math,struct,sys
path=sys.argv[1]
rate=48000
with open(path,'wb') as f:
    for n in range(rate*2):
        x=0.035*math.sin(2*math.pi*523.25*n/rate)+0.018*math.sin(2*math.pi*1568.0*n/rate)
        f.write(struct.pack('<f',x))
PYTONE
  if pw-cat --playback --target '@DEFAULT_AUDIO_SINK@' --rate 48000 --channels 1 --format f32 "$tone_file" >/dev/null 2>&1 & then
    playback_pid=$!
    speaker_ok=0
    for _ in 1 2 3 4 5 6; do
      sleep 0.5
      if forgehx mic dsp get "$device_id" >"$state_file" 2>/dev/null && python3 - "$state_file" <<'PYREF'
import json,sys
try: d=json.load(open(sys.argv[1]))
except Exception: raise SystemExit(1)
s=(d.get('state') or {})
n=s.get('noise_scene_telemetry') or {}
score=(n.get('classes') or {}).get('speaker_leak')
ok=(n.get('reference_available') is True and isinstance(score,(int,float)) and 0.0 <= float(score) <= 1.0
    and s.get('processed_source_node_name') == 'aetherstream.system.microphone')
raise SystemExit(0 if ok else 1)
PYREF
      then speaker_ok=1; break; fi
    done
    wait "$playback_pid" 2>/dev/null || true
    if (( speaker_ok == 1 )); then
      printf 'FORGEHX_SPEAKER_REFERENCE=PASS\n'
    else
      printf 'FORGEHX_SPEAKER_REFERENCE=FAIL:reference_not_observed_during_playback\n'; fail=1
    fi
  else
    printf 'FORGEHX_SPEAKER_REFERENCE=FAIL:playback_command_failed\n'; fail=1
  fi
else
  printf 'FORGEHX_SPEAKER_REFERENCE=INFO:not_testable_no_playback_sink\n'
fi

if (( fail == 0 )); then
  printf 'FORGEHX_EXTRANEOUS_NOISE_MONITOR=PASS\n'
  printf 'FORGEHX_10_0_28_VERIFY=PASS\n'
  printf 'FORGEHX_VERIFY_FILE=%s\n' "$OUT"
  exit 0
else
  printf 'FORGEHX_10_0_28_VERIFY=FAIL\n'
  printf 'FORGEHX_VERIFY_FILE=%s\n' "$OUT"
  exit 1
fi
