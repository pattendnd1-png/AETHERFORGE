#!/usr/bin/env bash
set -u
VERSION="10.0.17"
OUT_DIR="${FORGEHX_DOWNLOADS_DIR:-$HOME/Downloads}"
OUT="$OUT_DIR/ForgeHX-${VERSION}-VERIFY.txt"
LOG_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/forgehx/logs/verify-${VERSION}"
mkdir -p "$OUT_DIR" "$LOG_DIR"
rm -f "$LOG_DIR"/*.log 2>/dev/null || true
status_source=FAIL
status_bridge=FAIL
status_format=NOT_RUN
status_clippy=NOT_RUN
status_tests=NOT_RUN
status_build=NOT_RUN
status_runtime=NOT_RUN
failed=()
run_step(){ local n="$1"; shift; if "$@" >"$LOG_DIR/$n.log" 2>&1; then return 0; fi; failed+=("$n"); return 1; }
if run_step source python3 scripts/check-v10.0.17-source.py && run_step bridge python3 scripts/test-10.0.17-aetherstream-bridge.py; then
  status_source=PASS; status_bridge=PASS
fi
if command -v cargo >/dev/null 2>&1; then
  status_format=FAIL; status_clippy=FAIL; status_tests=FAIL; status_build=FAIL
  run_step fmt cargo fmt --all --check && status_format=PASS
  run_step clippy cargo clippy --workspace --all-targets --all-features -- -D warnings && status_clippy=PASS
  run_step tests cargo test --workspace --all-features && status_tests=PASS
  run_step build cargo build --workspace --release && status_build=PASS
else
  failed+=(cargo_missing); printf 'cargo/rustc missing\n' >"$LOG_DIR/cargo_missing.log"
fi
if command -v wpctl >/dev/null 2>&1 && command -v pw-cli >/dev/null 2>&1 && command -v systemctl >/dev/null 2>&1; then
  status_runtime=FAIL
  run_step runtime bash scripts/smoke-10.0.17-aetherstream-bridge.sh && status_runtime=PASS
else
  failed+=(runtime_tools_missing); printf 'wpctl/pw-cli/systemctl missing\n' >"$LOG_DIR/runtime_tools_missing.log"
fi
overall=PASS
for s in "$status_source" "$status_bridge" "$status_format" "$status_clippy" "$status_tests" "$status_build" "$status_runtime"; do [[ "$s" == PASS ]] || overall=FAIL; done
tmp="$(mktemp "${OUT}.tmp.XXXXXX")"
{
  echo "FORGEHX_VERSION=$VERSION"
  echo "FORGEHX_SOURCE_CONTRACT=$status_source"
  echo "FORGEHX_AETHERSTREAM_BRIDGE=$status_bridge"
  echo "FORGEHX_FORMAT=$status_format"
  echo "FORGEHX_CLIPPY=$status_clippy"
  echo "FORGEHX_TESTS=$status_tests"
  echo "FORGEHX_BUILD=$status_build"
  echo "FORGEHX_AETHERSTREAM_RUNTIME=$status_runtime"
  echo "FORGEHX_SYSTEM_MIC_OWNER=AETHERSTREAM"
  echo "FORGEHX_APP_FACING_SOURCE_COUNT=0"
  echo "FORGEHX_VERIFY=$overall"
  echo "FORGEHX_FAILURE_LOG_DIR=$LOG_DIR"
  if ((${#failed[@]})); then
    echo 'FORGEHX_FAILURE_DETAIL_BEGIN'
    for step in "${failed[@]}"; do echo "--- $step ---"; tail -n 100 "$LOG_DIR/$step.log" 2>/dev/null || true; done
    echo 'FORGEHX_FAILURE_DETAIL_END'
  fi
} >"$tmp"
mv -f "$tmp" "$OUT"
cat "$OUT"
[[ "$overall" == PASS ]]
