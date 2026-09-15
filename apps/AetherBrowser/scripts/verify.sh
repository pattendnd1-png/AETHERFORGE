#!/usr/bin/env bash
set -uo pipefail

VERSION='2.1.60'
TAG='V2_1_60'
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
OUT_DIR=${AETHER_BROWSER_OUT_DIR:-$HOME/Downloads}
VERIFY_PHASE=${AETHER_BROWSER_VERIFY_PHASE:-full}
VERIFY_FILE="$OUT_DIR/Aether-Browser-v${VERSION}-VERIFY.txt"
mkdir -p "$OUT_DIR"
: > "$VERIFY_FILE"
cd "$ROOT" || exit 2

fail=0
record(){ printf '%s\n' "$1" | tee -a "$VERIFY_FILE"; }
gate(){
  local name="$1"; shift
  record "${name}=START"
  "$@" 2>&1 | tee -a "$VERIFY_FILE"
  local rc=${PIPESTATUS[0]}
  if (( rc == 0 )); then
    record "${name}=PASS"
  else
    record "${name}=FAIL:${rc}"
    fail=1
  fi
}

record "AETHER_BROWSER_VERSION=${VERSION}"
record 'AETHER_BROWSER_ENGINE=chromium-x11'
record 'AETHER_BROWSER_ARCHITECTURE=rust-native'
record 'AETHER_BROWSER_PATCH_FOCUS=ASSISTANT_REMOVAL+BUILD_SIMPLIFICATION+OPERA_GX_DEB_REFIT+SAFE_BROWSER_TAKEOVER+CAPTURE_SAFETY'
record 'AETHER_BROWSER_RUNTIME_TEST_EXCLUSIONS=TWITCH+YOUTUBE+YOUTUBE_MUSIC'
record "AETHER_BROWSER_VERIFY_PHASE=${VERIFY_PHASE}"
if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
  record "AETHER_BROWSER_RUST_TOOLCHAIN=PASS:$(cargo --version | tr ' ' '_')"
  record "AETHER_BROWSER_RUSTC_VERSION=$(rustc --version | awk '{print $2}')"
else
  record 'AETHER_BROWSER_RUST_TOOLCHAIN=FAIL:missing-cargo-or-rustc'
  record "AETHER_BROWSER_${TAG}_VERIFY=FAIL"
  exit 20
fi

mapfile -t CURRENT_CONTRACT_TESTS < <(
  find "$ROOT/tests" -maxdepth 1 -type f -name 'current-*.sh' -printf '%f\n' | sort
)
record 'AETHER_BROWSER_VERIFY_SELF_CONTAINMENT=START'
missing_contract=0
for t in "${CURRENT_CONTRACT_TESTS[@]}"; do
  if [[ ! -f "tests/$t" ]]; then
    record "AETHER_BROWSER_VERIFY_SELF_CONTAINMENT=FAIL:missing:tests/$t"
    missing_contract=1
  fi
done
if (( missing_contract != 0 )); then
  record "AETHER_BROWSER_${TAG}_VERIFY=FAIL"
  exit 21
fi
record 'AETHER_BROWSER_VERIFY_SELF_CONTAINMENT=PASS'

gate AETHER_BROWSER_FMT cargo fmt --all -- --check
if [[ ! -f Cargo.lock ]]; then
  record 'AETHER_BROWSER_LOCKFILE=START'
  if cargo generate-lockfile 2>&1 | tee -a "$VERIFY_FILE"; then record 'AETHER_BROWSER_LOCKFILE=PASS'; else record 'AETHER_BROWSER_LOCKFILE=FAIL'; fail=1; fi
else
  record 'AETHER_BROWSER_LOCKFILE=PASS:present'
fi

gate AETHER_BROWSER_CHECK cargo check --locked --workspace --all-features
gate AETHER_BROWSER_CLIPPY cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
gate AETHER_BROWSER_TEST_FULL cargo test --locked --workspace --all-features --no-fail-fast
gate AETHER_BROWSER_HOME_NATIVE_TEST cargo test --locked -p aether-engine-servo home_has_no_servo_backing_surface
gate AETHER_BROWSER_VELORA_PROVIDER_TEST cargo test --locked -p aether-stream-providers
if grep -q '^AETHER_BROWSER_TEST_FULL=PASS$' "$VERIFY_FILE" \
  && grep -q '^AETHER_BROWSER_HOME_NATIVE_TEST=PASS$' "$VERIFY_FILE" \
  && grep -q '^AETHER_BROWSER_VELORA_PROVIDER_TEST=PASS$' "$VERIFY_FILE"; then
  record 'AETHER_BROWSER_TEST=PASS'
else
  record 'AETHER_BROWSER_TEST=FAIL'
  fail=1
fi
gate AETHER_BROWSER_BUILD bash scripts/build-first-party-release.sh
record 'AETHER_BROWSER_KNOWN_GOOD_MEDIA_RUNTIME=SKIPPED:TWITCH+YOUTUBE+YOUTUBE_MUSIC'
record 'AETHER_BROWSER_PREINSTALL_NETWORK_MEDIA=DEFERRED'
record 'AETHER_BROWSER_PREINSTALL_VISUAL_PROBES=DEFERRED'

gate AETHER_BROWSER_POST_BUILD_IDENTITY bash -c '[[ "$(target/release/aether-browser --version)" == "Aether Browser 2.1.60" ]]'
for t in "${CURRENT_CONTRACT_TESTS[@]}"; do
  key=$(basename "$t" .sh | tr '[:lower:]-' '[:upper:]_')
  gate "AETHER_BROWSER_${key}" bash "tests/$t"
done

gate AETHER_BROWSER_LIBRARY_STATUS_SMOKE bash -c 'out=$(target/release/aether-browser --library-status); grep -qF "AETHER_BROWSER_LIBRARY=READY" <<<"$out" && grep -qF "AETHER_BROWSER_LIBRARY_SELF_TEST=PASS" <<<"$out"'

if (( fail == 0 )); then
  record "AETHER_BROWSER_${TAG}_VERIFY=PASS"
  exit 0
fi
record "AETHER_BROWSER_${TAG}_VERIFY=FAIL"
exit 1
