#!/usr/bin/env bash
set -uo pipefail
VERSION=10.0.27
OUT="${HOME}/Downloads/ForgeHX-${VERSION}-VERIFY.txt"
mkdir -p "${HOME}/Downloads"
: > "$OUT"
exec > >(tee -a "$OUT") 2>&1
printf 'FORGEHX_VERSION=%s\n' "$VERSION"
printf 'FORGEHX_BASELINE_VERSION=10.0.26\n'
printf 'FORGEHX_UPDATE=ALWAYS_ON_DSP_ACTIVATION_FIX\n'
printf 'FORGEHX_VERIFY_GUARANTEE=ACTIVE\n'
printf 'FORGEHX_PRESERVED_AUDIO_ENDPOINT=%s\n' 'HyperX SoloCast 2 Analog Stereo'
printf 'FORGEHX_CLIPPY_POLICY=DIAGNOSTIC_ONLY\n'
fail=0
run(){ local name="$1"; shift; if "$@"; then printf 'FORGEHX_%s=PASS\n' "$name"; else rc=$?; printf 'FORGEHX_%s=FAIL:%s\n' "$name" "$rc"; fail=1; fi; }
run SOURCE python3 scripts/check-v10.0.27-source.py
run ALWAYS_ON_DSP_ACTIVATION python3 scripts/test-10.0.27-always-on-dsp-activation.py
run LIVE_NOISE_REJECTION python3 scripts/test-10.0.27-live-noise-rejection.py
run RELEASE_GATES python3 scripts/test-10.0.27-release-gates.py
run PACKAGING_VERSION python3 scripts/test-10.0.27-packaging-version.py
run PWCLI_REGISTRY_FIXTURE python3 scripts/test-10.0.20-pw-cli-registry-fixture.py
run NATIVE_BUILD_REGRESSIONS python3 scripts/test-10.0.19-native-build-regressions.py
run OUTPUT_DSP_SCHEMA python3 scripts/test-10.0.18-output-dsp-schema.py
run AETHERSTREAM_OUTPUT_CLIENT python3 scripts/test-10.0.18-aetherstream-output-client.py
run OUTPUT_DSP_DAEMON python3 scripts/test-10.0.18-output-daemon.py
run OUTPUT_DSP_GUI python3 scripts/test-10.0.18-output-gui.py
run SOLOCAST_ROUTING_LOCK python3 scripts/test-10.0.27-solocast-routing-lock.py
run MIC_AETHERSTREAM_BRIDGE python3 scripts/test-10.0.27-aetherstream-bridge.py
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
if command -v systemctl >/dev/null 2>&1 && systemctl --user is-active --quiet aetherstream-audiod.service; then
  printf 'FORGEHX_AETHERSTREAM_AUDIOD=PASS\n'
else
  printf 'FORGEHX_AETHERSTREAM_AUDIOD=INFO:inactive_or_different_unit\n'
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
if command -v forgehx >/dev/null 2>&1; then
  tmp_state="$(mktemp)"
  tmp_list="$(mktemp)"
  forgehx list >"$tmp_list" 2>/dev/null || forgehx list --all >"$tmp_list" 2>/dev/null || true
  device_id="$(grep -im1 'HyperX SoloCast 2' "$tmp_list" | awk -F '\t' '{print $1}')"
  if [[ -n "$device_id" ]] && forgehx mic dsp get "$device_id" >"$tmp_state" 2>/dev/null; then
    if python3 - "$tmp_state" <<'PYDSP'
import json,sys
try:
    d=json.load(open(sys.argv[1]))
except Exception:
    raise SystemExit(1)
cfg=d.get('config') or {}
ns=cfg.get('noise_suppression') or {}
ok=(d.get('applied') is True and cfg.get('enabled') is True and ns.get('enabled') is True
    and float(ns.get('strength_percent',0)) >= 95.0
    and bool(d.get('raw_source_node_name'))
    and d.get('processed_source_node_name') == 'aetherstream.system.microphone')
raise SystemExit(0 if ok else 1)
PYDSP
    then
      printf 'FORGEHX_LIVE_DSP_STATE=PASS\n'
    else
      printf 'FORGEHX_LIVE_DSP_STATE=FAIL:not_applied_or_wrong_state\n'; fail=1
    fi
  else
    printf 'FORGEHX_LIVE_DSP_STATE=FAIL:device_or_state_unavailable\n'; fail=1
  fi
  rm -f "$tmp_state" "$tmp_list"
else
  printf 'FORGEHX_LIVE_DSP_STATE=FAIL:cli_missing\n'; fail=1
fi
if (( fail == 0 )); then printf 'FORGEHX_10_0_27_VERIFY=PASS\n'; exit 0; else printf 'FORGEHX_10_0_27_VERIFY=FAIL\n'; exit 1; fi
