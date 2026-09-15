#!/usr/bin/env bash
set -Eeuo pipefail

GENERATION='20260913-WINDOWS-CLEAN-REBUILD-3-CUTOVER'
SOURCE_ARCHIVE='OpenDeck-v1.3.0-WINDOWS-CLEAN-REBUILD-3-SOURCE.tar.xz'
SOURCE_SHA256='496e7b24e915e9d0bbc78f848947a9ac55cc43b53ef54f4e5100f4cd52c2a389'
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
DOWNLOADS="${HOME}/Downloads"
WORK="${DOWNLOADS}/.opendeck-v130-windows-clean-rebuild-3"
LOGDIR="${DOWNLOADS}/OpenDeck-v1.3.0-WINDOWS-CLEAN-REBUILD-3-logs"
VERIFY="${DOWNLOADS}/OpenDeck-v1.3.0-WINDOWS-CLEAN-REBUILD-3-HOST-VERIFY.txt"
REFERENCE_DIR="${DOWNLOADS}/OpenDeck-Windows-7.5.1-REFERENCE"
CURRENT_STAGE='BOOTSTRAP'
CURRENT_LOG=''

mark(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
fail(){
  local rc="${1:-1}"
  mark "OPENDECK_V130_CLEAN3_FAILURE_STAGE=${CURRENT_STAGE}"
  mark "OPENDECK_V130_CLEAN3_FAILURE_RC=${rc}"
  if [[ -n "$CURRENT_LOG" && -f "$CURRENT_LOG" ]]; then
    mark "OPENDECK_V130_CLEAN3_FAILURE_LOG=${CURRENT_LOG}"
    mark 'OPENDECK_V130_CLEAN3_FAILURE_TAIL_BEGIN'
    tail -n 160 "$CURRENT_LOG" | tee -a "$VERIFY" || true
    mark 'OPENDECK_V130_CLEAN3_FAILURE_TAIL_END'
  fi
  mark "OPENDECK_V1_3_0_WINDOWS_CLEAN_CUTOVER=FAIL:${rc}"
  mark 'OPENDECK_V130_TERMINAL_SESSION=PRESERVED'
  exit "$rc"
}
trap 'rc=0; trap - ERR; fail ""' ERR

run_logged(){
  local stage="$1"; shift
  CURRENT_STAGE="$stage"
  CURRENT_LOG="${LOGDIR}/${stage}.log"
  mark "OPENDECK_V130_STAGE=${stage}:START"
  set +e
  "$@" > >(tee "$CURRENT_LOG") 2>&1
  local rc=$?
  set -e
  if (( rc != 0 )); then fail "$rc"; fi
  mark "OPENDECK_V130_STAGE=${stage}:PASS"
  CURRENT_LOG=''
}

self_test(){
  [[ -f "${SCRIPT_DIR}/${SOURCE_ARCHIVE}" ]] || { echo 'SELF_TEST_FAIL:missing-source'; exit 2; }
  [[ "$(sha256sum "${SCRIPT_DIR}/${SOURCE_ARCHIVE}" | awk '{print $1}')" == "$SOURCE_SHA256" ]] || { echo 'SELF_TEST_FAIL:source-sha'; exit 3; }
  bash -n "${BASH_SOURCE[0]}"
  echo 'OPENDECK_V130_CLEAN3_RUNNER_SELF_TEST=PASS'
}

if [[ "${1:-}" == '--self-test' ]]; then self_test; exit 0; fi

mkdir -p "$DOWNLOADS" "$LOGDIR"
: > "$VERIFY"
mark "OPENDECK_V130_CHECKPOINT_GENERATION=${GENERATION}"
mark 'OPENDECK_V130_RELEASE_STATUS=DEVELOPMENT_CLEAN_REBUILD_NOT_CANONICAL_RELEASE'
mark 'OPENDECK_V130_CUTOVER_POLICY=BUILD_AND_QUALIFY_NEW_BEFORE_REMOVING_OLD'
mark 'OPENDECK_V130_PROFILE_CONFIG_POLICY=PRESERVE_AND_BACKUP'
mark 'OPENDECK_V130_AUTO_LAUNCH=DISABLED'
mark 'OPENDECK_V130_SERVICE_AUTOSTART=DISABLED'
mark 'OPENDECK_V130_WINDOWS_REFERENCE_EXE_MATERIALIZATION=NONBLOCKING_AFTER_EXACT_MSI_VERIFICATION'

CURRENT_STAGE='SOURCE_ARCHIVE'
SOURCE_PATH="${SCRIPT_DIR}/${SOURCE_ARCHIVE}"
[[ -f "$SOURCE_PATH" ]] || { mark "ERROR=missing-source:${SOURCE_PATH}"; fail 2; }
ACTUAL_SHA="$(sha256sum "$SOURCE_PATH" | awk '{print $1}')"
[[ "$ACTUAL_SHA" == "$SOURCE_SHA256" ]] || { mark "ERROR=source-sha:${ACTUAL_SHA}"; fail 3; }
mark "OPENDECK_V130_SOURCE_SHA256=PASS:${ACTUAL_SHA}"

CURRENT_STAGE='STORAGE_PREFLIGHT'
FREE_KIB="$(df -Pk "$DOWNLOADS" | awk 'NR==2 {print $4}')"
mark "OPENDECK_V130_STORAGE_FREE_KIB=${FREE_KIB}"
(( FREE_KIB >= 8388608 )) || { mark 'ERROR=need-at-least-8GiB-free'; fail 4; }
mark 'OPENDECK_V130_STORAGE_PREFLIGHT=PASS'

CURRENT_STAGE='EXTRACT'
rm -rf "$WORK"
mkdir -p "$WORK"
tar -xJf "$SOURCE_PATH" -C "$WORK"
ROOT="${WORK}/OpenDeck-v1.3.0-WINDOWS-CLEAN-REBUILD-3-SOURCE"
[[ -f "$ROOT/Cargo.toml" && -f "$ROOT/apps/opendeck-studio/package.json" ]] || { mark 'ERROR=source-contract-after-extract'; fail 5; }
mark "OPENDECK_V130_EXTRACT=PASS:${ROOT}"
cd "$ROOT"

run_logged 'REFERENCE_EXTRACTOR_SELF_TEST' bash scripts/reference/capture-streamdeck-windows-751.sh --self-test
run_logged 'WINDOWS_751_REFERENCE' bash scripts/reference/capture-streamdeck-windows-751.sh "$REFERENCE_DIR"
mark "OPENDECK_V130_WINDOWS_REFERENCE_REPORT=${REFERENCE_DIR}/OpenDeck-Windows-7.5.1-REFERENCE.txt"

run_logged 'SOURCE_GATES' python3 scripts/verify/check-v130-windows-clean-rebuild.py
run_logged 'HID_OWNERSHIP' python3 scripts/verify/check-hid-ownership.py
run_logged 'STREAMDECK_751_REGISTRY' python3 scripts/verify/check-streamdeck-751-parity.py
run_logged 'CUTOVER_GATE' python3 scripts/verify/check-v130-cutover.py

run_logged 'FRONTEND_DEPENDENCIES' npm --prefix apps/opendeck-studio ci --no-audit --no-fund
run_logged 'FRONTEND_TESTS' npm --prefix apps/opendeck-studio test -- --run
run_logged 'FRONTEND_LINT' npm --prefix apps/opendeck-studio run lint
run_logged 'FRONTEND_BUILD' npm --prefix apps/opendeck-studio run build
run_logged 'CARGO_FMT' cargo fmt --all -- --check
run_logged 'CARGO_TEST' cargo test --workspace --all-targets --all-features
run_logged 'CARGO_CLIPPY' cargo clippy --workspace --all-targets --all-features --keep-going -- -D warnings
run_logged 'CARGO_RELEASE' cargo build --workspace --all-features --release
run_logged 'TAURI_BUILD' npm --prefix apps/opendeck-studio run tauri -- build --no-bundle

run_logged 'POST_BUILD_SOURCE_GATES' python3 scripts/verify/check-v130-windows-clean-rebuild.py
run_logged 'POST_BUILD_HID_OWNERSHIP' python3 scripts/verify/check-hid-ownership.py
run_logged 'POST_BUILD_CUTOVER_GATE' python3 scripts/verify/check-v130-cutover.py

CURRENT_STAGE='HOST_CLEAN_ARCHIVE'
HOST_CLEAN="${DOWNLOADS}/OpenDeck-v1.3.0-WINDOWS-CLEAN-REBUILD-3-HOST-CLEAN-SOURCE.tar.xz"
HOST_CLEAN_SHA="${HOST_CLEAN}.sha256"
tar --exclude='node_modules' --exclude='target' --exclude='dist' --exclude='.git' -cJf "$HOST_CLEAN" -C "$WORK" "$(basename "$ROOT")"
(cd "$DOWNLOADS" && sha256sum "$(basename "$HOST_CLEAN")" > "$(basename "$HOST_CLEAN_SHA")")
mark "OPENDECK_V130_HOST_CLEAN_SOURCE=${HOST_CLEAN}"
mark "OPENDECK_V130_HOST_CLEAN_SHA256=$(sha256sum "$HOST_CLEAN" | awk '{print $1}')"

# Destructive cutover happens only after every new-build qualification gate above is green.
run_logged 'CUTOVER_UNINSTALL_OLD_INSTALL_NEW' bash scripts/cutover-local.sh

mark 'OPENDECK_V130_OLD_INSTALL=REMOVED'
mark 'OPENDECK_V130_NEW_CLEAN_REBUILD=INSTALLED'
mark 'OPENDECK_V130_NEW_SERVICES=DISABLED_STOPPED'
mark 'OPENDECK_V130_NEW_STUDIO=AUTO_LAUNCH_DISABLED'
mark 'OPENDECK_V130_PARITY_EVIDENCE_STATUS=STILL_REQUIRES_FINAL_12_OF_12_AND_PHYSICAL_PLUS_ACCEPTANCE'
mark 'OPENDECK_V1_3_0_WINDOWS_CLEAN_CUTOVER=PASS'
mark 'OPENDECK_V130_TERMINAL_SESSION=PRESERVED'
